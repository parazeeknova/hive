//! Cross-replica wakeups for long-polls, task waits and MCP subscriptions.
//!
//! Writers commit their state change first, then call [`publish`]. Every replica
//! runs one `LISTEN hive_events` connection that wakes local waiters; the
//! publishing replica also wakes its own waiters directly, so a single process
//! (including tests without a listener) never depends on the database round trip.
//! Notifications only say "look again": waiters always re-read the database, so a
//! lost notification costs at most one fallback interval, never correctness.
use crate::{features::common::error::ApiResult, state::AppState};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::Notify;

const CHANNEL: &str = "hive_events";
type Waiters = HashMap<String, Vec<Weak<Notify>>>;

/// Every watch handle has exactly one consumer, so wakeups use `notify_one`:
/// a wake that arrives while the consumer is busy (for example, between two
/// reads of an SSE stream) is stored as a permit instead of being lost.
#[derive(Clone)]
pub struct Realtime {
    waiters: Arc<Mutex<Waiters>>,
    subscriptions: Arc<tokio::sync::Semaphore>,
    per_credential: Arc<Mutex<HashMap<uuid::Uuid, usize>>>,
}
impl Default for Realtime {
    fn default() -> Self {
        Self::new(512)
    }
}
/// Holds one open `subscriptions/listen` stream; released on drop.
pub struct SubscriptionSlot {
    _permit: tokio::sync::OwnedSemaphorePermit,
    credential: uuid::Uuid,
    counts: Arc<Mutex<HashMap<uuid::Uuid, usize>>>,
}
impl Drop for SubscriptionSlot {
    fn drop(&mut self) {
        if let Ok(mut counts) = self.counts.lock()
            && let Some(count) = counts.get_mut(&self.credential)
        {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&self.credential);
            }
        }
    }
}

