//! The worksheet canvas: blueprint grid, anchored region cards, geometry-based
//! hit-testing (never egui hover senses), and an explicit pointer state machine.
use crate::state::{Drag, UiAction, UiState, View3d};
use crate::theme::{self, Icon, Theme};
use crate::{editor, math_render, plots};
use aaiyyan_ast::{Expr, Statement};
use aaiyyan_core::Session;
use aaiyyan_worksheet::{Block, BlockKind, ControlKind};
use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use uuid::Uuid;

pub const GRID: f32 = 12.0;
const REGION_W: f32 = 440.0;
const DRAG_THRESHOLD: f32 = 4.0;
const CARD_PAD: f32 = 7.0;

pub fn snap(v: f32, on: bool) -> f32 { if on { (v / GRID).round() * GRID } else { v } }

struct Hit { id: Uuid, rect: Rect, grip: Option<Rect>, tools: Option<Rect>, editable: bool }

fn is_editable(kind: &BlockKind) -> bool {
    matches!(kind, BlockKind::Equation { .. } | BlockKind::Result { .. } | BlockKind::Text { .. } | BlockKind::Heading { .. })
}

fn draw_grid(painter: &egui::Painter, view: Rect, pan: Vec2, zoom: f32, t: &Theme) {
    let step = GRID * zoom;
    if step < 3.0 { return; }
    let origin = view.min - pan;
    let fx = ((view.left() - origin.x) / step).floor();
    let lx = ((view.right() - origin.x) / step).ceil();
    let fy = ((view.top() - origin.y) / step).floor();
    let ly = ((view.bottom() - origin.y) / step).ceil();
    let mut i = fx;
    while i <= lx {
        let x = origin.x + i * step;
        let major = (i.round() as i64) % 5 == 0;
        painter.line_segment([Pos2::new(x, view.top()), Pos2::new(x, view.bottom())],
            Stroke::new(if major { 1.0_f32 } else { 0.7_f32 }, if major { t.grid_major } else { t.grid }));
        i += 1.0;
    }
    let mut j = fy;
    while j <= ly {
        let y = origin.y + j * step;
        let major = (j.round() as i64) % 5 == 0;
        painter.line_segment([Pos2::new(view.left(), y), Pos2::new(view.right(), y)],
            Stroke::new(if major { 1.0_f32 } else { 0.7_f32 }, if major { t.grid_major } else { t.grid }));
        j += 1.0;
    }
    if view.contains(origin) {
        let st = Stroke::new(1.2_f32, t.accent_line);
        painter.line_segment([origin - Vec2::new(9.0, 0.0), origin + Vec2::new(9.0, 0.0)], st);
        painter.line_segment([origin - Vec2::new(0.0, 9.0), origin + Vec2::new(0.0, 9.0)], st);
        painter.circle_stroke(origin, 3.0_f32, st);
        painter.text(origin + Vec2::new(11.0, 9.0), egui::Align2::LEFT_CENTER, "0, 0",
            egui::FontId::proportional(9.5), t.ink_faint);
    }
}

