use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::Duration;

use diesel::PgConnection;
use diesel::RunQueryDsl;
use diesel::sql_query;

use crate::inner::Inner;
use crate::invalidation::Invalidation;
use crate::{Error, NotifyChannel, PgPool};

/// How long the loop sleeps when no notification is pending.
const POLL_INTERVAL: Duration = Duration::from_millis(100);
/// First reconnect delay; doubles after every failure.
const INITIAL_BACKOFF: Duration = Duration::from_millis(100);
/// Upper bound of the reconnect delay.
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// How long `spawn` waits for the first LISTEN to be in place.
const READY_TIMEOUT: Duration = Duration::from_secs(5);

// TODO(v2:multi-instance): nothing needed now; LISTEN/NOTIFY already covers multiple instances.

/// Starts the listener thread for `inner` and waits until it is listening.
///
/// The thread keeps one connection taken from the pool for its whole life
/// (and takes another from the pool when it has to reconnect), so the pool
/// must have at least two connections. It holds only a `Weak` reference to
/// `inner` and exits within one poll interval after the last client is dropped.
///
/// If the first connection attempt fails, `spawn` still returns; the thread
/// keeps retrying with backoff in the background.
pub(crate) fn spawn(inner: &Arc<Inner>) -> Result<(), Error> {
    let weak = Arc::downgrade(inner);
    let pool = inner.pool.clone();
    let channel = inner.channel.clone();
    let (ready_tx, ready_rx) = sync_channel::<()>(1);
    thread::Builder::new()
        .name("air-tight-listener".to_owned())
        .spawn(move || run(weak, pool, channel, ready_tx))
        .map_err(|err| Error::Notify(format!("could not start listener thread: {err}")))?;
    if ready_rx.recv_timeout(READY_TIMEOUT).is_err() {
        tracing::warn!("listener did not become ready in time, continuing without waiting");
    }
    Ok(())
}

fn run(weak: Weak<Inner>, pool: PgPool, channel: NotifyChannel, ready: SyncSender<()>) {
    let mut backoff = INITIAL_BACKOFF;
    let mut reconnecting = false;
    loop {
        if weak.strong_count() == 0 {
            return;
        }
        let outcome = session(&weak, &pool, &channel, &ready, reconnecting, &mut backoff);
        match outcome {
            Ok(()) => return,
            Err(err) => {
                // Unblocks `spawn` when the very first attempt failed.
                let _ = ready.try_send(());
                tracing::warn!(error = %err, retry_in = ?backoff, "listener connection lost");
                if !sleep_while_alive(&weak, backoff) {
                    return;
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                reconnecting = true;
            }
        }
    }
}

/// Runs one connection until it fails (`Err`) or all clients are gone (`Ok`).
fn session(
    weak: &Weak<Inner>,
    pool: &PgPool,
    channel: &NotifyChannel,
    ready: &SyncSender<()>,
    reconnecting: bool,
    backoff: &mut Duration,
) -> Result<(), Error> {
    let mut conn = pool.get()?;
    // The channel is a validated `NotifyChannel` (`[a-z_][a-z0-9_]*`), so
    // formatting it into the statement cannot inject SQL; LISTEN accepts no bind
    // parameters.
    sql_query(format!("LISTEN {channel}")).execute(&mut *conn)?;
    *backoff = INITIAL_BACKOFF;
    if reconnecting {
        // Notifications sent while we were away are lost, so start clean.
        match weak.upgrade() {
            Some(inner) => inner.invalidate_all(),
            None => return Ok(()),
        }
    }
    let _ = ready.try_send(());
    loop {
        let batch = drain(&mut conn)?;
        let Some(inner) = weak.upgrade() else {
            return Ok(());
        };
        if batch.is_empty() {
            drop(inner);
            thread::sleep(POLL_INTERVAL);
            continue;
        }
        for payload in batch {
            match Invalidation::parse(&payload) {
                Some(invalidation) => inner.apply(&mut conn, &invalidation),
                None => tracing::warn!(payload = %payload, "ignoring unknown notify payload"),
            }
        }
    }
}

/// Reads every pending notification payload; this also pumps the socket.
fn drain(conn: &mut PgConnection) -> Result<Vec<String>, Error> {
    let mut payloads = Vec::new();
    for notification in conn.notifications_iter() {
        payloads.push(notification?.payload);
    }
    Ok(payloads)
}

/// Sleeps for `total` in short slices; returns false once all clients are gone.
fn sleep_while_alive(weak: &Weak<Inner>, total: Duration) -> bool {
    let mut left = total;
    while !left.is_zero() {
        if weak.strong_count() == 0 {
            return false;
        }
        let step = left.min(POLL_INTERVAL);
        thread::sleep(step);
        left -= step;
    }
    weak.strong_count() > 0
}
