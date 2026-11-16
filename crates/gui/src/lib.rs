//! Mathcad-style egui front end. No mathematical evaluation happens here.
pub mod canvas;
pub mod editor;
pub mod math_render;
pub mod palette;
pub mod panels;
pub mod plots;
pub mod state;
pub mod theme;

pub use state::{UiAction, UiState};

use aaiyyan_core::Session;
use std::time::Duration;

pub fn render(ctx: &egui::Context, session: &mut Session, ui: &mut UiState) {
    theme::install_fonts(ctx);
    theme::apply(ctx, &session.project.layout);
    ctx.set_zoom_factor(1.0);
    shortcuts(ctx, session, ui);

    let mut actions = Vec::new();
    // Panel order matters: the status strip claims the outermost bottom edge first.
    panels::status_bar(ctx, session, ui, &mut actions);
    panels::ribbon(ctx, session, ui, &mut actions);
    if session.project.layout.show_bottom { panels::bottom_panel(ctx, session, ui, &mut actions); }
    if session.project.layout.show_left { panels::left_panel(ctx, session, ui, &mut actions); }
    if session.project.layout.show_right { panels::right_panel(ctx, session, ui, &mut actions); }
    canvas::show(ctx, session, ui, &mut actions);
    palette::show(ctx, ui, &mut actions);

    panels::apply_actions(ctx, session, ui, actions);

    if session.stale && session.project.settings.mode == aaiyyan_project::CalcMode::Auto {
        session.request_recalc();
    }
    if session.poll() { ctx.request_repaint(); }
    if session.busy { ctx.request_repaint_after(Duration::from_millis(40)); }
}

fn shortcuts(ctx: &egui::Context, session: &mut Session, ui: &mut UiState) {
    let mut actions = Vec::new();
    let editing = ui.editing;
    let cursor = ui.cursor;
    let zoom = ui.zoom;
    let idle = ctx.memory(|m| m.focused().is_none());

    ctx.input(|input| {
        let cmd = input.modifiers.command;
        let shift = input.modifiers.shift;
        if cmd && input.key_pressed(egui::Key::S) { actions.push(if shift { UiAction::SaveAs } else { UiAction::Save }); }
        if cmd && input.key_pressed(egui::Key::O) { actions.push(UiAction::Open); }
        if cmd && input.key_pressed(egui::Key::N) { actions.push(UiAction::New); }
        if cmd && input.key_pressed(egui::Key::Z) { actions.push(if shift { UiAction::Redo } else { UiAction::Undo }); }
        if cmd && input.key_pressed(egui::Key::Y) { actions.push(UiAction::Redo); }
        if cmd && input.key_pressed(egui::Key::Plus) { actions.push(UiAction::SetZoom(zoom + 0.1)); }
        if cmd && input.key_pressed(egui::Key::Minus) { actions.push(UiAction::SetZoom(zoom - 0.1)); }
        if cmd && input.key_pressed(egui::Key::Num0) { actions.push(UiAction::ResetView); }
        if input.key_pressed(egui::Key::F5) { actions.push(UiAction::Recalculate); }
        if input.key_pressed(egui::Key::Escape) {
            match editing { Some(id) => actions.push(UiAction::Edit(id, false)), None => actions.push(UiAction::Cancel) }
        }
        if idle && editing.is_none() {
            if input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace) {
                if let Some(id) = ui.selected { actions.push(UiAction::Delete(id)); }
            }
            if let Some(c) = cursor {
                let step = crate::canvas::GRID;
                let mut moved = None;
                if input.key_pressed(egui::Key::ArrowLeft) { moved = Some(egui::Vec2::new(c.x - step, c.y)); }
                if input.key_pressed(egui::Key::ArrowRight) { moved = Some(egui::Vec2::new(c.x + step, c.y)); }
                if input.key_pressed(egui::Key::ArrowUp) { moved = Some(egui::Vec2::new(c.x, c.y - step)); }
                if input.key_pressed(egui::Key::ArrowDown) { moved = Some(egui::Vec2::new(c.x, c.y + step)); }
                if let Some(m) = moved { ui.cursor = Some(m); }
                if !cmd && !input.modifiers.alt {
                    let mut text = String::new();
                    for ev in &input.events {
                        if let egui::Event::Text(s) = ev {
                            for ch in s.chars() { if !ch.is_control() { text.push(ch); } }
                        }
                    }
                    if !text.is_empty() { actions.push(UiAction::InsertSnippetAt(text, c.x, c.y)); }
                    else if input.key_pressed(egui::Key::Enter) { actions.push(UiAction::InsertSnippetAt(String::new(), c.x, c.y)); }
                }
            }
        }
    });
    panels::apply_actions(ctx, session, ui, actions);
}