pub fn show(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let t = theme::current(ctx);
    let ws = session.worksheet().clone();

    egui::CentralPanel::default().frame(egui::Frame::none().fill(t.page)).show(ctx, |ui| {
        let view = ui.max_rect();
        let zoom = state.zoom;
        let origin = view.min;

        if state.show_grid { draw_grid(&ui.painter_at(view), view, state.pan, zoom, &t); }

        if let Some(p) = ctx.pointer_interact_pos() {
            if view.contains(p) {
                let over_palette = state.palette_rect.map(|r| r.expand(4.0).contains(p)).unwrap_or(false);
                if !over_palette {
                    let (ctrl, scroll) = ctx.input(|i| (i.modifiers.ctrl, i.raw_scroll_delta));
                    if ctrl && scroll.y.abs() > 0.0 {
                        let world = (p - origin + state.pan) / zoom;
                        let nz = (zoom * (1.0 + scroll.y * 0.0015)).clamp(0.35, 3.0);
                        state.pan = world * nz - (p - origin);
                        state.zoom = nz;
                    } else if scroll != Vec2::ZERO {
                        state.pan -= scroll;
                    }
                }
            }
        }

        let pointer = ctx.pointer_interact_pos();
        state.hovered = pointer.and_then(|p| {
            ws.blocks.iter().rev().find_map(|b| {
                let size = state.sizes.get(&b.id).copied().unwrap_or(Vec2::new(REGION_W, 26.0)) * zoom;
                let pos = origin + Vec2::new(b.x, b.y) * zoom - state.pan;
                if Rect::from_min_size(pos, size).expand(CARD_PAD + 4.0).contains(p) { Some(b.id) } else { None }
            })
        });

        if ws.blocks.is_empty() { empty_state(ui, view, &t); }

        let mut hits: Vec<Hit> = Vec::new();
        for block in &ws.blocks {
            if let Some(h) = region(ui, ctx, session, state, block, actions, &t, origin, zoom) { hits.push(h); }
        }

        interact(ctx, state, actions, &hits, view, origin, zoom);
        splits(ui, ctx, state, view, &t,
            session.project.layout.show_left,
            session.project.layout.show_right,
            session.project.layout.show_bottom);

        if let Some(c) = state.cursor {
            if state.editing.is_none() {
                let p = origin + c * zoom - state.pan;
                if view.contains(p) {
                    let painter = ui.painter_at(view);
                    let pulse = ctx.animate_bool_with_time(egui::Id::new("crosshair"), true, 0.2);
                    let s = 7.0_f32 + pulse * 0.6;
                    let st = Stroke::new(1.7_f32, t.accent);
                    painter.line_segment([p - Vec2::new(s, 0.0), p + Vec2::new(s, 0.0)], st);
                    painter.line_segment([p - Vec2::new(0.0, s), p + Vec2::new(0.0, s)], st);
                    painter.circle_stroke(p, 2.0_f32, Stroke::new(1.0_f32, t.accent_line));
                }
            }
        }

        // Canvas HUD
        let hud = Rect::from_min_size(view.left_top() + Vec2::new(10.0, 8.0), Vec2::new(250.0, 22.0));
        let hp = ui.painter_at(hud);
        hp.rect_filled(hud, 11.0, t.page.linear_multiply(0.86));
        hp.rect_stroke(hud, 11.0, Stroke::new(1.0_f32, t.hairline));
        hp.text(hud.left_center() + Vec2::new(10.0, 0.0), egui::Align2::LEFT_CENTER,
            format!("{}   ·   {} regions   ·   {:.0}%", ws.name, ws.blocks.len(), zoom * 100.0),
            egui::FontId::proportional(10.5), t.ink_faint);

        ui.painter().text(view.right_bottom() - Vec2::new(12.0, 9.0), egui::Align2::RIGHT_BOTTOM,
            "click region: edit  ·  click grid: insert  ·  grip / alt-drag: move  ·  ctrl+wheel: zoom",
            egui::FontId::proportional(10.0), t.ink_faint);
    });
}

fn empty_state(ui: &mut Ui, view: Rect, t: &Theme) {
    let w = 380.0;
    let r = Rect::from_center_size(view.center() - Vec2::new(0.0, 30.0), Vec2::new(w, 132.0));
    let p = ui.painter_at(r);
    p.rect_filled(r, 8.0, t.card);
    p.rect_stroke(r, 8.0, Stroke::new(1.0_f32, t.card_edge));
    theme::paint_icon(&p, Rect::from_center_size(Pos2::new(r.center().x, r.top() + 34.0), Vec2::splat(26.0)), Icon::Equation, t.accent);
    p.text(Pos2::new(r.center().x, r.top() + 66.0), egui::Align2::CENTER_CENTER,
        "Click anywhere on the grid and start typing", egui::FontId::proportional(14.0), t.ink);
    p.text(Pos2::new(r.center().x, r.top() + 90.0), egui::Align2::CENTER_CENTER,
        "2+3=        E := 210 GPa        delta =", theme::math(13.0), t.ink_faint);
    p.text(Pos2::new(r.center().x, r.bottom() - 16.0), egui::Align2::CENTER_CENTER,
        "trailing = evaluates  ·  := defines", egui::FontId::proportional(10.5), t.ink_faint);
}

