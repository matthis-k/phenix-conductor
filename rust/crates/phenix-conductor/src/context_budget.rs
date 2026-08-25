use crate::{ContextProjectionAccounting, ResolvedInvocation};
use phenix_core::{BackendCatalog, ExecutionId, ModelTarget};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

const APPROXIMATE_BYTES_PER_TOKEN: u64 = 4;
const PRUNE_PRESSURE_PERCENT: u64 = 80;
const COMPACT_PRESSURE_PERCENT: u64 = 95;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextBudgetPolicy {
    pub output_reserve_tokens: u64,
    pub safety_margin_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextBudgetError {
    ModelNotAdvertised(ModelTarget),
    MissingContextCapacity(ModelTarget),
}

impl Display for ContextBudgetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotAdvertised(target) => write!(
                f,
                "resolved model {}/{}/{} is not present in its backend catalog",
                target.backend, target.provider, target.model
            ),
            Self::MissingContextCapacity(target) => write!(
                f,
                "resolved model {}/{}/{} does not advertise context capacity",
                target.backend, target.provider, target.model
            ),
        }
    }
}

impl Error for ContextBudgetError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedModelContextCapacity {
    pub context_window_tokens: u64,
    pub tool_schema_tokens: u64,
    pub output_reserve_tokens: u64,
    pub safety_margin_tokens: u64,
}

impl ResolvedModelContextCapacity {
    pub fn from_catalog(
        catalog: &BackendCatalog,
        target: &ModelTarget,
        tool_schema_tokens: u64,
        policy: ContextBudgetPolicy,
    ) -> Result<Self, ContextBudgetError> {
        let descriptor = catalog
            .model_descriptor(target)
            .ok_or_else(|| ContextBudgetError::ModelNotAdvertised(target.clone()))?;
        let advertised = descriptor
            .context_capacity
            .ok_or_else(|| ContextBudgetError::MissingContextCapacity(target.clone()))?;
        Ok(Self {
            context_window_tokens: advertised.context_window_tokens,
            tool_schema_tokens,
            output_reserve_tokens: policy.output_reserve_tokens,
            safety_margin_tokens: policy.safety_margin_tokens,
        })
    }

