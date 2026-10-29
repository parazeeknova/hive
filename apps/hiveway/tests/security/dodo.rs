use super::*;
use axum::extract::{Path, State};
use hiveway::entities::invocations;
use tokio::sync::RwLock;

#[derive(Default)]
pub(super) struct ProviderFixture(pub RwLock<Value>);
pub(super) async fn payment(
    State(fixture): State<Arc<ProviderFixture>>,
    Path(id): Path<String>,
) -> std::result::Result<axum::Json<Value>, StatusCode> {
    let value = fixture.0.read().await;
    if value["payment_id"] != id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(axum::Json(value.clone()))
}
pub(super) async fn session(
    State(fixture): State<Arc<ProviderFixture>>,
    Path(id): Path<String>,
) -> std::result::Result<axum::Json<Value>, StatusCode> {
    let value = fixture.0.read().await;
    if value["checkout_session_id"] != id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(axum::Json(
        json!({"id":id,"payment_id":value["payment_id"]}),
    ))
}
pub(super) async fn exercise(
    app: Router,
    state: &AppState,
    fixture: &ProviderFixture,
    id: Uuid,
    tokens: (&str, &str),
) -> Result {
    let inv = invocations::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing invocation")?;
    let owner = agent_credentials::Entity::find_by_id(inv.credential_id)
        .one(&state.postgres)
        .await?
        .ok_or("missing credential")?;
    let canonical = json!({"status":"succeeded","currency":"USD","total_amount":inv.price/10_000,
        "payment_id":"pay_recovered","checkout_session_id":"cks_recovered",
        "metadata":{"hive_invocation_id":id.to_string(),"hive_wallet":owner.wallet},
        "product_cart":[{"product_id":"pdt_fixture","quantity":1}]});
    *fixture.0.write().await = canonical.clone();
    let path = format!("/v1/invocations/{id}/checkout/recover");
    let body = json!({"payment_id":"pay_recovered"}).to_string();
    let bearer = format!("Bearer {}", tokens.0);
    let other = format!("Bearer {}", tokens.1);
    let (status, _) = attack(
        app.clone(),
        "POST",
        &path,
        body.clone(),
        &[("authorization", &other)],
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for pointer in [
        "/total_amount",
        "/metadata/hive_wallet",
        "/metadata/hive_invocation_id",
    ] {
        let mut altered = canonical.clone();
        *altered
            .pointer_mut(pointer)
            .ok_or("missing fixture field")? = if pointer == "/total_amount" {
            json!(0)
        } else {
            json!("invalid")
        };
        *fixture.0.write().await = altered;
        let (status, _) = attack(
            app.clone(),
            "POST",
            &path,
            body.clone(),
            &[("authorization", &bearer)],
        )
        .await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            invocations::Entity::find_by_id(id)
                .one(&state.postgres)
                .await?
                .ok_or("missing invocation")?
                .state,
            "checkout_review"
        );
    }
    *fixture.0.write().await = canonical;
    let headers = [("authorization", bearer.as_str())];
    let (a, b) = tokio::join!(
        attack(app.clone(), "POST", &path, body.clone(), &headers),
        attack(app, "POST", &path, body, &headers)
    );
    assert_eq!(a?.1["state"], "paid");
    assert_eq!(b?.1["state"], "paid");
    let current = agent_credentials::Entity::find_by_id(inv.credential_id)
        .one(&state.postgres)
        .await?
        .ok_or("missing credential")?;
    assert_eq!(current.spent, owner.spent + inv.price);
    assert_eq!(current.reserved, owner.reserved - inv.price);
    let inv = invocations::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing invocation")?;
    assert_eq!(inv.checkout_id.as_deref(), Some("cks_recovered"));
    assert!(inv.payment_issue.is_none());
    Ok(())
}