fn interact(ctx: &egui::Context, state: &mut UiState, actions: &mut Vec<UiAction>, hits: &[Hit], view: Rect, origin: Pos2, zoom: f32) {
    let Some(p) = ctx.pointer_interact_pos() else { return };
    let over_palette = state.palette_rect.map(|r| r.expand(4.0).contains(p)).unwrap_or(false);
    let near_split = (p.x - view.left()) < 8.0
        || (view.right() - p.x) < 8.0
        || (view.bottom() - p.y) < 8.0;
    if !view.contains(p) || over_palette || near_split || ctx.is_using_pointer() {
        if ctx.input(|i| i.pointer.primary_released()) { state.drag = None; state.press = None; state.moved = false; }
        return;
    }
    let (pressed, released, down, middle, alt, delta) = ctx.input(|i| (
        i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.primary_down(),
        i.pointer.button_down(egui::PointerButton::Middle), i.modifiers.alt, i.pointer.delta(),
    ));
    let top = |pred: &dyn Fn(&Hit) -> bool| hits.iter().rev().find(|h| pred(h)).map(|h| h.id);
    let hit_region = top(&|h| h.rect.contains(p));
    let hit_grip = top(&|h| h.grip.map(|g| g.contains(p)).unwrap_or(false));
    let hit_tools = top(&|h| h.tools.map(|g| g.contains(p)).unwrap_or(false));

    if middle && down { state.pan -= delta; state.drag = None; state.press = None; return; }

    if pressed {
        state.press = Some(p);
        state.moved = false;
        if let Some(id) = hit_grip { state.drag = Some(Drag::Move(id)); }
        else if alt { if let Some(id) = hit_region { state.drag = Some(Drag::Move(id)); } }
        else if hit_region.is_none() && hit_tools.is_none() { state.drag = Some(Drag::Pan); }
        else { state.drag = None; }
    }
    if down {
        if let Some(press) = state.press { if p.distance(press) > DRAG_THRESHOLD { state.moved = true; } }
        if state.moved {
            match state.drag {
                Some(Drag::Pan) => state.pan -= delta,
                Some(Drag::Move(id)) => { let d = delta / zoom; actions.push(UiAction::SetBlockPosDelta(id, d.x, d.y)); }
                None => {}
            }
        }
    }
    if released {
        if !state.moved {
            if hit_tools.is_some() {
            } else if let Some(id) = hit_grip {
                actions.push(UiAction::Select(id));
            } else if let Some(id) = hit_region {
                if state.editing != Some(id) {
                    actions.push(UiAction::Select(id));
                    if hits.iter().find(|h| h.id == id).map(|h| h.editable).unwrap_or(false) {
                        actions.push(UiAction::Edit(id, true));
                    }
                }
            } else {
                let w = (p - origin + state.pan) / zoom;
                state.cursor = Some(Vec2::new(snap(w.x, state.snap), snap(w.y, state.snap)));
                state.editing = None;
                state.selected = None;
            }
        }
        state.drag = None; state.press = None; state.moved = false;
    }
}

fn is_literal(e: &Expr) -> bool { matches!(e, Expr::Number(_) | Expr::Str(_) | Expr::Bool(_)) }

