use super::*;
use hiveway::entities::{invocations, refund_operations};
use sea_orm::IntoActiveModel;
use sea_orm::sea_query::{Expr, ExprTrait};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use tokio::sync::RwLock;

pub(super) struct Fixture {
    pub status: AtomicU16,
    pub posts: AtomicU32,
    pub refund: RwLock<Value>,
    provider: Arc<super::dodo::ProviderFixture>,
}
impl Fixture {
    pub fn new(provider: Arc<super::dodo::ProviderFixture>) -> Self {
        Self {
            status: AtomicU16::new(200),
            posts: AtomicU32::new(0),
            refund: RwLock::new(Value::Null),
            provider,
        }
    }
}
async fn create(
    axum::extract::State(fixture): axum::extract::State<Arc<Fixture>>,
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    fixture.posts.fetch_add(1, Ordering::Relaxed);
    let status = fixture.status.load(Ordering::Relaxed);
    if status != 200 {
        return StatusCode::from_u16(status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            .into_response();
    }
    let refund = fixture.refund.read().await.clone();
    assert_eq!(body["payment_id"], refund["payment_id"]);
    fixture.provider.0.write().await["refunds"] = json!([refund]);
    axum::Json(refund).into_response()
}
async fn retrieve(
    axum::extract::State(fixture): axum::extract::State<Arc<Fixture>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> std::result::Result<axum::Json<Value>, StatusCode> {
    let refund = fixture.refund.read().await;
    if refund["refund_id"] != id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(axum::Json(refund.clone()))
}
pub(super) fn router(fixture: Arc<Fixture>) -> Router {
    Router::new()
        .route("/refunds", axum::routing::post(create))
        .route("/refunds/{id}", axum::routing::get(retrieve))
        .with_state(fixture)
}
pub(super) async fn exercise(
    app: Router,
    state: &AppState,
    fixture: &Fixture,
    original: Uuid,
    tokens: (&str, &str),
) -> Result {
    let original = invocations::Entity::find_by_id(original)
        .one(&state.postgres)
        .await?
        .ok_or("missing invocation")?;
    for mode in [200, 500, 400, 201] {
        exercise_case(&app, state, fixture, &original, tokens, mode).await?;
    }
    Ok(())
}
async fn exercise_case(
    app: &Router,
    state: &AppState,
    fixture: &Fixture,
    original: &invocations::Model,
    tokens: (&str, &str),
    mode: u16,
) -> Result {
    let id = seed(state, original).await?;
    let payment = format!("pay_{}", id.simple());
    let refund = json!({"refund_id":format!("ref_{}",id.simple()),"payment_id":payment,"currency":"USD","amount":if mode == 201 {2}else{1},"is_partial":false,"status":"succeeded"});
    *fixture.refund.write().await = refund;
    *fixture.provider.0.write().await = json!({"payment_id":payment,"metadata":{"hive_invocation_id":id.to_string()},"total_amount":1,"currency":"USD","refunds":[]});
    fixture
        .status
        .store(if mode == 201 { 200 } else { mode }, Ordering::Relaxed);
    let posts = fixture.posts.load(Ordering::Relaxed);
    let path = format!("/v1/invocations/{id}/refund");
    let bearer = format!("Bearer {}", tokens.0);
    let other = format!("Bearer {}", tokens.1);
    let before = spent(state, original.credential_id).await?;
    assert_eq!(
        attack(
            app.clone(),
            "POST",
            &path,
            String::new(),
            &[("authorization", &other)]
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(fixture.posts.load(Ordering::Relaxed), posts);
    let headers = [("authorization", bearer.as_str())];
    let (a, b) = tokio::join!(
        attack(app.clone(), "POST", &path, String::new(), &headers),
        attack(app.clone(), "POST", &path, String::new(), &headers)
    );
    let (a, b) = (a?, b?);
    if mode == 201 {
        assert!([a.0, b.0].contains(&StatusCode::BAD_REQUEST));
        assert!(
            [a.0, b.0]
                .iter()
                .all(|status| [StatusCode::OK, StatusCode::BAD_REQUEST].contains(status))
        );
    } else {
        assert_eq!(a.0, StatusCode::OK);
        assert_eq!(b.0, StatusCode::OK);
    }
    assert_eq!(
        fixture.posts.load(Ordering::Relaxed),
        posts + 1,
        "refund creation must not be repeated"
    );
    let operation = refund_operations::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing refund")?;
    if mode == 201 {
        assert!(["pending", "review"].contains(&operation.state.as_str()));
    } else {
        assert_eq!(
            operation.state,
            match mode {
                200 => "succeeded",
                400 => "failed",
                _ => "review",
            }
        );
    }
    let current = spent(state, original.credential_id).await?;
    assert_eq!(
        current,
        before - if mode == 200 { original.price } else { 0 }
    );
    if mode == 500 || mode == 201 {
        fixture.refund.write().await["amount"] = json!(1);
        fixture.provider.0.write().await["refunds"] = json!([fixture.refund.read().await.clone()]);
        let (a, b) = tokio::join!(
            attack(app.clone(), "POST", &path, String::new(), &headers),
            attack(app.clone(), "POST", &path, String::new(), &headers)
        );
        assert_eq!(a?.1["state"], "succeeded");
        assert_eq!(b?.1["state"], "succeeded");
    }
    let _ = attack(app.clone(), "POST", &path, String::new(), &headers).await?;
    assert_eq!(fixture.posts.load(Ordering::Relaxed), posts + 1);
    let current = spent(state, original.credential_id).await?;
    assert_eq!(
        current,
        before - if mode == 400 { 0 } else { original.price }
    );
    Ok(())
}
async fn seed(state: &AppState, original: &invocations::Model) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let mut inv = original.clone();
    inv.id = id;
    inv.idempotency_key = Uuid::new_v4();
    inv.state = "execution_failed".into();
    inv.checkout_id = Some(format!("cks_{}", id.simple()));
    inv.provider_payment_id = Some(format!("pay_{}", id.simple()));
    inv.payment_issue = Some("refund_required".into());
    inv.created_at = 0;
    inv.updated_at = 0;
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::Spent,
            Expr::col(agent_credentials::Column::Spent).add(inv.price),
        )
        .filter(agent_credentials::Column::Id.eq(inv.credential_id))
        .exec(&state.postgres)
        .await?;
    invocations::Entity::insert(inv.into_active_model())
        .exec(&state.postgres)
        .await?;
    Ok(id)
}

async fn spent(state: &AppState, credential: Uuid) -> Result<i64> {
    Ok(agent_credentials::Entity::find_by_id(credential)
        .one(&state.postgres)
        .await?
        .ok_or("missing credential")?
        .spent)
}
