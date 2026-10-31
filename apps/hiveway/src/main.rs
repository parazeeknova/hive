use anyhow::Context;
use fred::prelude::ClientLike;
use hiveway::{config::Config, state::AppState, telemetry};
use std::future::IntoFuture;

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
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    tracing::info!(
        service = "hiveway",
        address = %config.bind_address,
        "Hiveway ready"
    );
    let cors = hiveway::api::cors(&state.settings.auth_uri)?;
    let (router, drain) = hiveway::api::protect(
        hiveway::observe(hiveway::router().merge(hiveway::api::router())).with_state(state.clone()),
        state.clone(),
    );
    let (stop_worker, worker_shutdown) = tokio::sync::oneshot::channel();
    let (stop_server, server_shutdown) = tokio::sync::oneshot::channel();
    let mut worker = tokio::spawn(background_workers(state.clone(), worker_shutdown));
    let server = axum::serve(
        listener,
        router
            .layer(cors)
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::select! {
            () = shutdown() => {},
            _ = server_shutdown => {},
        }
    });
    let server = server.into_future();
    tokio::pin!(server);
    let (result, worker_result) = tokio::select! {
        result = &mut server => {
            let _ = stop_worker.send(());
            (result, worker.await.context("Background worker task failed"))
        }
        outcome = &mut worker => {
            let error = match outcome {
                Ok(()) => anyhow::anyhow!("Background worker task stopped unexpectedly"),
                Err(error) => anyhow::Error::new(error).context("Background worker task failed"),
            };
            tracing::error!(%error, "Background workers stopped");
            let _ = stop_server.send(());
            (server.await, Err(error))
        }
    };
    if let Err(error) = &worker_result {
        tracing::error!(%error, "Background worker task failed");
    }
    let drained = drain
        .wait(std::time::Duration::from_secs(
            state.settings.runtime.http.shutdown_drain_timeout_seconds,
        ))
        .await;
    if let Err(error) = &drained {
        tracing::error!(%error, "Handler drain deadline exceeded; proceeding with shutdown");
    }
    let postgres = state.postgres.close().await;
    let redis = state.redis.quit().await;
    result.context("serving Hiveway")?;
    postgres.context("closing PostgreSQL connection")?;
    redis.context("closing Redis connection")?;
    let _drained = drained.context("draining Hiveway handlers")?;
    worker_result
}

async fn background_workers(state: AppState, shutdown: tokio::sync::oneshot::Receiver<()>) {
    let (stop_recovery, recovery_shutdown) = tokio::sync::oneshot::channel();
    let (stop_builder, builder_shutdown) = tokio::sync::oneshot::channel();
    let builder = builder_worker(
        || hiveway::features::builder::worker::tick(&state),
        builder_shutdown,
    );
    let stop = async {
        let _ = shutdown.await;
        let _ = stop_recovery.send(());
        let _ = stop_builder.send(());
    };
    tokio::join!(
        recovery_worker(state.clone(), recovery_shutdown),
        builder,
        stop
    );
}

async fn builder_worker<F, Fut>(mut tick: F, mut shutdown: tokio::sync::oneshot::Receiver<()>)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = hiveway::features::common::error::ApiResult<()>>,
{
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! { biased; _=&mut shutdown=>break, _=interval.tick()=>{} }
        tokio::select! {
            biased;
            _=&mut shutdown=>break,
            result=tick()=>{if let Err(error)=result {tracing::error!(error=error.code,"Hosted builder cycle failed");}}
        }
    }
}

async fn recovery_worker(state: AppState, mut shutdown: tokio::sync::oneshot::Receiver<()>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(
        state.settings.runtime.worker.interval_seconds,
    ));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => break,
            _ = interval.tick() => {},
        }
        // Shutdown is observed between ticks so financial recovery is not cancelled midway.
        if let Err(error) = hiveway::features::pollen::worker::tick(&state).await {
            tracing::error!(error = error.code, "Pollen recovery cycle failed");
        }
    }
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

#[cfg(all(test, not(miri)))]
mod tests {
    #[tokio::test]
    async fn shutdown_cancels_a_running_builder_tick() -> anyhow::Result<()> {
        let started = std::sync::Arc::new(tokio::sync::Notify::new());
        let signal = started.clone();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(super::builder_worker(
            move || {
                let signal = signal.clone();
                async move {
                    signal.notify_one();
                    std::future::pending().await
                }
            },
            shutdown,
        ));
        tokio::time::timeout(std::time::Duration::from_secs(2), started.notified()).await?;
        let _ = stop.send(());
        tokio::time::timeout(std::time::Duration::from_secs(1), worker).await??;
        Ok(())
    }
}
