//! Split view: two open documents side by side in one window. The active tab is one pane;
//! tools, panels and keys follow it. A press in the other pane makes that pane active.

use crate::PdfCraftApp;
use crate::theme::Tokens;

impl PdfCraftApp {
    /// The view index of the pane that is not active, while split view shows.
    pub(crate) fn split_partner(&self) -> Option<usize> {
        if self.full_screen {
            return None;
        }
        let pair = self.split?;
        let active_id = self.views.get(self.active?)?.id;
        let other = pair.into_iter().find(|id| *id != active_id)?;
        self.views.iter().position(|v| v.id == other)
    }
}

/// Start split view (the active tab on the left, the next tab on the right) or end it.
pub(crate) fn toggle_split(app: &mut PdfCraftApp) {
    if app.split.is_some() {
        app.split = None;
        return;
    }
    let Some(a) = app.active else { return };
    if app.views.len() < 2 {
        app.notify_error("Open a second PDF to use split view");
        return;
    }
    let b = (a + 1) % app.views.len();
    app.split = Some([app.views[a].id, app.views[b].id]);
    app.split_focus = 0;
}

/// Keep the split consistent with the tabs, once per frame before drawing: it ends when a pane's
/// tab closes or no tab is active; the active tab takes the focused pane, or moves the focus when
/// it is already the other pane.
pub(crate) fn reconcile(app: &mut PdfCraftApp) {
    let Some(mut pair) = app.split else { return };
    let active_id = app.active.and_then(|i| app.views.get(i)).map(|v| v.id);
    let open = pair.iter().all(|id| app.views.iter().any(|v| v.id == *id));
    let Some(active_id) = active_id.filter(|_| open) else {
        app.split = None;
        return;
    };
    if let Some(k) = pair.iter().position(|id| *id == active_id) {
        app.split_focus = k;
    } else {
        let focus = app.split_focus.min(1);
        pair[focus] = active_id;
        app.split = Some(pair);
    }
}

/// The two panes, each a page view of its own with a title strip. The pane under a press
/// becomes the active tab.
pub(crate) fn split_area(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let Some(pair) = app.split else { return };
    let t = Tokens::get(ui.ctx());
    let rect = ui.available_rect_before_wrap();
    let gap = 1.0;
    let half_w = ((rect.width() - gap) / 2.0).max(1.0);
    let caption_h = 22.0;
    let halves = [
        egui::Rect::from_min_size(rect.min, egui::vec2(half_w, rect.height())),
        egui::Rect::from_min_size(egui::pos2(rect.min.x + half_w + gap, rect.min.y), egui::vec2(half_w, rect.height())),
    ];
    // A press in the other pane gives it the focus.
    if ui.input(|i| i.pointer.any_pressed())
        && let Some(p) = ui.input(|i| i.pointer.hover_pos())
        && let Some(k) = halves.iter().position(|r| r.contains(p))
        && k != app.split_focus
        && let Some(i) = app.views.iter().position(|v| v.id == pair[k])
    {
        app.active = Some(i);
        app.split_focus = k;
    }
    for (k, r) in halves.into_iter().enumerate() {
        // Drawing a pane can close its tab and shift the indexes: find each pane again by id.
        let Some(index) = app.views.iter().position(|v| v.id == pair[k]) else { return };
        let name = app.session.get(pair[k]).map(|d| d.name.clone()).unwrap_or_default();
        let caption = egui::Rect::from_min_size(r.min, egui::vec2(r.width(), caption_h));
        let focused = k == app.split_focus;
        ui.painter().rect_filled(caption, 0.0, if focused { t.accent_soft } else { t.panel });
        ui.painter().text(
            egui::pos2(caption.min.x + 10.0, caption.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(12.0),
            if focused { t.text } else { t.text_muted },
        );
        let body = egui::Rect::from_min_max(egui::pos2(r.min.x, r.min.y + caption_h), r.max);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body).id_salt(("split-half", k)));
        child.set_clip_rect(body);
        crate::canvas::document_area(app, index, &mut child);
    }
    ui.painter().vline(rect.min.x + half_w, rect.y_range(), egui::Stroke::new(gap, t.border));
}