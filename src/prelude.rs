//! Convenience re-exports for the most common types.
//!
//! ```
//! use rataframe::prelude::*;
//! ```

pub use color_eyre::Result;
pub use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
pub use ratatui::layout::{Constraint, Layout, Rect};
pub use ratatui::widgets::Paragraph;
pub use ratatui::Frame;

pub use crate::app::{Application, EventResult};
pub use crate::command::Command;
pub use crate::context::{Context, EventContext, ViewContext};