    #[must_use]
    pub fn usable_input_tokens(self) -> u64 {
        self.context_window_tokens
            .saturating_sub(self.tool_schema_tokens)
            .saturating_sub(self.output_reserve_tokens)
            .saturating_sub(self.safety_margin_tokens)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextPressure {
    WithinBudget,
    PruneRequired,
    CompactRequired,
    Overflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextManagementTrigger {
    Continue,
    DeterministicPrune,
    ModelCompaction,
    OverflowRecovery,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ContextBudgetCategory {
    BasePrompt,
    InjectedContext,
    ArtifactDescriptors,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextCategoryDemand {
    pub category: ContextBudgetCategory,
    pub estimated_tokens: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextCategoryBudget {
    pub category: ContextBudgetCategory,
    pub estimated_tokens: u64,
    pub target_tokens: u64,
    pub pressure: ContextPressure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionContextBudget {
    pub execution_id: ExecutionId,
    pub model: ModelTarget,
    pub capacity: ResolvedModelContextCapacity,
    pub estimated_input_tokens: u64,
    pub usable_input_tokens: u64,
    pub remaining_input_tokens: u64,
    pub pressure: ContextPressure,
    pub category_budgets: Vec<ContextCategoryBudget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextManagementDecision {
    pub trigger: ContextManagementTrigger,
    pub pressured_categories: Vec<ContextBudgetCategory>,
}

impl ExecutionContextBudget {
    #[must_use]
    pub fn utilization_percent(&self) -> u64 {
        if self.usable_input_tokens == 0 {
            return if self.estimated_input_tokens == 0 {
                0
            } else {
                100
            };
        }
        self.estimated_input_tokens
            .saturating_mul(100)
            .saturating_div(self.usable_input_tokens)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ContextBudgetManager;

impl ContextBudgetManager {
    #[must_use]
    pub fn management_decision(budget: &ExecutionContextBudget) -> ContextManagementDecision {
        let strongest_pressure = budget
            .category_budgets
            .iter()
            .fold(budget.pressure, |strongest, category| {
                stronger_pressure(strongest, category.pressure)
            });
        let pressured_categories = budget
            .category_budgets
            .iter()
            .filter(|category| category.pressure != ContextPressure::WithinBudget)
            .map(|category| category.category)
            .collect();
        let trigger = match strongest_pressure {
            ContextPressure::WithinBudget => ContextManagementTrigger::Continue,
            ContextPressure::PruneRequired => ContextManagementTrigger::DeterministicPrune,
            ContextPressure::CompactRequired => ContextManagementTrigger::ModelCompaction,
            ContextPressure::Overflow => ContextManagementTrigger::OverflowRecovery,
        };
        ContextManagementDecision {
            trigger,
            pressured_categories,
        }
    }

    pub fn budget_resolved_invocation(
        invocation: &ResolvedInvocation,
        catalog: &BackendCatalog,
        policy: ContextBudgetPolicy,
    ) -> Result<ExecutionContextBudget, ContextBudgetError> {
        let tool_schema_tokens = estimate_tool_schema_tokens(invocation);
        let capacity = ResolvedModelContextCapacity::from_catalog(
            catalog,
            &invocation.model,
            tool_schema_tokens,
            policy,
        )?;
        let category_demands = category_demands(&invocation.context_accounting);
        Ok(Self::budget_prompt(
            &invocation.execution_id,
            &invocation.model,
            invocation.context_accounting.rendered_prompt_bytes,
            capacity,
            &category_demands,
        ))
    }

    fn budget_prompt(
        execution_id: &ExecutionId,
        model: &ModelTarget,
        rendered_prompt_bytes: u64,
        capacity: ResolvedModelContextCapacity,
        category_demands: &[ContextCategoryDemand],
    ) -> ExecutionContextBudget {
        let estimated_input_tokens = estimate_tokens(rendered_prompt_bytes);
        let usable_input_tokens = capacity.usable_input_tokens();
        let remaining_input_tokens = usable_input_tokens.saturating_sub(estimated_input_tokens);
        let pressure = classify_pressure(estimated_input_tokens, usable_input_tokens);

        ExecutionContextBudget {
            execution_id: execution_id.clone(),
            model: model.clone(),
            capacity,
            estimated_input_tokens,
            usable_input_tokens,
            remaining_input_tokens,
            pressure,
            category_budgets: allocate_category_budgets(usable_input_tokens, category_demands),
        }
    }
}

fn category_demands(accounting: &ContextProjectionAccounting) -> [ContextCategoryDemand; 3] {
    [
        ContextCategoryDemand {
            category: ContextBudgetCategory::BasePrompt,
            estimated_tokens: estimate_tokens(accounting.base_prompt_bytes),
        },
        ContextCategoryDemand {
            category: ContextBudgetCategory::InjectedContext,
            estimated_tokens: estimate_tokens(accounting.injected_context_bytes),
        },
        ContextCategoryDemand {
            category: ContextBudgetCategory::ArtifactDescriptors,
            estimated_tokens: estimate_tokens(accounting.artifact_descriptor_bytes),
        },
    ]
}

fn allocate_category_budgets(
    usable_input_tokens: u64,
    demands: &[ContextCategoryDemand],
) -> Vec<ContextCategoryBudget> {
    let mut demands = demands.to_vec();
    demands.sort_by_key(|demand| demand.category);
    let total_demand = demands.iter().fold(0_u64, |total, demand| {
        total.saturating_add(demand.estimated_tokens)
    });
    if total_demand == 0 {
        return demands
            .into_iter()
            .map(|demand| ContextCategoryBudget {
                category: demand.category,
                estimated_tokens: 0,
                target_tokens: 0,
                pressure: ContextPressure::WithinBudget,
            })
            .collect();
    }

    let mut assigned = 0_u64;
    let last = demands.len().saturating_sub(1);
    demands
        .into_iter()
        .enumerate()
        .map(|(index, demand)| {
            let target_tokens = if index == last {
                usable_input_tokens.saturating_sub(assigned)
            } else {
                usable_input_tokens
                    .saturating_mul(demand.estimated_tokens)
                    .saturating_div(total_demand)
            };
            assigned = assigned.saturating_add(target_tokens);
            ContextCategoryBudget {
                category: demand.category,
                estimated_tokens: demand.estimated_tokens,
                target_tokens,
                pressure: classify_pressure(demand.estimated_tokens, target_tokens),
            }
        })
        .collect()
}

fn estimate_tool_schema_tokens(invocation: &ResolvedInvocation) -> u64 {
    let bytes = invocation
        .tools
        .callables
        .iter()
        .fold(0_u64, |total, descriptor| {
            let schema_bytes = serde_json::to_vec(&descriptor.input_schema)
                .expect("serde_json::Value serialization cannot fail")
                .len() as u64;
            total
                .saturating_add(descriptor.id.as_str().len() as u64)
                .saturating_add(descriptor.description.len() as u64)
                .saturating_add(schema_bytes)
        });
    estimate_tokens(bytes)
}

fn estimate_tokens(bytes: u64) -> u64 {
    bytes.saturating_add(APPROXIMATE_BYTES_PER_TOKEN - 1) / APPROXIMATE_BYTES_PER_TOKEN
}

fn stronger_pressure(left: ContextPressure, right: ContextPressure) -> ContextPressure {
    if pressure_rank(right) > pressure_rank(left) {
        right
    } else {
        left
    }
}

const fn pressure_rank(pressure: ContextPressure) -> u8 {
    match pressure {
        ContextPressure::WithinBudget => 0,
        ContextPressure::PruneRequired => 1,
        ContextPressure::CompactRequired => 2,
        ContextPressure::Overflow => 3,
    }
}

fn classify_pressure(estimated_input_tokens: u64, usable_input_tokens: u64) -> ContextPressure {
    if estimated_input_tokens > usable_input_tokens {
        return ContextPressure::Overflow;
    }
    if usable_input_tokens == 0 {
        return ContextPressure::WithinBudget;
    }

    let utilization = estimated_input_tokens
        .saturating_mul(100)
        .saturating_div(usable_input_tokens);
    if utilization >= COMPACT_PRESSURE_PERCENT {
        ContextPressure::CompactRequired
    } else if utilization >= PRUNE_PRESSURE_PERCENT {
        ContextPressure::PruneRequired
    } else {
        ContextPressure::WithinBudget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_backend::ToolProvision;
    use phenix_core::{
        AuthenticationState, BackendId, CallableDescriptor, CallableId, CallableKind,
        CallablePolicy, CapabilitySet, ConfigRevisionId, ExecutionTarget, InferenceEffort,
        InferenceOptions, ModelContextCapacity, ModelDescriptor, ModelId, ProviderId, SessionId,
    };
    use serde_json::json;

    fn target(model: &str) -> ModelTarget {
        ModelTarget {
            backend: BackendId::parse("backend").unwrap(),
            provider: ProviderId::parse("provider").unwrap(),
            model: ModelId::parse(model).unwrap(),
            inference: InferenceOptions::default(),
        }
    }

    fn descriptor(target: ModelTarget, context_window_tokens: Option<u64>) -> ModelDescriptor {
        ModelDescriptor {
            target,
            name: "test model".to_owned(),
            selectable: true,
            context_capacity: context_window_tokens.map(|context_window_tokens| {
                ModelContextCapacity {
                    context_window_tokens,
                }
            }),
        }
    }

    fn catalog(models: Vec<ModelDescriptor>) -> BackendCatalog {
        BackendCatalog {
            backend: BackendId::parse("backend").unwrap(),
            models,
            authentication_state: AuthenticationState::NotRequired,
            authentication_methods: Vec::new(),
        }
    }

    fn resolved(execution: &str, model: ModelTarget, prompt_bytes: usize) -> ResolvedInvocation {
        ResolvedInvocation {
            execution_id: ExecutionId::parse(execution).unwrap(),
            session_id: SessionId::parse("session-1").unwrap(),
            config_revision: ConfigRevisionId::parse("config-1").unwrap(),
            callable: None,
            requested_target: ExecutionTarget::Fixed(model.clone()),
            model,
            prompt: "x".repeat(prompt_bytes),
            context_accounting: ContextProjectionAccounting {
                catalog_estimated_cost: 0,
                base_prompt_bytes: prompt_bytes as u64,
                injected_content_bytes: 0,
                injected_context_bytes: 0,
                artifact_descriptor_bytes: 0,
                rendered_prompt_bytes: prompt_bytes as u64,
            },
            tools: ToolProvision::default(),
        }
    }

    fn policy() -> ContextBudgetPolicy {
        ContextBudgetPolicy {
            output_reserve_tokens: 500,
            safety_margin_tokens: 250,
        }
    }

    #[test]
    fn route_capacity_change_recalculates_budget_for_same_execution() {
        let large_target = target("large");
        let small_target = target("small");
        let models = catalog(vec![
            descriptor(large_target.clone(), Some(16_000)),
            descriptor(small_target.clone(), Some(8_000)),
        ]);
        let large = ContextBudgetManager::budget_resolved_invocation(
            &resolved("execution-1", large_target, 24_000),
            &models,
            policy(),
        )
        .unwrap();
        let small = ContextBudgetManager::budget_resolved_invocation(
            &resolved("execution-1", small_target, 24_000),
            &models,
            policy(),
        )
        .unwrap();

        assert_eq!(large.pressure, ContextPressure::WithinBudget);
        assert_eq!(small.pressure, ContextPressure::PruneRequired);
        assert_eq!(large.execution_id, small.execution_id);
        assert!(large.usable_input_tokens > small.usable_input_tokens);
    }

    #[test]
    fn tool_schema_output_and_safety_reserves_reduce_usable_capacity() {
        let model = target("tool-model");
        let models = catalog(vec![descriptor(model.clone(), Some(10_000))]);
        let mut invocation = resolved("execution-1", model, 400);
        invocation.tools.callables.push(CallableDescriptor {
            id: CallableId::parse("search").unwrap(),
            kind: CallableKind::Tool,
            description: "search the workspace with a structured query".repeat(8),
            input_schema: json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }),
            output_schema: json!({"type": "object"}),
            capabilities: CapabilitySet::default(),
            policy: CallablePolicy::default(),
        });

        let budget = ContextBudgetManager::budget_resolved_invocation(
            &invocation,
            &models,
            ContextBudgetPolicy {
                output_reserve_tokens: 2_000,
                safety_margin_tokens: 1_000,
            },
        )
        .unwrap();

        assert!(budget.capacity.tool_schema_tokens > 0);
        assert_eq!(
            budget.usable_input_tokens,
            10_000 - budget.capacity.tool_schema_tokens - 2_000 - 1_000
        );
    }

    #[test]
    fn catalog_lookup_uses_model_identity_not_inference_effort() {
        let base = target("reasoning-model");
        let models = catalog(vec![descriptor(base.clone(), Some(4_000))]);
        let mut resolved_target = base;
        resolved_target.inference.effort = Some(InferenceEffort::High);
        let budget = ContextBudgetManager::budget_resolved_invocation(
            &resolved("execution-1", resolved_target, 400),
            &models,
            policy(),
        )
        .unwrap();
        assert_eq!(budget.capacity.context_window_tokens, 4_000);
    }

    #[test]
    fn missing_capacity_fails_instead_of_guessing() {
        let model = target("unknown-capacity");
        let models = catalog(vec![descriptor(model.clone(), None)]);
        assert!(matches!(
            ContextBudgetManager::budget_resolved_invocation(
                &resolved("execution-1", model.clone(), 400),
                &models,
                policy(),
            ),
            Err(ContextBudgetError::MissingContextCapacity(target)) if target == model
        ));
    }

    #[test]
    fn category_targets_follow_projection_accounting_instead_of_fixed_quotas() {
        let model = target("category-model");
        let models = catalog(vec![descriptor(model.clone(), Some(4_750))]);
        let mut first_invocation = resolved("execution-1", model.clone(), 4_000);
        first_invocation.context_accounting.base_prompt_bytes = 3_000;
        first_invocation.context_accounting.injected_context_bytes = 1_000;
        let first =
            ContextBudgetManager::budget_resolved_invocation(&first_invocation, &models, policy())
                .unwrap();

        let mut second_invocation = resolved("execution-1", model, 4_000);
        second_invocation.context_accounting.base_prompt_bytes = 1_000;
        second_invocation.context_accounting.injected_context_bytes = 3_000;
        let second =
            ContextBudgetManager::budget_resolved_invocation(&second_invocation, &models, policy())
                .unwrap();

        assert_eq!(first.usable_input_tokens, 4_000);
        assert_eq!(first.category_budgets[0].target_tokens, 3_000);
        assert_eq!(first.category_budgets[1].target_tokens, 1_000);
        assert_eq!(first.category_budgets[2].target_tokens, 0);
        assert_eq!(
            first
                .category_budgets
                .iter()
                .map(|budget| budget.target_tokens)
                .sum::<u64>(),
            first.usable_input_tokens
        );
        assert_eq!(second.category_budgets[0].target_tokens, 1_000);
        assert_eq!(second.category_budgets[1].target_tokens, 3_000);
        assert_eq!(second.category_budgets[2].target_tokens, 0);
    }

    #[test]
    fn category_pressure_is_derived_from_dynamic_target() {
        let budgets = allocate_category_budgets(
            1_000,
            &[
                ContextCategoryDemand {
                    category: ContextBudgetCategory::BasePrompt,
                    estimated_tokens: 900,
                },
                ContextCategoryDemand {
                    category: ContextBudgetCategory::ArtifactDescriptors,
                    estimated_tokens: 100,
                },
            ],
        );
        assert_eq!(budgets[0].target_tokens, 900);
        assert_eq!(budgets[1].target_tokens, 100);
        assert_eq!(budgets[0].pressure, ContextPressure::CompactRequired);
        assert_eq!(budgets[1].pressure, ContextPressure::CompactRequired);
    }

    #[test]
    fn pressure_thresholds_scale_with_catalog_capacity() {
        let model = target("threshold-model");
        let models = catalog(vec![descriptor(model.clone(), Some(1_750))]);
        let prune = ContextBudgetManager::budget_resolved_invocation(
            &resolved("execution-1", model.clone(), 3_200),
            &models,
            policy(),
        )
        .unwrap();
        let compact = ContextBudgetManager::budget_resolved_invocation(
            &resolved("execution-1", model, 3_800),
            &models,
            policy(),
        )
        .unwrap();
        assert_eq!(prune.usable_input_tokens, 1_000);
        assert_eq!(prune.pressure, ContextPressure::PruneRequired);
        assert_eq!(compact.pressure, ContextPressure::CompactRequired);
    }
    #[test]
    fn management_decision_uses_strongest_category_pressure() {
        let model = target("decision-model");
        let models = catalog(vec![descriptor(model.clone(), Some(4_750))]);
        let invocation = resolved("execution-1", model, 2_000);
        let mut budget =
            ContextBudgetManager::budget_resolved_invocation(&invocation, &models, policy())
                .unwrap();
        budget.pressure = ContextPressure::WithinBudget;
        budget.category_budgets[0].pressure = ContextPressure::PruneRequired;
        budget.category_budgets[1].pressure = ContextPressure::CompactRequired;

        assert_eq!(
            ContextBudgetManager::management_decision(&budget),
            ContextManagementDecision {
                trigger: ContextManagementTrigger::ModelCompaction,
                pressured_categories: vec![
                    ContextBudgetCategory::BasePrompt,
                    ContextBudgetCategory::InjectedContext,
                ],
            }
        );
    }

    #[test]
    fn management_decision_never_compacts_at_prune_only_pressure() {
        let model = target("prune-only-model");
        let models = catalog(vec![descriptor(model.clone(), Some(4_750))]);
        let invocation = resolved("execution-1", model, 2_000);
        let mut budget =
            ContextBudgetManager::budget_resolved_invocation(&invocation, &models, policy())
                .unwrap();
        budget.pressure = ContextPressure::PruneRequired;
        for category in &mut budget.category_budgets {
            category.pressure = ContextPressure::PruneRequired;
        }

        assert_eq!(
            ContextBudgetManager::management_decision(&budget).trigger,
            ContextManagementTrigger::DeterministicPrune
        );
    }

    #[test]
    fn overflow_recovery_outranks_compaction_pressure() {
        let model = target("overflow-decision-model");
        let models = catalog(vec![descriptor(model.clone(), Some(4_750))]);
        let invocation = resolved("execution-1", model, 2_000);
        let mut budget =
            ContextBudgetManager::budget_resolved_invocation(&invocation, &models, policy())
                .unwrap();
        budget.pressure = ContextPressure::CompactRequired;
        budget.category_budgets[0].pressure = ContextPressure::Overflow;

        assert_eq!(
            ContextBudgetManager::management_decision(&budget).trigger,
            ContextManagementTrigger::OverflowRecovery
        );
    }
}
