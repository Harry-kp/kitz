use super::PanelId;

/// Tracks panel focus state and zoom toggle. Owned by the runtime.
#[derive(Debug)]
pub struct PanelManager {
    panel_ids: Vec<PanelId>,
    focused_idx: usize,
    zoomed: bool,
}

impl PanelManager {
    pub fn new(panel_ids: Vec<PanelId>) -> Self {
        Self {
            panel_ids,
            focused_idx: 0,
            zoomed: false,
        }
    }

    pub fn focused_id(&self) -> Option<PanelId> {
        self.panel_ids.get(self.focused_idx).copied()
    }

    pub fn is_focused(&self, id: PanelId) -> bool {
        self.focused_id() == Some(id)
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoomed
    }

    pub fn focus_next(&mut self) {
        if !self.panel_ids.is_empty() {
            self.focused_idx = (self.focused_idx + 1) % self.panel_ids.len();
        }
    }

    pub fn focus_prev(&mut self) {
        if !self.panel_ids.is_empty() {
            self.focused_idx = (self.focused_idx + self.panel_ids.len() - 1) % self.panel_ids.len();
        }
    }

    pub fn focus_panel(&mut self, id: PanelId) {
        if let Some(idx) = self.panel_ids.iter().position(|&pid| pid == id) {
            self.focused_idx = idx;
        }
    }

    pub fn toggle_zoom(&mut self) {
        self.zoomed = !self.zoomed;
    }

    pub fn panel_ids(&self) -> &[PanelId] {
        &self.panel_ids
    }

    /// Sync with a new layout (e.g. after screen change). Preserves focus
    /// if the previously focused panel still exists.
    pub fn sync_layout(&mut self, ids: Vec<PanelId>) {
        let old_focus = self.focused_id();
        self.panel_ids = ids;
        self.focused_idx = 0;
        if let Some(old) = old_focus {
            if let Some(idx) = self.panel_ids.iter().position(|&pid| pid == old) {
                self.focused_idx = idx;
            }
        }
    }
}
