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
    let config = Config::from_env()?;
    let state = AppState::connect(&config).await?;
    let worker_state = state.clone();
    let worker = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = hiveway::features::pollen::worker::tick(&worker_state).await {
                tracing::error!(error = error.code, "Pollen recovery cycle failed");
            }
        }
    });
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    tracing::info!(
        service = "hiveway",
        address = %config.bind_address,
        "Hiveway ready"
    );
    let result = axum::serve(
        listener,
        hiveway::observe(hiveway::router().merge(hiveway::api::router()))
            .layer(hiveway::api::cors(&state.settings.auth_uri)?)
            .with_state(state.clone())
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await;
    worker.abort();
    let _ = worker.await;
    state.postgres.close().await?;
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
