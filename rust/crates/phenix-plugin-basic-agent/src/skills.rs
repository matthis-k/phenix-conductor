use phenix_core::{
    skill_service, Authority, CapabilityId, ComponentExport, ComponentId, ComponentInterface,
    ComponentManifest, DurableSchema, InterfaceId, PluginContext, PluginExecution, PluginHost,
    PluginId, PluginInstance, PluginManifest, ResourceNamespace, ServiceContribution, ServiceId,
    ServiceRole, SkillCommand, SkillDefinition, SkillResponse, TransactionOp, SKILL_SERVICE,
};

pub const BASIC_SKILLS_PLUGIN: &str = "phenix.basic-skills";
pub const BASIC_SKILLS_COMPONENT: &str = "phenix.basic-skills";
const BASIC_SKILLS_NAMESPACE: &str = "phenix.basic-skills.state";
const INDEX_KEY: &str = "skills/@all";

type BasicSkillsContext<'host, 'runtime> = PluginContext<'host, 'runtime, ()>;

fn context<'host, 'runtime>(
    host: &'host PluginHost<'runtime>,
) -> BasicSkillsContext<'host, 'runtime> {
    PluginContext::new(host, (), (), ())
}

pub struct BasicSkillsInterface;

impl ComponentInterface for BasicSkillsInterface {
    type Request = SkillCommand;
    type Response = SkillResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SKILL_SERVICE).expect("static skill interface id is valid")
    }
}

#[must_use]
pub fn basic_skills_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(BASIC_SKILLS_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: ServiceRole::Terminal,
            service: skill_service(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        resource_namespaces: vec![namespace()],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn basic_skills_component_manifest() -> ComponentManifest {
    ComponentManifest {
        id: ComponentId::parse(BASIC_SKILLS_COMPONENT).expect("static component id is valid"),
        owner: basic_skills_manifest().id,
        imports: Vec::new(),
        exports: vec![ComponentExport {
            interface: BasicSkillsInterface::interface_id(),
            priority: 10,
            required_authority: Authority::default(),
        }],
        maximum_authority: persistence_authority(),
    }
}

#[must_use]
pub fn basic_skills_factory() -> Box<dyn PluginInstance> {
    Box::new(BasicSkills)
}

struct BasicSkills;

impl PluginInstance for BasicSkills {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        context(host)
            .kernel
            .register_durable_schema(&DurableSchema::new(namespace(), 1))
            .map_err(|error| error.to_string())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &skill_service() {
            return Err(format!("unsupported basic skill service: {service}"));
        }
        let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = handle(&context(host), command)?;
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn handle(
    context: &BasicSkillsContext<'_, '_>,
    command: SkillCommand,
) -> Result<SkillResponse, String> {
    match command {
        SkillCommand::Register { skill } => {
            require_id(&skill.id)?;
            write_skill(context, &skill)?;
            Ok(SkillResponse::Skill { skill: Some(skill) })
        }
        SkillCommand::Get { id } => Ok(SkillResponse::Skill {
            skill: read_skill(context, &id)?,
        }),
        SkillCommand::List => Ok(SkillResponse::Skills {
            skills: read_ids(context)?
                .into_iter()
                .map(|id| {
                    read_skill(context, &id)?.ok_or_else(|| format!("missing durable skill: {id}"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn write_skill(
    context: &BasicSkillsContext<'_, '_>,
    skill: &SkillDefinition,
) -> Result<(), String> {
    let mut ids = read_ids(context)?;
    if !ids.contains(&skill.id) {
        ids.push(skill.id.clone());
        ids.sort();
    }
    context
        .kernel
        .transact_durable(
            &namespace(),
            &[
                TransactionOp::Put {
                    key: format!("skill/{}", skill.id),
                    value: serde_json::to_vec(skill).map_err(|error| error.to_string())?,
                },
                TransactionOp::Put {
                    key: INDEX_KEY.into(),
                    value: serde_json::to_vec(&ids).map_err(|error| error.to_string())?,
                },
            ],
        )
        .map_err(|error| error.to_string())
}

fn read_skill(
    context: &BasicSkillsContext<'_, '_>,
    id: &str,
) -> Result<Option<SkillDefinition>, String> {
    context
        .kernel
        .read_durable(&namespace(), &format!("skill/{id}"))
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn read_ids(context: &BasicSkillsContext<'_, '_>) -> Result<Vec<String>, String> {
    context
        .kernel
        .read_durable(&namespace(), INDEX_KEY)
        .map_err(|error| error.to_string())?
        .map(|value| serde_json::from_slice(&value).map_err(|error| error.to_string()))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn require_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        Err("skill id must not be empty".into())
    } else {
        Ok(())
    }
}

fn namespace() -> ResourceNamespace {
    ResourceNamespace::parse(BASIC_SKILLS_NAMESPACE).expect("static namespace is valid")
}

fn persistence_authority() -> Authority {
    Authority::new([
        capability("kernel.persistence.schema"),
        capability("kernel.persistence.read"),
        capability("kernel.persistence.write"),
    ])
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}
