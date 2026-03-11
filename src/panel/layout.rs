use ratatui::layout::{Constraint, Direction, Layout, Rect};

use super::PanelId;

/// Describes how panels are arranged on screen.
///
/// The convention path: implement `Application::panels()` and return one of
/// these. The framework renders borders, handles focus, and auto-generates
/// the footer and help overlay.
#[derive(Debug, Clone)]
pub enum PanelLayout {
    /// No panels — the app uses the custom `view()` path.
    None,
    /// A single panel filling the main area.
    Single(PanelId),
    /// Side-by-side panels, left to right.
    Horizontal(Vec<(PanelId, Constraint)>),
    /// Stacked panels, top to bottom.
    Vertical(Vec<(PanelId, Constraint)>),
}

impl PanelLayout {
    pub fn none() -> Self {
        Self::None
    }

    pub fn single(id: PanelId) -> Self {
        Self::Single(id)
    }

    pub fn horizontal(panels: Vec<(PanelId, Constraint)>) -> Self {
        Self::Horizontal(panels)
    }

    pub fn vertical(panels: Vec<(PanelId, Constraint)>) -> Self {
        Self::Vertical(panels)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Collect all panel IDs in display order.
    pub fn panel_ids(&self) -> Vec<PanelId> {
        match self {
            Self::None => vec![],
            Self::Single(id) => vec![id],
            Self::Horizontal(panels) | Self::Vertical(panels) => {
                panels.iter().map(|(id, _)| *id).collect()
            }
        }
    }

    /// Compute the `Rect` for each panel given the available area.
    pub fn compute_rects(&self, area: Rect) -> Vec<(PanelId, Rect)> {
        match self {
            Self::None => vec![],
            Self::Single(id) => vec![(*id, area)],
            Self::Horizontal(panels) => {
                let constraints: Vec<Constraint> = panels.iter().map(|(_, c)| *c).collect();
                let rects = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(area);
                panels
                    .iter()
                    .zip(rects.iter())
                    .map(|((id, _), rect)| (*id, *rect))
                    .collect()
            }
            Self::Vertical(panels) => {
                let constraints: Vec<Constraint> = panels.iter().map(|(_, c)| *c).collect();
                let rects = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(area);
                panels
                    .iter()
                    .zip(rects.iter())
                    .map(|((id, _), rect)| (*id, *rect))
                    .collect()
            }
        }
    }
}
