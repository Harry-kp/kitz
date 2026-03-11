use std::fmt::Debug;
use std::sync::mpsc;
use std::time::Duration;

/// A declarative subscription that the runtime manages.
///
/// Subscriptions are re-evaluated every frame: if a subscription's id appears
/// in the current set but not the previous one, the runtime starts it. If it
/// disappears, the runtime stops it.
pub struct Subscription<M> {
    pub(crate) id: &'static str,
    pub(crate) kind: SubKind<M>,
}

pub(crate) enum SubKind<M> {
    /// Fire a message at a fixed interval.
    Every(Duration, Box<dyn Fn() -> M + Send>),
}

impl<M: Debug + Send + 'static> Subscription<M> {
    /// No subscriptions.
    pub fn none() -> Vec<Self> {
        vec![]
    }

    /// Emit a message at a fixed interval.
    ///
    /// ```ignore
    /// Subscription::every("clock-tick", Duration::from_secs(1), || Msg::Tick)
    /// ```
    pub fn every(
        id: &'static str,
        interval: Duration,
        msg_fn: impl Fn() -> M + Send + 'static,
    ) -> Self {
        Subscription {
            id,
            kind: SubKind::Every(interval, Box::new(msg_fn)),
        }
    }
}

/// Manages running subscriptions. Diffs the desired set against the active
/// set each frame.
pub(crate) struct SubscriptionManager<M: Debug + Send + 'static> {
    active: Vec<ActiveSub>,
    _phantom: std::marker::PhantomData<M>,
}

struct ActiveSub {
    id: &'static str,
    handle: Option<std::thread::JoinHandle<()>>,
    cancel: mpsc::Sender<()>,
}

impl<M: Debug + Send + 'static> SubscriptionManager<M> {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Diff the desired subscriptions against the currently active ones.
    /// Start new ones, stop removed ones.
    pub fn sync(&mut self, desired: Vec<Subscription<M>>, msg_tx: &mpsc::Sender<M>) {
        let desired_ids: Vec<&str> = desired.iter().map(|s| s.id).collect();

        // Stop subscriptions that are no longer desired
        self.active.retain(|active| {
            if desired_ids.contains(&active.id) {
                true
            } else {
                let _ = active.cancel.send(());
                false
            }
        });

        let active_ids: Vec<&str> = self.active.iter().map(|a| a.id).collect();

        // Start new subscriptions
        for sub in desired {
            if active_ids.contains(&sub.id) {
                continue;
            }

            let (cancel_tx, cancel_rx) = mpsc::channel();
            let tx = msg_tx.clone();

            let handle = match sub.kind {
                SubKind::Every(interval, msg_fn) => std::thread::spawn(move || loop {
                    std::thread::sleep(interval);
                    if cancel_rx.try_recv().is_ok() {
                        break;
                    }
                    let msg = msg_fn();
                    if tx.send(msg).is_err() {
                        break;
                    }
                }),
            };

            self.active.push(ActiveSub {
                id: sub.id,
                handle: Some(handle),
                cancel: cancel_tx,
            });
        }
    }

    /// Shut down all subscriptions.
    pub fn shutdown(&mut self) {
        for sub in &self.active {
            let _ = sub.cancel.send(());
        }
        for sub in &mut self.active {
            if let Some(handle) = sub.handle.take() {
                let _ = handle.join();
            }
        }
        self.active.clear();
    }
}

impl<M: Debug + Send + 'static> Drop for SubscriptionManager<M> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
