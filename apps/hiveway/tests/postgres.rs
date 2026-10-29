//! Run explicitly against a disposable local PostgreSQL database:
//! `HIVE_TEST_DATABASE_URL=postgres://... cargo test --test postgres -- --ignored`
#![cfg(not(miri))]
use hiveway::{
    database,
    entities::{agent_credentials, capabilities},
};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Query};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
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
        verify_migrations(&db, &schema).await?;
        // The runner must be safe to invoke again without recreating tables.
        database::migrate(&db).await?;
        let id = Uuid::new_v4();
        let owner = "0x1111111111111111111111111111111111111111";
        agent_credentials::ActiveModel {
            id: Set(id),
            wallet: Set(owner.into()),
            name: Set("test agent".into()),
            token_hash: Set(vec![7; 32]),
            scopes: Set(vec!["repository:analyze".into()]),
            per_call_limit: Set(100),
            total_limit: Set(100),
            spent: Set(0),
            reserved: Set(0),
            expires_at: Set(i64::MAX),
            revoked_at: Set(None),
            created_at: Set(0),
        }
        .insert(&db)
        .await?;
        let cap = capabilities::ActiveModel {
            id: Set(Uuid::new_v4()),
            creator_wallet: Set(owner.into()),
            name: Set("test capability".into()),
            description: Set("isolated integration fixture".into()),
            version: Set("1".into()),
            runtime: Set("repository_stack".into()),
            price: Set(60),
            active: Set(true),
            created_at: Set(0),
            definition: Set(serde_json::json!({})),
        }
        .insert(&db)
        .await?;
        assert_eq!(cap.price, 60);
        let reserve = || {
            agent_credentials::Entity::update_many()
                .col_expr(
                    agent_credentials::Column::Reserved,
                    Expr::col(agent_credentials::Column::Reserved).add(60_i64),
                )
                .filter(agent_credentials::Column::Id.eq(id))
                .filter(
                    Expr::col(agent_credentials::Column::Spent)
                        .add(Expr::col(agent_credentials::Column::Reserved))
                        .add(60_i64)
                        .lte(Expr::col(agent_credentials::Column::TotalLimit)),
                )
                .exec(&db)
        };
        // Concurrent reservations must not overspend a shared budget.
        let (a, b) = tokio::join!(reserve(), reserve());
        assert_eq!(a?.rows_affected + b?.rows_affected, 1);
        let tx = db.begin().await?;
        agent_credentials::Entity::update_many()
            .col_expr(agent_credentials::Column::Reserved, Expr::val(0_i64))
            .col_expr(agent_credentials::Column::Spent, Expr::val(60_i64))
            .filter(agent_credentials::Column::Id.eq(id))
            .exec(&tx)
            .await?;
        tx.rollback().await?;
        let credential = agent_credentials::Entity::find()
            .filter(agent_credentials::Column::Id.eq(id))
            .one(&db)
            .await?
            .ok_or("missing fixture")?;
        assert_eq!((credential.reserved, credential.spent), (60, 0));
        assert_database_constraints(&db, id, owner).await?;
        assert_settlement_output(&db, id, cap.id, owner).await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome
}

async fn assert_database_constraints(
    db: &sea_orm::DatabaseConnection,
    id: Uuid,
    owner: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        agent_credentials::Entity::update_many()
            .col_expr(agent_credentials::Column::Reserved, Expr::val(101_i64))
            .filter(agent_credentials::Column::Id.eq(id))
            .exec(db)
            .await
            .is_err(),
        "database constraint must reject overspending even outside API code"
    );
    assert!(
        capabilities::ActiveModel {
            id: Set(Uuid::new_v4()),
            creator_wallet: Set(owner.into()),
            name: Set("bad runtime".into()),
            description: Set("constraint fixture".into()),
            version: Set("1".into()),
            runtime: Set("arbitrary_shell".into()),
            price: Set(1),
            active: Set(true),
            created_at: Set(0),
            definition: Set(serde_json::json!({}))
        }
        .insert(db)
        .await
        .is_err()
    );
    assert_eq!(
        sea_orm_migration::seaql_migrations::Entity::find()
            .count(db)
            .await?,
        4
    );
    assert!(
        hiveway::entities::creator_sales::ActiveModel {
            invocation_id: Set(Uuid::new_v4()),
            creator_wallet: Set(owner.into()),
            payer_wallet: Set(owner.into()),
            amount: Set(1),
            network: Set("fixture".into()),
            transaction_hash: Set(Uuid::new_v4().to_string()),
            created_at: Set(0)
        }
        .insert(db)
        .await
        .is_err(),
        "foreign keys must reject sales without an invocation"
    );
    Ok(())
}

async fn verify_migrations(
    db: &sea_orm::DatabaseConnection,
    schema: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (first, second) = tokio::join!(database::migrate(db), database::migrate(db));
    first?;
    second?;
    let index = db
        .query_one(
            &Query::select()
                .column(Alias::new("indexdef"))
                .from(Alias::new("pg_indexes"))
                .and_where(Expr::col(Alias::new("schemaname")).eq(schema.to_owned()))
                .and_where(Expr::col(Alias::new("indexname")).eq("creator_sales_creator_activity"))
                .to_owned(),
        )
        .await?
        .ok_or("creator activity index missing")?;
    let definition: String = index.try_get_by_index(0)?;
    assert!(definition.contains("(creator_wallet, created_at DESC, invocation_id DESC)"));
    Ok(())
}

async fn assert_settlement_output(
    db: &sea_orm::DatabaseConnection,
    credential: Uuid,
    capability: Uuid,
    owner: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use hiveway::entities::invocations;
    let id = Uuid::new_v4();
    invocations::ActiveModel {
        id: Set(id),
        credential_id: Set(credential),
        capability_id: Set(capability),
        creator_wallet: Set(owner.into()),
        idempotency_key: Set(Uuid::new_v4()),
        request_hash: Set(vec![1; 32]),
        input: Set(serde_json::json!({})),
        requirements: Set(serde_json::json!({})),
        resource: Set(serde_json::json!({})),
        state: Set("settling".into()),
        price: Set(1),
        expires_at: Set(0),
        created_at: Set(0),
        updated_at: Set(0),
        ..Default::default()
    }
    .insert(db)
    .await?;
    for (result, settlement) in [
        (None, None),
        (Some(serde_json::json!({})), None),
        (None, Some(serde_json::json!({}))),
    ] {
        assert!(
            invocations::Entity::update_many()
                .col_expr(invocations::Column::State, Expr::val("settled"))
                .col_expr(invocations::Column::Result, Expr::val(result))
                .col_expr(invocations::Column::Settlement, Expr::val(settlement))
                .filter(invocations::Column::Id.eq(id))
                .exec(db)
                .await
                .is_err()
        );
    }
    let updated = invocations::Entity::update_many()
        .col_expr(invocations::Column::State, Expr::val("settled"))
        .col_expr(
            invocations::Column::Result,
            Expr::val(serde_json::json!({"ok":true})),
        )
        .col_expr(
            invocations::Column::Settlement,
            Expr::val(serde_json::json!({"success":true})),
        )
        .filter(invocations::Column::Id.eq(id))
        .exec(db)
        .await?;
    assert_eq!(updated.rows_affected, 1);
    Ok(())
}
