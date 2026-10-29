//! Run explicitly against a disposable local PostgreSQL database:
//! `HIVE_TEST_DATABASE_URL=postgres`://... cargo test --test postgres -- --ignored
#![cfg(not(miri))]
use hiveway::{
    database,
    entities::{agent_credentials, capabilities},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DbBackend, EntityTrait, QueryFilter,
    Set, Statement, TransactionTrait,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires HIVE_TEST_DATABASE_URL pointing at local PostgreSQL"]
async fn migrations_entities_and_budget_transactions() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("HIVE_TEST_DATABASE_URL")?;
    let admin = Database::connect(url.clone()).await?;
    let schema = format!("hive_test_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await?;
    let mut options = sea_orm::ConnectOptions::new(url);
    options.set_schema_search_path(schema.clone());
    let db = Database::connect(options).await?;
    let outcome = async {
        database::migrate(&db).await?;
        // The runner must be safe to invoke again without recreating tables.
        database::migrate(&db).await?;
        let id = Uuid::new_v4();
        let owner = "0x1111111111111111111111111111111111111111";
        agent_credentials::ActiveModel {
            id: Set(id), wallet: Set(owner.into()), name: Set("test agent".into()),
            token_hash: Set(vec![7;32]), scopes: Set(vec!["repository:analyze".into()]),
            per_call_limit: Set(100), total_limit: Set(100), spent: Set(0), reserved: Set(0),
            expires_at: Set(i64::MAX), revoked_at: Set(None), created_at: Set(0),
        }.insert(&db).await?;
        let cap = capabilities::ActiveModel {
            id: Set(Uuid::new_v4()), creator_wallet: Set(owner.into()), name: Set("test capability".into()),
            description: Set("isolated integration fixture".into()), version: Set("1".into()),
            runtime: Set("repository_stack".into()), price: Set(60), active: Set(true), created_at: Set(0), definition: Set(serde_json::json!({})),
        }.insert(&db).await?;
        assert_eq!(cap.price, 60);
        let reserve = || Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE agent_credentials SET reserved=reserved+60 WHERE id=$1 AND spent+reserved+60<=total_limit", [id.into()]);
        // Concurrent reservations must not overspend a shared budget.
        let (a,b) = tokio::join!(db.execute_raw(reserve()), db.execute_raw(reserve()));
        assert_eq!(a?.rows_affected()+b?.rows_affected(),1);
        let tx=db.begin().await?;
        tx.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE agent_credentials SET reserved=0,spent=60 WHERE id=$1",[id.into()])).await?;
        tx.rollback().await?;
        let credential=agent_credentials::Entity::find().filter(agent_credentials::Column::Id.eq(id)).one(&db).await?.ok_or("missing fixture")?;
        assert_eq!((credential.reserved,credential.spent),(60,0));
        Ok::<(),Box<dyn std::error::Error>>(())
    }.await;
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome
}

#[tokio::test]
#[ignore = "requires HIVE_TEST_DATABASE_URL pointing at local PostgreSQL"]
async fn adopts_only_matching_successful_legacy_migrations()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha384};
    let url = std::env::var("HIVE_TEST_DATABASE_URL")?;
    let admin = Database::connect(url.clone()).await?;
    let original = include_str!("../migrations/202610290001_transaction_core.sql");
    for valid in [true, false] {
        let schema = format!("hive_test_{}", Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await?;
        let mut options = sea_orm::ConnectOptions::new(url.clone());
        options.set_schema_search_path(schema.clone());
        let db = Database::connect(options).await?;
        let outcome = async {
            db.execute_unprepared(original).await?;
            db.execute_unprepared("CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY, success BOOLEAN NOT NULL, checksum BYTEA NOT NULL)").await?;
            let checksum = if valid { Sha384::digest(original.as_bytes()).to_vec() } else { vec![0;48] };
            db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "INSERT INTO _sqlx_migrations VALUES($1,TRUE,$2)", [202_610_290_001_i64.into(),checksum.into()])).await?;
            let migration=database::migrate(&db).await;
            if valid {
                migration?;
                database::migrate(&db).await?;
                // The second migration must have run despite adoption of the first.
                let row=db.query_one_raw(Statement::from_string(DbBackend::Postgres,
                    "SELECT definition FROM capabilities LIMIT 1")).await?;
                assert!(row.is_none());
            } else {
                assert!(migration.is_err(), "changed legacy SQL must never be silently adopted");
            }
            Ok::<(),Box<dyn std::error::Error>>(())
        }.await;
        db.close().await?;
        admin
            .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
            .await?;
        outcome?;
    }
    admin.close().await?;
    Ok(())
}