#[allow(clippy::too_many_arguments)]
fn region(
    ui: &mut Ui, ctx: &egui::Context, session: &mut Session, state: &mut UiState,
    block: &Block, actions: &mut Vec<UiAction>, t: &Theme, origin: Pos2, zoom: f32,
) -> Option<Hit> {
    let selected = state.selected == Some(block.id);
    let editing = state.editing == Some(block.id);
    let hovered = state.hovered == Some(block.id);
    let pos = origin + Vec2::new(block.x, block.y) * zoom - state.pan;
    let width = REGION_W * zoom;
    let max_rect = Rect::from_min_size(pos, Vec2::new(width, 4000.0));
    if !max_rect.intersects(ui.clip_rect()) { return None; }

    // Anchored card, painted beneath the content using last frame's measured size.
    let prev = state.sizes.get(&block.id).copied().unwrap_or(Vec2::new(width / zoom, 26.0)) * zoom;
    let card_rect = Rect::from_min_size(pos, prev).expand(CARD_PAD);
    let anim = ctx.animate_bool_with_time(egui::Id::new(("card", block.id)), hovered || selected || editing, 0.12);
    if anim > 0.001 && anim < 0.999 { ctx.request_repaint(); }
    {
        let p = ui.painter();
        let base = t.card;
        p.rect_filled(card_rect, 4.0, base);
        let edge = if editing { t.accent } else if selected { t.accent_line } else { t.card_edge };
        p.rect_stroke(card_rect, 4.0, Stroke::new(if editing { 1.4_f32 } else { 1.0_f32 }, edge));
        if anim > 0.01 {
            p.rect_stroke(card_rect.expand(1.5), 5.0, Stroke::new(1.0_f32, t.glow().linear_multiply(anim)));
        }
        if selected || editing {
            let bar = Rect::from_min_size(Pos2::new(card_rect.left() - 1.0, card_rect.top() + 2.0), Vec2::new(2.5, card_rect.height() - 4.0));
            p.rect_filled(bar, 1.5, t.accent);
        }
        if !block.enabled { p.rect_filled(card_rect, 4.0, t.page.linear_multiply(0.55)); }
    }

    let math_size = 18.5 * zoom;
    let inner = ui.allocate_ui_at_rect(max_rect, |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 3.0 * zoom;
        match &block.kind {
            BlockKind::Equation { source } | BlockKind::Result { source } => {
                if editing {
                    let mut text = source.replace('\n', "; ");
                    let resp = ui.add(egui::TextEdit::singleline(&mut text)
                        .id(egui::Id::new(("eq", block.id)))
                        .font(theme::math(math_size))
                        .desired_width(width - 16.0)
                        .margin(4.0)
                        .hint_text("2+3=   ·   delta := 5*w*L^4/(384*E*I)"));
                    if state.focus_request == Some(block.id) { resp.request_focus(); }
                    if resp.changed() { actions.push(UiAction::UpdateSource(block.id, text.clone())); }
                    if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        actions.push(UiAction::UpdateSource(block.id, text));
                        actions.push(UiAction::Edit(block.id, false));
                    }
                } else {
                    let stmts = session.parsed_block(block.id);
                    let values = session.values_for(block.id).to_vec();
                    let result_block = matches!(block.kind, BlockKind::Result { .. });
                    if stmts.is_empty() {
                        ui.label(egui::RichText::new(source).font(theme::math(math_size)).color(t.ink_faint));
                    }
                    for stmt in &stmts {
                        let value = match stmt {
                            Statement::Assign { name, .. } | Statement::FuncDef { name, .. } =>
                                values.iter().find(|(n, _)| n == name).map(|(_, v)| v),
                            Statement::Expr(_) => values.iter().find(|(n, _)| n == "_").map(|(_, v)| v),
                        };
                        let echo = match stmt {
                            Statement::FuncDef { .. } => false,
                            Statement::Expr(_) => true,
                            Statement::Assign { .. } => result_block || !is_literal(stmt.value_expr()),
                        };
                        math_render::show(ui, &math_render::definition_node(stmt, value, echo), math_size);
                    }
                }
                for err in session.errors_for(block.id) {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Info, 12.0, t.danger);
                        ui.label(egui::RichText::new(err).size(11.0 * zoom).color(t.danger));
                    });
                    if err.contains("undefined symbol") {
                        let name = err.split("'").nth(1).unwrap_or("x");
                        ui.label(egui::RichText::new(format!("define it first:  {name} := 1.5")).size(10.0 * zoom).color(t.ink_faint));
                    }
                }
            }
            BlockKind::Heading { level, text } => {
                if editing {
                    let mut s = text.clone();
                    let r = ui.add(egui::TextEdit::singleline(&mut s).id(egui::Id::new(("h", block.id)))
                        .font(egui::FontId::proportional(19.0 * zoom)).desired_width(width - 16.0));
                    if state.focus_request == Some(block.id) { r.request_focus(); }
                    if r.changed() { actions.push(UiAction::UpdateSource(block.id, s)); }
                } else {
                    let size = match level { 1 => 21.0, 2 => 17.5, _ => 15.0 } * zoom;
                    ui.label(egui::RichText::new(text).size(size).strong().color(t.ink));
                    if *level == 1 {
                        let r = ui.available_rect_before_wrap();
                        ui.painter().line_segment([Pos2::new(r.left(), r.top()), Pos2::new(r.left() + 120.0, r.top())],
                            Stroke::new(2.0_f32, t.accent));
                        ui.add_space(4.0);
                    }
                }
            }
            BlockKind::Text { text } => {
                if editing {
                    let mut s = text.clone();
                    let r = ui.add(egui::TextEdit::multiline(&mut s).id(egui::Id::new(("t", block.id)))
                        .desired_rows(3).desired_width(width - 16.0));
                    if state.focus_request == Some(block.id) { r.request_focus(); }
                    if r.changed() { actions.push(UiAction::UpdateSource(block.id, s)); }
                } else {
                    ui.label(egui::RichText::new(text).size(13.0 * zoom).color(t.ink_soft));
                }
            }
            BlockKind::Python { source, trusted } => {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Python, 14.0, t.accent);
                    ui.label(egui::RichText::new("Python").size(10.5).strong().color(t.ink_soft));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Run").clicked() { actions.push(UiAction::RunPython(block.id)); }
                        let mut tr = *trusted;
                        if ui.checkbox(&mut tr, "trusted").changed() { actions.push(UiAction::SetTrust(block.id, tr)); }
                    });
                });
                let mut code = source.clone();
                let r = editor::code_editor(ui, egui::Id::new(("py", block.id)), &mut code, 6, false);
                if r.changed() { actions.push(UiAction::UpdateSource(block.id, code)); }
            }
            BlockKind::Plot2D(spec) => match session.sample_plot2d(spec) {
                Ok(series) => plots::show_2d(ui, egui::Id::new(("p2", block.id)), spec, &series),
                Err(e) => { ui.colored_label(t.danger, e); }
            },
            BlockKind::Plot3D(spec) => {
                let view = state.views3d.entry(block.id).or_insert_with(View3d::new);
                match session.sample_plot3d(spec) {
                    Ok(surface) => plots::show_3d(ui, &surface, view),
                    Err(e) => { ui.colored_label(t.danger, e); }
                }
            }
            BlockKind::Table { columns, rows } => {
                egui::Grid::new(("tbl", block.id)).striped(true).min_col_width(62.0).show(ui, |ui| {
                    for c in columns { ui.label(egui::RichText::new(c).strong().size(11.5).color(t.ink)); }
                    ui.end_row();
                    for r in rows.iter().take(40) {
                        for c in r { ui.label(egui::RichText::new(c).size(11.5).color(t.ink_soft)); }
                        ui.end_row();
                    }
                });
            }
            BlockKind::Control(spec) => {
                let mut kind = spec.kind.clone();
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&spec.label).font(theme::math_italic(14.0 * zoom)).color(t.ink));
                    match &mut kind {
                        ControlKind::Slider { min, max, value, .. } => {
                            changed |= ui.add(egui::Slider::new(value, *min..=*max).show_value(false).text("")).changed();
                            let mut v = *value;
                            if ui.add(egui::DragValue::new(&mut v).speed(0.1)).changed() { *value = v; changed = true; }
                        }
                        ControlKind::Number { value } => { changed |= ui.add(egui::DragValue::new(value).speed(0.1)).changed(); }
                        ControlKind::Checkbox { value } => { changed |= ui.checkbox(value, "").changed(); }
                        ControlKind::Dropdown { options, selected } => {
                            let cur = options.get(*selected).cloned().unwrap_or_default();
                            egui::ComboBox::from_id_source(("dd", block.id)).selected_text(cur).show_ui(ui, |ui| {
                                for (i, o) in options.iter().enumerate() {
                                    if ui.selectable_label(*selected == i, o).clicked() { *selected = i; changed = true; }
                                }
                            });
                        }
                    }
                });
                if changed { actions.push(UiAction::UpdateControl(block.id, kind)); }
            }
            BlockKind::Image { path, caption } => {
                ui.label(egui::RichText::new(format!("{path}  {caption}")).size(11.5).color(t.ink_soft));
            }
        }
        ui.min_rect()
    });

    let used = inner.inner;
    state.sizes.insert(block.id, used.size() / zoom);
    state.focus_request = state.focus_request.filter(|id| *id != block.id);

    let box_rect = used.expand(CARD_PAD);
    let mut grip = None;
    let mut tools = None;

    if selected || editing || hovered {
        let g = Rect::from_min_size(Pos2::new(box_rect.left() - 15.0, box_rect.top()), Vec2::new(12.0, box_rect.height().max(18.0)));
        let gp = ui.painter_at(g);
        gp.rect_filled(g, 3.0, if hovered { t.accent_soft } else { Color32::TRANSPARENT });
        for i in 0..4 { for j in 0..2 {
            gp.circle_filled(Pos2::new(g.left() + 3.5 + j as f32 * 4.0, g.top() + 4.0 + i as f32 * 4.0), 1.0_f32, t.ink_faint);
        } }
        grip = Some(g);

        if selected || editing {
            let tr = Rect::from_min_size(Pos2::new(box_rect.right() + 5.0, box_rect.top()), Vec2::new(28.0, 82.0));
            let tp = ui.painter_at(tr);
            tp.rect_filled(tr, 4.0, t.card);
            tp.rect_stroke(tr, 4.0, Stroke::new(1.0_f32, t.card_edge));
            ui.allocate_ui_at_rect(tr.shrink(1.0), |tu| {
                tu.spacing_mut().item_spacing.y = 1.0;
                if theme::ribbon_button(tu, if editing { Icon::Check } else { Icon::Pencil }, "", false, true, t)
                    .on_hover_text(if editing { "Finish editing (Esc)" } else { "Edit region" }).clicked()
                { actions.push(UiAction::Edit(block.id, !editing)); }
                if theme::ribbon_button(tu, Icon::Trash, "", false, true, t).on_hover_text("Delete region").clicked() {
                    actions.push(UiAction::Delete(block.id));
                }
                let mut en = block.enabled;
                if tu.checkbox(&mut en, "").on_hover_text("Include in calculation").changed() {
                    actions.push(UiAction::ToggleEnabled(block.id));
                }
            });
            tools = Some(tr);
        }
    }

    if !block.enabled {
        ui.painter().text(box_rect.right_top() + Vec2::new(6.0, -1.0), egui::Align2::LEFT_BOTTOM,
            "disabled", egui::FontId::proportional(9.5), t.warn);
    }

    Some(Hit { id: block.id, rect: box_rect, grip, tools, editable: is_editable(&block.kind) })
}


