use std::fmt::Debug;
use std::sync::mpsc;

/// Describes side-effects that the runtime should execute after
/// [`Application::update`](crate::app::Application::update) returns.
///
/// Commands are **values, not callbacks** — inspired by Iced / Elm. The
/// application never executes side-effects directly; it returns a `Command`
/// and the runtime takes care of the rest.
pub struct Command<M> {
    pub(crate) actions: Vec<Action<M>>,
}

pub(crate) enum Action<M> {
    Quit,
    Message(M),
    /// Spawn a background task. The closure runs on a new thread and sends
    /// the resulting message through the channel.
    Perform(Box<dyn FnOnce(mpsc::Sender<M>) + Send>),
}

impl<M: Debug + Send + 'static> Command<M> {
    /// No side-effects.
    pub fn none() -> Self {
        Command {
            actions: Vec::new(),
        }
    }

    /// Tell the runtime to shut down the application.
    pub fn quit() -> Self {
        Command {
            actions: vec![Action::Quit],
        }
    }

    /// Immediately re-dispatch a message through
    /// [`Application::update`](crate::app::Application::update).
    pub fn message(msg: M) -> Self {
        Command {
            actions: vec![Action::Message(msg)],
        }
    }

    /// Combine multiple commands into one. All actions execute.
    pub fn batch(cmds: impl IntoIterator<Item = Command<M>>) -> Self {
        Command {
            actions: cmds.into_iter().flat_map(|c| c.actions).collect(),
        }
    }

    /// Spawn a background task. The `task` closure runs on a new thread.
    /// When it completes, `mapper` converts the result into a message that
    /// gets dispatched to `Application::update`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Command::perform(
    ///     || reqwest::blocking::get("https://api.example.com/data")?.text(),
    ///     |result| match result {
    ///         Ok(body) => Msg::FetchSuccess(body),
    ///         Err(e) => Msg::FetchError(e.to_string()),
    ///     },
    /// )
    /// ```
    pub fn perform<T, F, Map>(task: F, mapper: Map) -> Self
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
        Map: FnOnce(T) -> M + Send + 'static,
    {
        Command {
            actions: vec![Action::Perform(Box::new(move |tx: mpsc::Sender<M>| {
                let result = task();
                let msg = mapper(result);
                let _ = tx.send(msg);
            }))],
        }
    }

    /// Returns `true` if this command has no actions.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}
