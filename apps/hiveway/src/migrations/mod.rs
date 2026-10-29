//! Native entity-derived baseline. Existing incompatible migration histories fail closed;
//! this runner never resets a populated database or adopts `SQLx` migration metadata.
use sea_orm_migration::prelude::*;
mod m20261029_000001_create_hiveway;
mod m20261029_000002_creator_sales_read_index;
mod m20261030_000003_payment_recovery_constraints;
mod m20261030_000004_refund_operations;
mod m20261030_000005_wasm_artifacts;
mod m20261030_000006_catalog_read_indexes;
mod m20261030_000007_creator_earnings;
mod m20261030_000008_catalog_search;
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261029_000001_create_hiveway::Migration),
            Box::new(m20261029_000002_creator_sales_read_index::Migration),
            Box::new(m20261030_000003_payment_recovery_constraints::Migration),
            Box::new(m20261030_000004_refund_operations::Migration),
            Box::new(m20261030_000005_wasm_artifacts::Migration),
            Box::new(m20261030_000006_catalog_read_indexes::Migration),
            Box::new(m20261030_000007_creator_earnings::Migration),
            Box::new(m20261030_000008_catalog_search::Migration),
        ]
    }
}
