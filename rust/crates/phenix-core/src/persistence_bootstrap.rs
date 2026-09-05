use crate::{DurableSchema, PluginId, SchemaMigration};

/// One plugin-owned durable schema prepared by Core before plugin startup.
///
/// The owner is explicit so persistence bootstrap never derives authority from
/// product-domain calls. Migrations are ordered by schema version by the
/// persistence backend when the target store is prepared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableSchemaRegistration {
    pub owner: PluginId,
    pub schema: DurableSchema,
    pub migrations: Vec<SchemaMigration>,
}

impl DurableSchemaRegistration {
    #[must_use]
    pub fn new(owner: PluginId, schema: DurableSchema) -> Self {
        Self {
            owner,
            schema,
            migrations: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_migrations(mut self, migrations: Vec<SchemaMigration>) -> Self {
        self.migrations = migrations;
        self
    }
}