// ---------------------------------------------------------------------------
// Panel splits. Sizes live in UiState and the handles are drawn at the canvas
// edges, so resizing never depends on egui panel state or fights the canvas pan.
// ---------------------------------------------------------------------------
fn split_band(
    ui: &mut Ui,
    ctx: &egui::Context,
    rect: Rect,
    id: egui::Id,
    vertical: bool,
    tip: &str,
    t: &Theme,
) -> (Vec2, bool) {
    let resp = ui.interact(rect, id, egui::Sense::click_and_drag()).on_hover_text(tip);
    let active = resp.hovered() || resp.dragged();
    let painter = ui.painter_at(rect);
    if active {
        painter.rect_filled(rect, 0.0, t.accent_soft);
        ctx.set_cursor_icon(if vertical {
            egui::CursorIcon::ResizeVertical
        } else {
            egui::CursorIcon::ResizeHorizontal
        });
    }
    let line_col = if active { t.accent_line } else { t.ribbon_edge };
    let grip_col = if active { t.accent } else { t.ink_faint };
    let c = rect.center();
    if vertical {
        painter.line_segment(
            [Pos2::new(rect.left(), rect.bottom() - 0.5), Pos2::new(rect.right(), rect.bottom() - 0.5)],
            Stroke::new(1.0_f32, line_col),
        );
        for k in -1..=1 {
            painter.circle_filled(Pos2::new(c.x + k as f32 * 5.0, c.y), 1.2_f32, grip_col);
        }
    } else {
        painter.line_segment(
            [Pos2::new(rect.center().x, rect.top()), Pos2::new(rect.center().x, rect.bottom())],
            Stroke::new(1.0_f32, line_col),
        );
        for k in -1..=1 {
            painter.circle_filled(Pos2::new(c.x, c.y + k as f32 * 5.0), 1.2_f32, grip_col);
        }
    }
    (resp.drag_delta(), resp.double_clicked())
}

