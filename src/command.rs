use std::fmt::Debug;

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
    // Future phases will add:
    // Perform(BoxFuture<M>),
    // PushOverlay(Box<dyn Overlay<M>>),
    // PopOverlay,
    // PushScreen(Box<dyn Screen<M>>),
    // PopScreen,
    // Toast(String, ToastLevel),
    // FocusPanel(PanelId),
    // ToggleZoom,
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

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}
