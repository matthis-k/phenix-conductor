#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticResourceMigration {
    pub from_version: u32,
    pub to_version: u32,
    pub method: &'static str,
}

pub trait StaticResourceDefinition {
    fn schema_version() -> u32;

    fn migrations() -> Vec<StaticResourceMigration> {
        Vec::new()
    }
}
