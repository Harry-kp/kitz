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
    /// Nested layout tree for complex arrangements.
    ///
    /// Example: a sidebar + vertically-stacked main panels:
    /// ```
    /// # use rataframe::panel::{PanelLayout, PanelId};
    /// # use ratatui::layout::{Constraint, Direction};
    /// PanelLayout::nested(Direction::Horizontal, vec![
    ///     (PanelLayout::single("sidebar"), Constraint::Percentage(30)),
    ///     (PanelLayout::vertical(vec![
    ///         ("top", Constraint::Percentage(60)),
    ///         ("bottom", Constraint::Percentage(40)),
    ///     ]), Constraint::Percentage(70)),
    /// ]);
    /// ```
    Nested(Direction, Vec<(Box<PanelLayout>, Constraint)>),
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

    /// Build a nested layout tree for complex multi-direction arrangements.
    pub fn nested(direction: Direction, children: Vec<(PanelLayout, Constraint)>) -> Self {
        Self::Nested(
            direction,
            children
                .into_iter()
                .map(|(layout, constraint)| (Box::new(layout), constraint))
                .collect(),
        )
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
            Self::Nested(_, children) => children
                .iter()
                .flat_map(|(layout, _)| layout.panel_ids())
                .collect(),
        }
    }

    /// Compute the `Rect` for each panel given the available area.
    pub fn compute_rects(&self, area: Rect) -> Vec<(PanelId, Rect)> {
        match self {
            Self::None => vec![],
            Self::Single(id) => vec![(*id, area)],
            Self::Horizontal(panels) => Self::compute_flat(panels, Direction::Horizontal, area),
            Self::Vertical(panels) => Self::compute_flat(panels, Direction::Vertical, area),
            Self::Nested(direction, children) => {
                let constraints: Vec<Constraint> = children.iter().map(|(_, c)| *c).collect();
                let rects = Layout::default()
                    .direction(*direction)
                    .constraints(constraints)
                    .split(area);
                children
                    .iter()
                    .zip(rects.iter())
                    .flat_map(|((layout, _), rect)| layout.compute_rects(*rect))
                    .collect()
            }
        }
    }

    fn compute_flat(
        panels: &[(PanelId, Constraint)],
        direction: Direction,
        area: Rect,
    ) -> Vec<(PanelId, Rect)> {
        let constraints: Vec<Constraint> = panels.iter().map(|(_, c)| *c).collect();
        let rects = Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(area);
        panels
            .iter()
            .zip(rects.iter())
            .map(|((id, _), rect)| (*id, *rect))
            .collect()
    }
}
