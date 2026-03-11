use std::fmt::Debug;

/// Mutable context passed to [`Application::update`](crate::app::Application::update).
///
/// In Phase 1 this is intentionally minimal. Later phases will expose the
/// overlay stack, navigation stack, toast queue, and more through this struct.
pub struct Context<M: Debug + Send + 'static> {
    _marker: std::marker::PhantomData<M>,
}

impl<M: Debug + Send + 'static> Context<M> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

/// Read-only context passed to [`Application::view`](crate::app::Application::view).
///
/// Will eventually carry the current theme, panel state, and a
/// `render_panels()` helper. For Phase 1 it is an empty marker.
pub struct ViewContext;

impl ViewContext {
    pub(crate) fn new() -> Self {
        Self
    }
}

/// Read-only context passed to
/// [`Application::handle_event`](crate::app::Application::handle_event).
///
/// Will carry information about the currently focused panel, active overlay,
/// etc. For Phase 1 it is an empty marker.
pub struct EventContext;

impl EventContext {
    pub(crate) fn new() -> Self {
        Self
    }
}
