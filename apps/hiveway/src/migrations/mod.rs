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
mod m20261030_000009_free_capabilities;
mod m20261030_000010_catalog_candidates;
mod m20261030_000011_account_profiles;
mod m20261030_000012_account_api_keys;
mod m20261031_000013_build_jobs;
mod m20261031_000014_publisher_activity;
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
            Box::new(m20261030_000009_free_capabilities::Migration),
            Box::new(m20261030_000010_catalog_candidates::Migration),
            Box::new(m20261030_000011_account_profiles::Migration),
            Box::new(m20261030_000012_account_api_keys::Migration),
            Box::new(m20261031_000013_build_jobs::Migration),
            Box::new(m20261031_000014_publisher_activity::Migration),
        ]
    }
}
