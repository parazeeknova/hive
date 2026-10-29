use anyhow::Context;
use fred::prelude::ClientLike;
use hiveway::{config::Config, state::AppState, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let telemetry = telemetry::init()?;
    let result = serve().await;
    if let Err(error) = &result {
        tracing::error!(error = %error, "Hiveway stopped");
    }
    let flush = tokio::task::spawn_blocking(move || telemetry.shutdown()).await?;
    result?;
    flush
}

async fn serve() -> anyhow::Result<()> {
    let state = AppState::connect(&Config::from_env()?).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    tracing::info!(
        service = "hiveway",
        address = "127.0.0.1:3001",
        "Hiveway ready"
    );
    let result = axum::serve(listener, hiveway::router().with_state(state.clone()))
        .with_graceful_shutdown(shutdown())
        .await;
    state.postgres.close().await;
    state
        .redis
        .quit()
        .await
        .context("closing Redis connection")?;
    result.context("serving Hiveway")
}

async fn shutdown() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "Failed to listen for Ctrl-C");
        }
    };
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { () = interrupt => {}, _ = terminate.recv() => {} }
            }
            Err(error) => {
                tracing::error!(%error, "Failed to listen for SIGTERM");
                interrupt.await;
            }
        }
    }
    #[cfg(not(unix))]
    interrupt.await;
}
