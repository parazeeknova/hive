//! Native entity-derived baseline. Existing incompatible migration histories fail closed;
//! this runner never resets a populated database or adopts `SQLx` migration metadata.
use sea_orm_migration::prelude::*;
mod m20261029_000001_create_hiveway;
mod m20261029_000002_creator_sales_read_index;
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261029_000001_create_hiveway::Migration),
            Box::new(m20261029_000002_creator_sales_read_index::Migration),
        ]
    }
}