#[allow(clippy::too_many_arguments)]
fn splits(
    ui: &mut Ui,
    ctx: &egui::Context,
    state: &mut UiState,
    view: Rect,
    t: &Theme,
    show_left: bool,
    show_right: bool,
    show_bottom: bool,
) {
    let band = 7.0_f32;

    if show_bottom {
        let r = Rect::from_min_max(Pos2::new(view.left(), view.bottom() - band), view.right_bottom());
        let tip = format!("Output dock  {:.0} px\ndrag to resize · double-click to toggle", state.dock_height);
        let (d, dbl) = split_band(ui, ctx, r, egui::Id::new("split.bottom"), true, &tip, t);
        if d != Vec2::ZERO {
            state.dock_height = (state.dock_height - d.y).clamp(110.0, 640.0);
        }
        if dbl {
            state.dock_height = if (state.dock_height - 186.0).abs() < 2.0 { 340.0 } else { 186.0 };
        }
    }

    if show_left {
        let r = Rect::from_min_max(view.left_top(), Pos2::new(view.left() + band, view.bottom()));
        let tip = format!("Navigation  {:.0} px\ndrag to resize · double-click to reset", state.left_width);
        let (d, dbl) = split_band(ui, ctx, r, egui::Id::new("split.left"), false, &tip, t);
        if d != Vec2::ZERO {
            state.left_width = (state.left_width + d.x).clamp(180.0, 440.0);
        }
        if dbl { state.left_width = 236.0; }
    }

    if show_right {
        let r = Rect::from_min_max(Pos2::new(view.right() - band, view.top()), view.right_bottom());
        let tip = format!("Inspector  {:.0} px\ndrag to resize · double-click to reset", state.right_width);
        let (d, dbl) = split_band(ui, ctx, r, egui::Id::new("split.right"), false, &tip, t);
        if d != Vec2::ZERO {
            state.right_width = (state.right_width - d.x).clamp(240.0, 540.0);
        }
        if dbl { state.right_width = 292.0; }
    }
}
