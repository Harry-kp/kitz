//! Convenience re-exports for the most common types.
//!
//! ```
//! use kitz::prelude::*;
//! ```

pub use color_eyre::Result;
pub use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub use crate::error::KitzError;
pub use ratatui::layout::{Constraint, Layout, Rect};
pub use ratatui::widgets::Paragraph;
pub use ratatui::Frame;

pub use crate::app::{Application, EventResult};
pub use crate::command::Command;
pub use crate::context::{Context, EventContext, ViewContext};
pub use crate::overlay::{CommandPaletteOverlay, ConfirmOverlay, PaletteCommand};
pub use crate::panel::{KeyHint, PanelId, PanelLayout};
pub use crate::screen::Screen;
pub use crate::subscription::Subscription;
pub use crate::testing::TestHarness;
pub use crate::theme::Theme;
pub use crate::toast::ToastLevel;
pub use crate::widgets::{centered_rect, TextInput, TextInputState};