impl Realtime {
    #[must_use]
    pub fn new(max_subscriptions: usize) -> Self {
        Self {
            waiters: Arc::default(),
            subscriptions: Arc::new(tokio::sync::Semaphore::new(max_subscriptions)),
            per_credential: Arc::default(),
        }
    }
    /// Reserve a long-lived notification stream within process and credential bounds.
    #[must_use]
    pub fn subscription(
        &self,
        credential: uuid::Uuid,
        per_credential: usize,
    ) -> Option<SubscriptionSlot> {
        let mut counts = self.per_credential.lock().ok()?;
        let count = counts.entry(credential).or_default();
        if *count >= per_credential {
            return None;
        }
        let permit = self.subscriptions.clone().try_acquire_owned().ok()?;
        *count += 1;
        Some(SubscriptionSlot {
            _permit: permit,
            credential,
            counts: self.per_credential.clone(),
        })
    }
    /// Register one wakeup handle under every key. Dropping the handle unregisters it.
    #[must_use]
    pub fn watch(&self, keys: &[String]) -> Arc<Notify> {
        let notify = Arc::new(Notify::new());
        if let Ok(mut waiters) = self.waiters.lock() {
            if waiters.len() > 4096 {
                waiters.retain(|_, list| {
                    list.retain(|weak| weak.strong_count() > 0);
                    !list.is_empty()
                });
            }
            for key in keys {
                waiters
                    .entry(key.clone())
                    .or_default()
                    .push(Arc::downgrade(&notify));
            }
        }
        notify
    }
    /// Wake local waiters registered under `key`.
    pub fn wake(&self, key: &str) {
        let live: Vec<Arc<Notify>> = self.waiters.lock().map_or_else(
            |_| Vec::new(),
            |mut waiters| {
                let Some(list) = waiters.get_mut(key) else {
                    return Vec::new();
                };
                list.retain(|weak| weak.strong_count() > 0);
                let live = list.iter().filter_map(Weak::upgrade).collect();
                if list.is_empty() {
                    waiters.remove(key);
                }
                live
            },
        );
        for notify in live {
            notify.notify_one();
        }
    }
    /// Wake everything after a possible gap in delivered notifications.
    pub fn wake_all(&self) {
        let live: Vec<Arc<Notify>> = self.waiters.lock().map_or_else(
            |_| Vec::new(),
            |waiters| {
                waiters
                    .values()
                    .flatten()
                    .filter_map(Weak::upgrade)
                    .collect()
            },
        );
        for notify in live {
            notify.notify_one();
        }
    }
    /// Relay database notifications from other replicas until shutdown.
    pub async fn listen(
        &self,
        pool: sea_orm::sqlx::PgPool,
        mut shutdown: tokio::sync::oneshot::Receiver<()>,
    ) {
        use sea_orm::sqlx::postgres::PgListener;
        loop {
            let mut listener = match PgListener::connect_with(&pool).await {
                Ok(listener) => listener,
                Err(error) => {
                    tracing::warn!(%error, "Realtime listener could not connect; retrying");
                    tokio::select! {
                        _ = &mut shutdown => return,
                        () = tokio::time::sleep(Duration::from_secs(2)) => continue,
                    }
                }
            };
            if let Err(error) = listener.listen(CHANNEL).await {
                tracing::warn!(%error, "Realtime LISTEN failed; retrying");
                tokio::select! {
                    _ = &mut shutdown => return,
                    () = tokio::time::sleep(Duration::from_secs(2)) => continue,
                }
            }
            // Anything published while disconnected was missed: let waiters re-check.
            self.wake_all();
            loop {
                tokio::select! {
                    _ = &mut shutdown => return,
                    received = listener.recv() => match received {
                        Ok(notification) => self.wake(notification.payload()),
                        Err(error) => {
                            tracing::warn!(%error, "Realtime listener disconnected");
                            self.wake_all();
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Wake waiters on every replica. Call after the state change has committed.
/// # Errors
/// Returns database errors; local waiters are woken regardless.
pub async fn publish(state: &AppState, keys: &[String]) -> ApiResult<()> {
    for key in keys {
        state.realtime.wake(key);
    }
    for key in keys {
        state
            .postgres
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT pg_notify($1, $2)",
                vec![CHANNEL.into(), key.clone().into()],
            ))
            .await?;
    }
    Ok(())
}

/// Best-effort variant for paths where the committed state is already durable.
pub async fn announce(state: &AppState, keys: &[String]) {
    if let Err(error) = publish(state, keys).await {
        tracing::warn!(error = error.code, "Realtime notification failed");
    }
}

#[must_use]
pub fn job_key(id: uuid::Uuid) -> String {
    format!("job:{}", id.simple())
}
#[must_use]
pub fn connection_key(id: uuid::Uuid) -> String {
    format!("conn:{}", id.simple())
}
#[must_use]
pub fn owner_key(owner: &str) -> String {
    format!("owner:{owner}")
}

/// Wait until `check` yields a value, re-checking on every wakeup and at least
/// every `fallback`. The wakeup handle is armed before each check, so a change
/// committed between the check and the wait is never lost.
/// # Errors
/// Propagates errors from `check`.
pub async fn wait_for<T, F, Fut>(
    realtime: &Realtime,
    keys: &[String],
    deadline: tokio::time::Instant,
    fallback: Duration,
    mut check: F,
) -> ApiResult<Option<T>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ApiResult<Option<T>>>,
{
    let notify = realtime.watch(keys);
    loop {
        let notified = notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if let Some(value) = check().await? {
            return Ok(Some(value));
        }
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        let pause = fallback.min(deadline - now);
        tokio::select! {
            () = &mut notified => {},
            () = tokio::time::sleep(pause) => {},
        }
    }
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn wakeups_reach_every_key_and_expire_with_handles() -> ApiResult<()> {
        let realtime = Realtime::default();
        let keys = vec!["job:a".to_owned(), "owner:b".to_owned()];
        let seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = seen.clone();
        let waker = realtime.clone();
        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            counter.store(1, std::sync::atomic::Ordering::SeqCst);
            waker.wake("owner:b");
        });
        let started = tokio::time::Instant::now();
        let value = wait_for(
            &realtime,
            &keys,
            started + Duration::from_secs(5),
            Duration::from_secs(5),
            || {
                let seen = seen.clone();
                async move {
                    Ok((seen.load(std::sync::atomic::Ordering::SeqCst) == 1).then_some(7))
                }
            },
        )
        .await?;
        assert_eq!(value, Some(7));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "woken, not polled"
        );
        let _ = task.await;
        // Handles are dropped; waking an unknown key is a no-op and prunes state.
        realtime.wake("owner:b");
        assert!(
            realtime
                .waiters
                .lock()
                .is_ok_and(|w| !w.contains_key("owner:b"))
        );
        Ok(())
    }
    #[tokio::test]
    async fn deadline_returns_none() -> ApiResult<()> {
        let realtime = Realtime::default();
        let value: Option<()> = wait_for(
            &realtime,
            &["job:x".to_owned()],
            tokio::time::Instant::now() + Duration::from_millis(40),
            Duration::from_millis(10),
            || async { Ok(None) },
        )
        .await?;
        assert!(value.is_none());
        Ok(())
    }
}
