use crate::editor;
use crate::state::{UiAction, UiState, PALETTES, RIBBON_TABS};
use crate::theme::{self, Icon, Theme};
use aaiyyan_core::Session;
use aaiyyan_plotting::{Plot2DSpec, Plot3DSpec, SeriesKind, SeriesSpec, SurfaceKind};
use aaiyyan_project::{Backend, CalcMode};
use aaiyyan_worksheet::{BlockKind, ControlSpec};
use egui::{Color32, Pos2, Stroke, Ui, Vec2};
use std::path::PathBuf;

fn plot2d(expr: &str, kind: SeriesKind, title: &str) -> Plot2DSpec {
    Plot2DSpec {
        title: title.into(), x_label: "x".into(), y_label: "f(x)".into(),
        x_min: if kind == SeriesKind::Polar { 0.0 } else { -6.0 },
        x_max: if kind == SeriesKind::Polar { 6.283185307 } else { 6.0 },
        samples: 400, show_legend: true,
        series: vec![SeriesSpec { expr: expr.into(), variable: "x".into(), label: expr.into(), kind, color: None }],
    }
}
fn plot3d(kind: SurfaceKind, expr: &str, title: &str) -> Plot3DSpec {
    Plot3DSpec { title: title.into(), expr: expr.into(), kind, ..Default::default() }
}
fn drop_point(state: &UiState) -> (f32, f32) {
    match state.cursor { Some(c) => (c.x, c.y), None => (state.pan.x + 120.0, state.pan.y + 96.0) }
}
fn insert_kind(state: &UiState, kind: BlockKind) -> UiAction {
    let (x, y) = drop_point(state);
    UiAction::InsertBlockAt(kind, x, y)
}

// ---------------------------------------------------------------- ribbon ---
pub fn ribbon(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let t = theme::current(ctx);
    state.palette_buttons.clear();
    egui::TopBottomPanel::top("ribbon")
        .frame(egui::Frame::none().fill(t.ribbon))
        .show(ctx, |ui| {
            // Title / quick access
            egui::Frame::none().fill(t.ribbon).inner_margin(egui::Margin::symmetric(10.0, 5.0)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let logo = egui::Rect::from_min_size(ui.cursor().left_top() + Vec2::new(0.0, 2.0), Vec2::splat(17.0));
                    theme::paint_icon(&ui.painter(), logo, Icon::Sigma, t.accent);
                    ui.add_space(21.0);
                    ui.label(egui::RichText::new("AaiyyanMath").size(12.5).strong().color(t.ink));
                    ui.label(egui::RichText::new("·").size(12.0).color(t.ink_faint));
                    ui.label(egui::RichText::new(&session.project.metadata.name).size(12.0).color(t.ink_soft));
                    ui.add_space(12.0);
                    for (ic, tip, a) in [(Icon::New, "New project", UiAction::New), (Icon::Open, "Open", UiAction::Open), (Icon::Save, "Save", UiAction::Save)] {
                        if theme::ribbon_button(ui, ic, "", false, true, &t).on_hover_text(tip).clicked() { actions.push(a); }
                    }
                    ui.add_space(4.0);
                    if theme::ribbon_button(ui, Icon::Undo, "", false, session.history.can_undo(), &t).on_hover_text("Undo (Ctrl+Z)").clicked() { actions.push(UiAction::Undo); }
                    if theme::ribbon_button(ui, Icon::Redo, "", false, session.history.can_redo(), &t).on_hover_text("Redo (Ctrl+Shift+Z)").clicked() { actions.push(UiAction::Redo); }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let calc = egui::Button::new(egui::RichText::new("Calculate   F5").size(12.0).strong().color(Color32::WHITE))
                            .fill(if session.busy { t.ink_faint } else { t.ok })
                            .rounding(4.0).min_size(Vec2::new(112.0, 26.0));
                        if ui.add(calc).clicked() { actions.push(UiAction::Recalculate); }
                        if theme::ribbon_button(ui, Icon::Stop, "Cancel", false, session.busy, &t).clicked() { actions.push(UiAction::Cancel); }
                        ui.add_space(6.0);
                        theme::chip(ui, if session.project.settings.mode == CalcMode::Auto { "Automatic" } else { "Manual" }, t.accent, &t);
                        theme::chip(ui, match session.project.settings.backend {
                            Backend::Native => "Native", Backend::Python => "Python", Backend::Auto => "Auto backend",
                        }, t.ink_soft, &t);
                        if session.busy { ui.spinner(); }
                    });
                });
            });

            // Tab strip
            egui::Frame::none().fill(t.chrome_alt).inner_margin(egui::Margin::symmetric(8.0, 0.0)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (i, name) in RIBBON_TABS.iter().enumerate() {
                        let active = state.ribbon == i;
                        let font = egui::FontId::proportional(12.0);
                        let w = ui.fonts(|f| f.layout_no_wrap(name.to_string(), font.clone(), t.ink).size().x) + 26.0;
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 27.0), egui::Sense::click());
                        let anim = ctx.animate_bool_with_time(resp.id.with("tab"), active || resp.hovered(), 0.12);
                        if anim > 0.001 && anim < 0.999 { ctx.request_repaint(); }
                        let painter = ui.painter_at(rect);
                        if active {
                            painter.rect_filled(rect, 3.0, t.page);
                            painter.line_segment([Pos2::new(rect.left() + 8.0, rect.bottom() - 1.0), Pos2::new(rect.right() - 8.0, rect.bottom() - 1.0)],
                                Stroke::new(2.0_f32, t.accent));
                        } else if anim > 0.01 {
                            painter.rect_filled(rect, 3.0, t.faint_hover().linear_multiply(anim));
                        }
                        painter.text(rect.center(), egui::Align2::CENTER_CENTER, *name, font,
                            if active { t.accent } else { t.ink_soft });
                        if resp.clicked() { state.ribbon = i; state.palette = None; }
                    }
                });
            });
            let r = ui.min_rect();
            ui.painter().line_segment([Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.bottom())],
                Stroke::new(1.0_f32, t.ribbon_edge));

            // Ribbon body — no scroll area, so no stray scrollbar.
            egui::Frame::none().fill(t.page).inner_margin(egui::Margin::symmetric(12.0, 7.0)).show(ui, |ui| {
                ui.set_min_height(70.0);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    match state.ribbon {
                        0 => math_tab(ui, state, actions, &t),
                        1 => plots_tab(ui, state, actions, &t),
                        2 => matrices_tab(ui, state, actions, &t),
                        3 => functions_tab(ui, state, actions, &t),
                        4 => calculation_tab(ui, session, actions, &t),
                        5 => document_tab(ui, actions, &t),
                        _ => view_tab(ui, session, state, actions, &t),
                    }
                });
            });
            let r = ui.min_rect();
            ui.painter().line_segment([Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.bottom())],
                Stroke::new(1.0_f32, t.ribbon_edge));
        });
}

fn palette_button(ui: &mut Ui, state: &mut UiState, index: usize, icon: Icon, t: &Theme) {
    let label = format!("{} \u{25BE}", PALETTES[index]);
    let resp = theme::ribbon_button(ui, icon, &label, true, true, t);
    state.palette_buttons.push((index, resp.rect));
    if resp.clicked() {
        state.palette = if state.palette == Some(index) { None } else { Some(index) };
        state.palette_anchor = crate::palette::anchor_for(ui, &resp);
    }
}

fn math_tab(ui: &mut Ui, state: &mut UiState, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "Palettes", t, |ui| {
        palette_button(ui, state, 0, Icon::Equation, t);
        palette_button(ui, state, 1, Icon::Pi, t);
        palette_button(ui, state, 2, Icon::Sigma, t);
        palette_button(ui, state, 3, Icon::Gear, t);
        palette_button(ui, state, 4, Icon::Doc, t);
        palette_button(ui, state, 5, Icon::Sine, t);
    });
    theme::ribbon_group(ui, "Regions", t, |ui| {
        if theme::ribbon_button(ui, Icon::Equation, "Equation", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Equation { source: String::new() }));
        }
        if theme::ribbon_button(ui, Icon::Text, "Text", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Text { text: "Engineering note.".into() }));
        }
        if theme::ribbon_button(ui, Icon::Heading, "Heading", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Heading { level: 1, text: "Section".into() }));
        }
        if theme::ribbon_button(ui, Icon::Slider, "Slider", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Control(ControlSpec::slider("a", 1.0, 100.0))));
        }
    });
    theme::ribbon_group(ui, "Quick insert", t, |ui| {
        for (ic, label, snip) in [
            (Icon::Root, "Root", "sqrt("), (Icon::Integral, "Integral", "integrate(f, 0, 1)"),
            (Icon::Matrix, "Matrix", "[[1, 2], [3, 4]]"), (Icon::Sigma, "Sum", "sum(v)"),
        ] {
            if theme::ribbon_button(ui, ic, label, true, true, t).clicked() { actions.push(UiAction::InsertSnippet(snip.into())); }
        }
    });
}

fn plots_tab(ui: &mut Ui, state: &mut UiState, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "2D plots", t, |ui| {
        for (label, expr, kind) in [
            ("Line", "sin(x)", SeriesKind::Line), ("Scatter", "sin(x)/x", SeriesKind::Scatter),
            ("Polar", "1 + cos(x)", SeriesKind::Polar), ("Parametric", "[cos(x), sin(x)]", SeriesKind::Parametric),
            ("Histogram", "rnorm(200, 0, 1)", SeriesKind::Histogram),
        ] {
            if theme::ribbon_button(ui, Icon::Plot2D, label, true, true, t).clicked() {
                actions.push(insert_kind(state, BlockKind::Plot2D(plot2d(expr, kind, label))));
            }
        }
    });
    theme::ribbon_group(ui, "3D plots", t, |ui| {
        for (label, expr, kind) in [
            ("Surface", "sin(x)*cos(y)", SurfaceKind::Surface),
            ("Scatter", "sin(x*y)", SurfaceKind::Scatter),
            ("Field", "[-y, x]", SurfaceKind::VectorField),
        ] {
            if theme::ribbon_button(ui, Icon::Plot3D, label, true, true, t).clicked() {
                actions.push(insert_kind(state, BlockKind::Plot3D(plot3d(kind, expr, label))));
            }
        }
    });
    theme::ribbon_group(ui, "Data", t, |ui| {
        if theme::ribbon_button(ui, Icon::Table, "Table", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Table {
                columns: vec!["x".into(), "y".into()],
                rows: vec![vec!["0".into(), "0".into()], vec!["1".into(), "1".into()]],
            }));
        }
        if theme::ribbon_button(ui, Icon::Open, "Read CSV", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Equation { source: "data := read_csv(\"data.csv\")".into() }));
        }
    });
}

fn matrices_tab(ui: &mut Ui, state: &mut UiState, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "Insert", t, |ui| {
        if theme::ribbon_button(ui, Icon::Matrix, "Matrix", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Equation { source: "A := [[1, 2], [3, 4]]".into() }));
        }
        if theme::ribbon_button(ui, Icon::Vector, "Vector", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Equation { source: "v := [1, 2, 3]".into() }));
        }
        if theme::ribbon_button(ui, Icon::Grid, "Identity", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Equation { source: "I3 := identity(3)".into() }));
        }
        if theme::ribbon_button(ui, Icon::Chevron, "Range", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Equation { source: "k := 0..10".into() }));
        }
    });
    theme::ribbon_group(ui, "Operations", t, |ui| {
        for (label, snip) in [
            ("Transpose", "transpose(A)"), ("Determinant", "det(A)"), ("Inverse", "inv(A)"),
            ("Rank", "rank(A)"), ("Solve", "solve(A, b)"), ("Dot", "dot(v, v)"), ("Cross", "cross(v, v)"),
        ] {
            if theme::ribbon_button(ui, Icon::Matrix, label, true, true, t).clicked() {
                actions.push(insert_kind(state, BlockKind::Result { source: snip.into() }));
            }
        }
    });
    theme::ribbon_group(ui, "Assembly", t, |ui| {
        for (label, snip) in [("Augment", "augment(A, A)"), ("Stack", "stack(A, A)"), ("Eigen", "eigenvals(A)")] {
            if theme::ribbon_button(ui, Icon::Sigma, label, true, true, t).clicked() {
                actions.push(insert_kind(state, BlockKind::Result { source: snip.into() }));
            }
        }
    });
}

fn functions_tab(ui: &mut Ui, state: &mut UiState, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "Library", t, |ui| {
        palette_button(ui, state, 5, Icon::Sine, t);
        palette_button(ui, state, 0, Icon::Equation, t);
        palette_button(ui, state, 2, Icon::Sigma, t);
    });
    theme::ribbon_group(ui, "Solvers", t, |ui| {
        for (label, snip) in [
            ("Root", "root(f, 0, 1)"), ("Newton", "newton(f, 1)"), ("Minimise", "minimize(f, 0, 1)"),
            ("Maximise", "maximize(f, 0, 1)"), ("ODE", "odesolve(f, 0, 1, 1, 200)"),
        ] {
            if theme::ribbon_button(ui, Icon::Gear, label, true, true, t).clicked() {
                actions.push(insert_kind(state, BlockKind::Result { source: snip.into() }));
            }
        }
    });
    theme::ribbon_group(ui, "Fitting", t, |ui| {
        for (label, snip) in [("Polyfit", "polyfit(x, y, 2)"), ("Linfit", "linfit(x, y)"),
                              ("Expfit", "expfit(x, y)"), ("Interp", "linterp(x, y, 0.5)")] {
            if theme::ribbon_button(ui, Icon::Plot2D, label, true, true, t).clicked() {
                actions.push(insert_kind(state, BlockKind::Result { source: snip.into() }));
            }
        }
    });
    theme::ribbon_group(ui, "Scripting", t, |ui| {
        if theme::ribbon_button(ui, Icon::Python, "Python", true, true, t).clicked() {
            actions.push(insert_kind(state, BlockKind::Python { source: "# trusted user code\n".into(), trusted: false }));
        }
    });
}

fn calculation_tab(ui: &mut Ui, session: &mut Session, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "Calculate", t, |ui| {
        if theme::ribbon_button(ui, Icon::Bolt, "Calculate", true, true, t).clicked() { actions.push(UiAction::Recalculate); }
        if theme::ribbon_button(ui, Icon::Stop, "Cancel", true, session.busy, t).clicked() { actions.push(UiAction::Cancel); }
        if theme::ribbon_button(ui, Icon::Reset, "Reset Py", true, true, t).clicked() { actions.push(UiAction::ResetPython); }
    });
    theme::ribbon_group(ui, "Mode", t, |ui| {
        theme::ribbon_stack(ui, 96.0, |ui| {
            let auto = session.project.settings.mode == CalcMode::Auto;
            if ui.selectable_label(auto, "Automatic").clicked() { actions.push(UiAction::SetMode(CalcMode::Auto)); }
            if ui.selectable_label(!auto, "Manual").clicked() { actions.push(UiAction::SetMode(CalcMode::Manual)); }
        });
    });
    theme::ribbon_group(ui, "Backend", t, |ui| {
        theme::ribbon_stack(ui, 148.0, |ui| {
            for (b, label) in [(Backend::Auto, "Auto"), (Backend::Native, "Native Rust"), (Backend::Python, "Embedded Python")] {
                if ui.selectable_label(session.project.settings.backend == b, label).clicked() { actions.push(UiAction::SetBackend(b)); }
            }
        });
    });
    theme::ribbon_group(ui, "Solver settings", t, |ui| {
        theme::ribbon_stack(ui, 210.0, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Precision").size(11.5).color(t.ink_soft));
                let mut p = session.project.settings.display_precision;
                if ui.add(egui::DragValue::new(&mut p).range(2..=12)).changed() { session.project.settings.display_precision = p; }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Timeout").size(11.5).color(t.ink_soft));
                let mut ms = session.project.settings.timeout_ms;
                if ui.add(egui::DragValue::new(&mut ms).range(500..=120000).speed(50).suffix(" ms")).changed() { session.project.settings.timeout_ms = ms; }
            });
            let mut allow = session.project.python.allow_user_code;
            if ui.checkbox(&mut allow, "Allow user Python").changed() { actions.push(UiAction::SetAllowUserCode(allow)); }
        });
    });
}

fn document_tab(ui: &mut Ui, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "Project", t, |ui| {
        if theme::ribbon_button(ui, Icon::New, "New", true, true, t).clicked() { actions.push(UiAction::New); }
        if theme::ribbon_button(ui, Icon::Open, "Open", true, true, t).clicked() { actions.push(UiAction::Open); }
        if theme::ribbon_button(ui, Icon::Save, "Save", true, true, t).clicked() { actions.push(UiAction::Save); }
        if theme::ribbon_button(ui, Icon::Copy, "Save as", true, true, t).clicked() { actions.push(UiAction::SaveAs); }
        if theme::ribbon_button(ui, Icon::Doc, "Demo", true, true, t).clicked() { actions.push(UiAction::LoadDemo); }
    });
    theme::ribbon_group(ui, "Export", t, |ui| {
        if theme::ribbon_button(ui, Icon::Python, "Python", true, true, t).clicked() { actions.push(UiAction::ExportPython); }
        if theme::ribbon_button(ui, Icon::Export, "HTML", true, true, t).clicked() { actions.push(UiAction::ExportHtml); }
        if theme::ribbon_button(ui, Icon::Export, "Markdown", true, true, t).clicked() { actions.push(UiAction::ExportMarkdown); }
        if theme::ribbon_button(ui, Icon::Doc, "Typst", true, true, t).clicked() { actions.push(UiAction::ExportTypst); }
        if theme::ribbon_button(ui, Icon::Doc, "PDF", true, true, t).clicked() { actions.push(UiAction::ExportPdf); }
    });
    theme::ribbon_group(ui, "Worksheets", t, |ui| {
        if theme::ribbon_button(ui, Icon::Plus, "Add sheet", true, true, t).clicked() { actions.push(UiAction::AddWorksheet); }
    });
}

fn view_tab(ui: &mut Ui, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>, t: &Theme) {
    theme::ribbon_group(ui, "Panels", t, |ui| {
        theme::ribbon_stack(ui, 116.0, |ui| {
            let mut l = session.project.layout.show_left;
            if ui.checkbox(&mut l, "Navigation").changed() { actions.push(UiAction::TogglePanel(0)); }
            let mut r = session.project.layout.show_right;
            if ui.checkbox(&mut r, "Inspector").changed() { actions.push(UiAction::TogglePanel(1)); }
            let mut b = session.project.layout.show_bottom;
            if ui.checkbox(&mut b, "Output dock").changed() { actions.push(UiAction::TogglePanel(2)); }
        });
    });
    theme::ribbon_group(ui, "Zoom", t, |ui| {
        if theme::ribbon_button(ui, Icon::Minus, "Out", true, true, t).clicked() { actions.push(UiAction::SetZoom(state.zoom - 0.1)); }
        if theme::ribbon_button(ui, Icon::Zoom, "100%", true, true, t).clicked() { actions.push(UiAction::SetZoom(1.0)); }
        if theme::ribbon_button(ui, Icon::Plus, "In", true, true, t).clicked() { actions.push(UiAction::SetZoom(state.zoom + 0.1)); }
        if theme::ribbon_button(ui, Icon::Reset, "Reset view", true, true, t).clicked() { actions.push(UiAction::ResetView); }
    });
    theme::ribbon_group(ui, "Grid", t, |ui| {
        if theme::ribbon_button(ui, Icon::Grid, if state.show_grid { "Grid on" } else { "Grid off" }, true, true, t).clicked() { actions.push(UiAction::ToggleGrid); }
        if theme::ribbon_button(ui, Icon::Check, if state.snap { "Snap on" } else { "Snap off" }, true, true, t).clicked() { actions.push(UiAction::ToggleSnap); }
    });
    theme::ribbon_group(ui, "Theme", t, |ui| {
        theme::ribbon_stack(ui, 92.0, |ui| {
            if ui.selectable_label(!t.night, "Mathcad").clicked() { actions.push(UiAction::SetTheme("mathcad".into())); }
            if ui.selectable_label(t.night, "Night").clicked() { actions.push(UiAction::SetTheme("night".into())); }
        });
    });
}

// ------------------------------------------------------------ navigation ---
pub fn left_panel(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let t = theme::current(ctx);
    egui::SidePanel::left("left").resizable(false).exact_width(state.left_width)
        .frame(egui::Frame::none().fill(t.ribbon).inner_margin(egui::Margin::symmetric(14.0, 8.0))
            .stroke(Stroke::new(1.0_f32, t.ribbon_edge)))
        .show(ctx, |ui| {
            theme::scroll(ui, &t, |ui| {
                ui.set_min_width(ui.available_width());
                theme::section(ui, "Worksheets", &t);
                let names: Vec<String> = session.project.worksheets.iter().map(|w| w.name.clone()).collect();
                let row_w = (ui.available_width() - 10.0).max(80.0);
                for (i, name) in names.iter().enumerate() {
                    let active = session.active == i;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(row_w, 25.0), egui::Sense::click());
                    let anim = ctx.animate_bool_with_time(resp.id.with("h"), active || resp.hovered(), 0.12);
                    if anim > 0.001 && anim < 0.999 { ctx.request_repaint(); }
                    let painter = ui.painter_at(rect);
                    if active {
                        painter.rect_filled(rect, 4.0, t.accent_soft);
                        painter.rect_filled(egui::Rect::from_min_size(rect.left_top(), Vec2::new(2.5, rect.height())), 1.5, t.accent);
                    } else if anim > 0.01 { painter.rect_filled(rect, 4.0, t.faint_hover().linear_multiply(anim)); }
                    theme::paint_icon(&painter, egui::Rect::from_center_size(Pos2::new(rect.left() + 16.0, rect.center().y), Vec2::splat(13.0)),
                        Icon::Doc, if active { t.accent } else { t.ink_faint });
                    painter.text(Pos2::new(rect.left() + 28.0, rect.center().y), egui::Align2::LEFT_CENTER, name,
                        egui::FontId::proportional(12.0), if active { t.accent_deep } else { t.ink });
                    if resp.clicked() { actions.push(UiAction::SelectWorksheet(i)); }
                }
                if theme::ribbon_button(ui, Icon::Plus, "New worksheet", false, true, &t).clicked() { actions.push(UiAction::AddWorksheet); }

                let vars = session.variables();
                let values: Vec<&aaiyyan_core::VariableInfo> = vars.iter().filter(|v| v.kind != "function").collect();
                theme::section(ui, &format!("Variables · {}", values.len()), &t);
                if values.is_empty() {
                    ui.label(egui::RichText::new("Define one:  E := 210 GPa").size(11.0).color(t.ink_faint));
                }
                for v in &values {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&v.name).font(theme::math_italic(14.0)).color(t.ink));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(&v.display).size(11.0)
                                .color(if v.unit.is_some() { t.unit } else { t.ink_soft }));
                        });
                    });
                }
                let funcs: Vec<&aaiyyan_core::VariableInfo> = vars.iter().filter(|v| v.kind == "function").collect();
                theme::section(ui, &format!("Functions · {}", funcs.len()), &t);
                if funcs.is_empty() {
                    ui.label(egui::RichText::new("Define one:  f(x) := a*sin(x)").size(11.0).color(t.ink_faint));
                }
                for v in funcs { ui.label(egui::RichText::new(&v.display).font(theme::math(13.0)).color(t.ink_soft)); }

                theme::section(ui, "Constants", &t);
                for (n, val) in [("pi", "3.14159265"), ("e", "2.71828183"), ("i", "imaginary unit")] {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(n).font(theme::math_italic(14.0)).color(t.ink));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(val).size(11.0).color(t.ink_faint));
                        });
                    });
                }
                theme::section(ui, "Engine", &t);
                let info = session.python_info();
                ui.horizontal(|ui| {
                    theme::icon(ui, if info.available { Icon::Check } else { Icon::Info }, 13.0, if info.available { t.ok } else { t.warn });
                    ui.label(egui::RichText::new(if info.available { format!("Python {}", info.version) } else { "Native Rust only".to_string() })
                        .size(11.0).color(t.ink_soft));
                });
                if !info.available {
                    ui.label(egui::RichText::new("build with --features python for SymPy / SciPy").size(10.0).color(t.ink_faint));
                }
                ui.add_space(10.0);
            });
        });
}

// -------------------------------------------------------------- inspector ---
fn kv(ui: &mut Ui, k: &str, v: &str, color: Color32, t: &Theme) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(k).size(11.0).color(t.ink_soft));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(v).size(11.5).strong().color(color));
        });
    });
}

pub fn right_panel(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let t = theme::current(ctx);
    let selected = state.selected;
    let block = selected.and_then(|id| session.worksheet().block(id).cloned());
    egui::SidePanel::right("right").resizable(false).exact_width(state.right_width)
        .frame(egui::Frame::none().fill(t.ribbon).inner_margin(egui::Margin::symmetric(13.0, 8.0))
            .stroke(Stroke::new(1.0_f32, t.ribbon_edge)))
        .show(ctx, |ui| {
            theme::scroll(ui, &t, |ui| {
                ui.set_min_width(ui.available_width());
                if let Some(block) = &block {
                    theme::section(ui, &format!("{} region", block.kind.type_name()), &t);
                    theme::card(ui, &t, |ui| {
                        kv(ui, "Position", &format!("{:.0}, {:.0}", block.x, block.y), t.ink, &t);
                        kv(ui, "Enabled", if block.enabled { "yes" } else { "no" }, if block.enabled { t.ok } else { t.warn }, &t);
                    });
                    match &block.kind {
                        BlockKind::Equation { source } | BlockKind::Result { source } => {
                            match aaiyyan_parser::parse_program(source) {
                                Ok(stmts) => for stmt in &stmts {
                                    if let Some(name) = stmt.defined_name() {
                                        if let Some(r) = session.result(name) {
                                            ui.add_space(6.0);
                                            theme::card(ui, &t, |ui| {
                                                ui.label(egui::RichText::new(name).font(theme::math_italic(15.0)).color(t.ink_soft));
                                                ui.add_space(2.0);
                                                ui.label(egui::RichText::new(&r.display).font(theme::math(20.0)).strong().color(t.ink));
                                            });
                                            if let aaiyyan_evaluator::Value::Quantity(q) = &r.value {
                                                theme::section(ui, "Dimensional analysis", &t);
                                                theme::card(ui, &t, |ui| {
                                                    kv(ui, "Quantity", &q.dim().quantity_name(), t.ink, &t);
                                                    kv(ui, "SI value", &theme::num(q.si), t.ink, &t);
                                                    kv(ui, "Unit", &q.unit.name, t.unit, &t);
                                                    kv(ui, "Dimension", &q.dim().to_unit_string(), t.ink_soft, &t);
                                                });
                                            }
                                        }
                                    }
                                    let deps = aaiyyan_ast::statement_dependencies(stmt);
                                    if !deps.is_empty() {
                                        theme::section(ui, "Depends on", &t);
                                        ui.horizontal_wrapped(|ui| { for d in deps { theme::chip(ui, &d, t.accent, &t); } });
                                    }
                                    theme::section(ui, "LaTeX", &t);
                                    ui.label(egui::RichText::new(aaiyyan_ast::latex::statement_to_latex(stmt))
                                        .font(theme::mono(10.5)).color(t.ink_soft));
                                },
                                Err(e) => {
                                    ui.add_space(6.0);
                                    theme::card(ui, &t, |ui| {
                                        ui.horizontal(|ui| {
                                            theme::icon(ui, Icon::Info, 13.0, t.danger);
                                            ui.label(egui::RichText::new(e.describe(source)).size(11.0).color(t.danger));
                                        });
                                    });
                                }
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Edit").clicked() { actions.push(UiAction::Edit(block.id, true)); }
                                if ui.button("Delete").clicked() { actions.push(UiAction::Delete(block.id)); }
                            });
                        }
                        BlockKind::Plot2D(spec) => {
                            theme::section(ui, "Plot properties", &t);
                            let mut s = spec.clone();
                            let mut changed = false;
                            theme::card(ui, &t, |ui| {
                                ui.label(egui::RichText::new("Title").size(10.5).color(t.ink_faint));
                                changed |= ui.text_edit_singleline(&mut s.title).changed();
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Expression").size(10.5).color(t.ink_faint));
                                let mut expr = s.series.first().map(|x| x.expr.clone()).unwrap_or_default();
                                if ui.text_edit_singleline(&mut expr).changed() {
                                    let variable = s.series.first().map(|x| x.variable.clone()).unwrap_or_else(|| "x".into());
                                    let kind = s.series.first().map(|x| x.kind).unwrap_or(SeriesKind::Line);
                                    s.series = vec![SeriesSpec { expr: expr.clone(), variable, label: expr, kind, color: None }];
                                    changed = true;
                                }
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("x range").size(10.5).color(t.ink_faint));
                                    changed |= ui.add(egui::DragValue::new(&mut s.x_min).speed(0.1)).changed();
                                    changed |= ui.add(egui::DragValue::new(&mut s.x_max).speed(0.1)).changed();
                                });
                                changed |= ui.add(egui::Slider::new(&mut s.samples, 20..=2000).text("samples")).changed();
                                changed |= ui.checkbox(&mut s.show_legend, "legend").changed();
                            });
                            if changed { actions.push(UiAction::UpdatePlot2D(block.id, s)); }
                        }
                        BlockKind::Plot3D(spec) => {
                            theme::section(ui, "Surface properties", &t);
                            let mut s = spec.clone();
                            let mut changed = false;
                            theme::card(ui, &t, |ui| {
                                ui.label(egui::RichText::new("z = f(x, y)").size(10.5).color(t.ink_faint));
                                changed |= ui.text_edit_singleline(&mut s.expr).changed();
                                changed |= ui.add(egui::Slider::new(&mut s.samples, 5..=120).text("grid")).changed();
                            });
                            if changed { actions.push(UiAction::UpdatePlot3D(block.id, s)); }
                        }
                        _ => {
                            ui.add_space(6.0);
                            theme::card(ui, &t, |ui| {
                                ui.label(egui::RichText::new("This region has no numeric properties.").size(11.0).color(t.ink_faint));
                            });
                        }
                    }
                } else {
                    // Nothing selected → a genuinely useful default panel.
                    theme::section(ui, "Worksheet", &t);
                    let vars = session.variables();
                    theme::card(ui, &t, |ui| {
                        kv(ui, "Regions", &session.worksheet().blocks.len().to_string(), t.ink, &t);
                        kv(ui, "Variables", &vars.iter().filter(|v| v.kind != "function").count().to_string(), t.ink, &t);
                        kv(ui, "Functions", &vars.iter().filter(|v| v.kind == "function").count().to_string(), t.ink, &t);
                        kv(ui, "Errors", &session.block_errors.len().to_string(),
                           if session.block_errors.is_empty() { t.ok } else { t.danger }, &t);
                        if let Some(r) = &session.last_calc {
                            kv(ui, "Last solve", &format!("{:.1} ms", r.duration_ms), t.ink_soft, &t);
                        }
                    });
                    theme::section(ui, "Quick start", &t);
                    theme::card(ui, &t, |ui| {
                        for (n, txt) in [
                            ("1", "Click the grid — a crosshair appears"),
                            ("2", "Type  E := 210 GPa  and press Enter"),
                            ("3", "Type  E =  to evaluate with units"),
                            ("4", "Open Palettes for operators and units"),
                        ] {
                            ui.horizontal(|ui| {
                                theme::chip(ui, n, t.accent, &t);
                                ui.label(egui::RichText::new(txt).size(11.0).color(t.ink_soft));
                            });
                            ui.add_space(2.0);
                        }
                    });
                    theme::section(ui, "Keyboard", &t);
                    theme::card(ui, &t, |ui| {
                        for (k, d) in [
                            ("F5", "Recalculate"), ("Esc", "Finish editing"), ("Enter", "Commit equation"),
                            ("Del", "Delete region"), ("Ctrl+Z", "Undo"), ("Ctrl+S", "Save"),
                            ("Ctrl+±", "Zoom"), ("Alt+drag", "Move region"),
                        ] {
                            ui.horizontal(|ui| {
                                theme::key_chip(ui, k, &t);
                                ui.label(egui::RichText::new(d).size(11.0).color(t.ink_soft));
                            });
                        }
                    });
                }

                theme::section(ui, "Calculation", &t);
                theme::card(ui, &t, |ui| {
                    kv(ui, "Mode", if session.project.settings.mode == CalcMode::Auto { "Automatic" } else { "Manual" }, t.accent, &t);
                    kv(ui, "Precision", &session.project.settings.display_precision.to_string(), t.ink, &t);
                    kv(ui, "Timeout", &format!("{} ms", session.project.settings.timeout_ms), t.ink_soft, &t);
                    let mut allow = session.project.python.allow_user_code;
                    ui.add_space(3.0);
                    if ui.checkbox(&mut allow, "Allow user Python code").changed() { actions.push(UiAction::SetAllowUserCode(allow)); }
                });
                ui.add_space(10.0);
            });
        });
}

// ------------------------------------------------------------------- dock ---
pub fn bottom_panel(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let t = theme::current(ctx);
    egui::TopBottomPanel::bottom("bottom").resizable(false).exact_height(state.dock_height)
        .frame(egui::Frame::none().fill(t.ribbon).inner_margin(egui::Margin::symmetric(12.0, 7.0))
            .stroke(Stroke::new(1.0_f32, t.ribbon_edge)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, label) in ["Generated Python", "Console", "Errors", "History"].iter().enumerate() {
                    let active = state.bottom_tab == i;
                    let font = egui::FontId::proportional(11.5);
                    let w = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font.clone(), t.ink).size().x) + 22.0;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 23.0), egui::Sense::click());
                    let anim = ctx.animate_bool_with_time(resp.id.with("h"), active || resp.hovered(), 0.12);
                    if anim > 0.001 && anim < 0.999 { ctx.request_repaint(); }
                    let painter = ui.painter_at(rect);
                    if active {
                        painter.rect_filled(rect, 3.0, t.accent_soft);
                        painter.line_segment([Pos2::new(rect.left() + 6.0, rect.bottom() - 1.0), Pos2::new(rect.right() - 6.0, rect.bottom() - 1.0)],
                            Stroke::new(2.0_f32, t.accent));
                    } else if anim > 0.01 { painter.rect_filled(rect, 3.0, t.faint_hover().linear_multiply(anim)); }
                    let badge = if i == 2 && !session.block_errors.is_empty() { format!("  {}", session.block_errors.len()) } else { String::new() };
                    painter.text(rect.center(), egui::Align2::CENTER_CENTER, format!("{label}{badge}"), font,
                        if active { t.accent_deep } else { t.ink_soft });
                    if resp.clicked() { state.bottom_tab = i; }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::ribbon_button(ui, Icon::Export, "Export script", false, true, &t).clicked() { actions.push(UiAction::ExportPython); }
                    if theme::ribbon_button(ui, Icon::Reset, "Reset namespace", false, true, &t).clicked() { actions.push(UiAction::ResetPython); }
                });
            });
            ui.add_space(5.0);
            match state.bottom_tab {
                0 => {
                    let mut code = session.generated.clone();
                    if code.trim().is_empty() { code = "# Define a region on the worksheet to generate Python.".into(); }
                    let _ = editor::code_editor(ui, egui::Id::new("generated"), &mut code, 7, true);
                }
                1 => theme::card(ui, &t, |ui| {
                    theme::scroll(ui, &t, |ui| {
                        ui.set_min_height(96.0);
                        if session.console.is_empty() {
                            ui.label(egui::RichText::new("Console is empty.").size(11.5).color(t.ink_faint));
                        }
                        for e in session.console.iter().rev() {
                            let c = match e.level {
                                aaiyyan_core::LogLevel::Info => t.ink_soft,
                                aaiyyan_core::LogLevel::Warning => t.warn,
                                aaiyyan_core::LogLevel::Error => t.danger,
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&e.time).font(theme::mono(10.5)).color(t.ink_faint));
                                ui.label(egui::RichText::new(&e.message).font(theme::mono(11.0)).color(c));
                            });
                        }
                    });
                }),
                2 => theme::card(ui, &t, |ui| {
                    theme::scroll(ui, &t, |ui| {
                        ui.set_min_height(96.0);
                        if session.block_errors.is_empty() {
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::Check, 14.0, t.ok);
                                ui.label(egui::RichText::new("No errors — worksheet is dimensionally consistent.").size(11.5).color(t.ok));
                            });
                        }
                        for (id, errs) in &session.block_errors {
                            for e in errs {
                                ui.horizontal(|ui| {
                                    theme::icon(ui, Icon::Info, 12.0, t.danger);
                                    ui.label(egui::RichText::new(format!("{id}: {e}")).size(11.0).color(t.danger));
                                });
                            }
                        }
                    });
                }),
                _ => theme::card(ui, &t, |ui| {
                    if let Some(r) = &session.last_calc {
                        kv(ui, "Backend", &r.backend, t.accent, &t);
                        kv(ui, "Duration", &format!("{:.2} ms", r.duration_ms), t.ink, &t);
                        kv(ui, "Errors", &r.errors.to_string(), if r.errors == 0 { t.ok } else { t.danger }, &t);
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(r.evaluated.join(", ")).font(theme::mono(10.5)).color(t.ink_soft));
                    } else {
                        ui.label(egui::RichText::new("No calculation yet — press F5.").size(11.5).color(t.ink_faint));
                    }
                }),
            }
        });
}

// ----------------------------------------------------------------- status ---
pub fn status_bar(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let t = theme::current(ctx);
    egui::TopBottomPanel::bottom("status").exact_height(27.0)
        .frame(egui::Frame::none().fill(t.chrome_alt).inner_margin(egui::Margin::symmetric(12.0, 3.0))
            .stroke(Stroke::new(1.0_f32, t.ribbon_edge)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (color, text) = if session.busy { (t.accent, "Calculating…") }
                    else if !session.block_errors.is_empty() { (t.danger, "Errors in worksheet") }
                    else if session.stale { (t.warn, "Recalculation pending") }
                    else { (t.ok, "Ready") };
                theme::chip(ui, text, color, &t);
                ui.label(egui::RichText::new(format!("Sheet {} / {}", session.active + 1, session.project.worksheets.len())).size(11.0).color(t.ink_faint));
                if let Some(c) = state.cursor {
                    ui.label(egui::RichText::new(format!("x {:.0}   y {:.0}", c.x, c.y)).size(11.0).color(t.ink_faint));
                }
                if let Some(r) = &session.last_calc {
                    ui.label(egui::RichText::new(format!("{} · {:.1} ms", r.backend, r.duration_ms)).size(11.0).color(t.ink_faint));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if theme::ribbon_button(ui, Icon::Plus, "", false, true, &t).on_hover_text("Zoom in").clicked() { actions.push(UiAction::SetZoom(state.zoom + 0.1)); }
                    ui.label(egui::RichText::new(format!("{:.0}%", state.zoom * 100.0)).size(11.0).color(t.ink_soft));
                    if theme::ribbon_button(ui, Icon::Minus, "", false, true, &t).on_hover_text("Zoom out").clicked() { actions.push(UiAction::SetZoom(state.zoom - 0.1)); }
                    ui.add_space(6.0);
                    if let Some(p) = &session.project_path {
                        ui.label(egui::RichText::new(p.display().to_string()).size(10.5).color(t.ink_faint));
                    }
                });
            });
        });
}

// ---------------------------------------------------------------- actions ---
pub fn apply_actions(ctx: &egui::Context, session: &mut Session, state: &mut UiState, actions: Vec<UiAction>) {
    for action in actions {
        match action {
            UiAction::New => {
                session.shutdown();
                *session = Session::from_project(aaiyyan_project::Project::empty("Untitled project"), None);
                *state = UiState::default();
            }
            UiAction::LoadDemo => { session.shutdown(); *session = Session::new(); *state = UiState::default(); }
            UiAction::Open => {
                if let Some(path) = rfd::FileDialog::new().add_filter("AaiyyanMath project", &["json"]).pick_file() {
                    match Session::open(&path) {
                        Ok(s) => { session.shutdown(); *session = s; *state = UiState::default(); session.request_recalc(); }
                        Err(e) => session.log(aaiyyan_core::LogLevel::Error, e),
                    }
                }
            }
            UiAction::Save => {
                let path = session.project_path.clone().or_else(save_dialog);
                if let Some(p) = path { if let Err(e) = session.save(&p) { session.log(aaiyyan_core::LogLevel::Error, e); } }
            }
            UiAction::SaveAs => {
                if let Some(p) = save_dialog() { if let Err(e) = session.save(&p) { session.log(aaiyyan_core::LogLevel::Error, e); } }
            }
            UiAction::Undo => session.undo(),
            UiAction::Redo => session.redo(),
            UiAction::Recalculate => session.force_recalc(),
            UiAction::Cancel => session.cancel(),
            UiAction::Select(id) => state.selected = Some(id),
            UiAction::Edit(id, on) => {
                state.selected = Some(id);
                state.editing = if on { Some(id) } else { None };
                if on { state.focus_request = Some(id); state.cursor = None; }
                else {
                    session.end_edit();
                    let pos = session.worksheet().block(id).map(|b| egui::Vec2::new(b.x, b.y));
                    let size = state.sizes.get(&id).copied();
                    if let (Some(p), Some(s)) = (pos, size) {
                        let step = crate::canvas::GRID;
                        let y = p.y + s.y / state.zoom.max(0.01) + step * 2.0;
                        state.cursor = Some(egui::Vec2::new(p.x, (y / step).round() * step));
                    }
                }
            }
            UiAction::UpdateSource(id, source) => session.update_source(id, source),
            UiAction::InsertSnippet(snip) => {
                let target = state.editing.or(state.selected);
                let existing = target.and_then(|id| session.worksheet().block(id).map(|b| (id, b.kind.clone())));
                match existing {
                    Some((id, BlockKind::Equation { source })) | Some((id, BlockKind::Result { source })) => {
                        session.update_source(id, format!("{source}{snip}"));
                        state.editing = Some(id); state.focus_request = Some(id);
                    }
                    _ => {
                        let (x, y) = drop_point(state);
                        let id = session.insert_block_at(BlockKind::Equation { source: snip }, x, y);
                        state.selected = Some(id); state.editing = Some(id);
                        state.focus_request = Some(id); state.cursor = None;
                    }
                }
            }
            UiAction::InsertSnippetAt(snip, x, y) => {
                let id = session.insert_block_at(BlockKind::Equation { source: snip }, x, y);
                state.selected = Some(id); state.editing = Some(id); state.focus_request = Some(id); state.cursor = None;
            }
            UiAction::InsertBlockAt(kind, x, y) => {
                let editable = matches!(kind, BlockKind::Equation { .. } | BlockKind::Result { .. } | BlockKind::Text { .. } | BlockKind::Heading { .. });
                let id = session.insert_block_at(kind, x, y);
                state.selected = Some(id);
                if editable { state.editing = Some(id); state.focus_request = Some(id); state.cursor = None; }
                else { state.cursor = Some(egui::Vec2::new(x, y + 348.0)); }
            }
            UiAction::SetBlockPos(id, x, y) => session.set_block_position(id, x, y),
            UiAction::SetBlockPosDelta(id, dx, dy) => {
                if let Some(b) = session.worksheet().block(id) {
                    let (x, y) = (b.x + dx, b.y + dy);
                    session.set_block_position(id, crate::canvas::snap(x, state.snap), crate::canvas::snap(y, state.snap));
                }
            }
            UiAction::UpdatePlot2D(id, spec) => session.update_block(id, BlockKind::Plot2D(spec)),
            UiAction::UpdatePlot3D(id, spec) => session.update_block(id, BlockKind::Plot3D(spec)),
            UiAction::UpdateControl(id, kind) => session.set_control_value(id, kind),
            UiAction::Insert(kind) => { let (x, y) = drop_point(state); session.insert_block_at(kind, x, y); }
            UiAction::Delete(id) => {
                session.remove_block(id);
                if state.selected == Some(id) { state.selected = None; }
                if state.editing == Some(id) { state.editing = None; }
            }
            UiAction::Move(id, delta) => session.move_block(id, delta),
            UiAction::ToggleEnabled(id) => {
                session.record_history();
                if let Some(b) = session.worksheet_mut().block_mut(id) { b.enabled = !b.enabled; }
                session.force_recalc();
            }
            UiAction::RunPython(id) => session.run_python_block(id),
            UiAction::SetTrust(id, trust) => {
                let source = match session.worksheet().block(id).map(|b| b.kind.clone()) {
                    Some(BlockKind::Python { source, .. }) => source, _ => continue,
                };
                session.update_block(id, BlockKind::Python { source, trusted: trust });
            }
            UiAction::AddWorksheet => session.add_worksheet(),
            UiAction::SelectWorksheet(i) => session.select_worksheet(i),
            UiAction::SetMode(mode) => { session.project.settings.mode = mode; if mode == CalcMode::Auto { session.force_recalc(); } }
            UiAction::SetBackend(b) => { session.project.settings.backend = b; session.force_recalc(); }
            UiAction::SetTheme(th) => session.project.layout.theme = th,
            UiAction::SetAllowUserCode(a) => { session.project.python.allow_user_code = a; session.apply_runtime_settings(); }
            UiAction::TogglePanel(i) => match i {
                0 => session.project.layout.show_left = !session.project.layout.show_left,
                1 => session.project.layout.show_right = !session.project.layout.show_right,
                _ => session.project.layout.show_bottom = !session.project.layout.show_bottom,
            },
            UiAction::SetZoom(z) => state.zoom = z.clamp(0.35, 3.0),
            UiAction::ToggleGrid => state.show_grid = !state.show_grid,
            UiAction::ToggleSnap => state.snap = !state.snap,
            UiAction::ResetView => { state.pan = egui::Vec2::ZERO; state.zoom = 1.0; }
            UiAction::ExportPython => {
                if let Some(p) = rfd::FileDialog::new().add_filter("Python", &["py"]).set_file_name("worksheet.py").save_file() {
                    match std::fs::write(&p, session.export_python()) {
                        Ok(_) => session.log(aaiyyan_core::LogLevel::Info, format!("exported {}", p.display())),
                        Err(e) => session.log(aaiyyan_core::LogLevel::Error, e.to_string()),
                    }
                }
            }
            UiAction::ExportHtml => { if let Some(p) = rfd::FileDialog::new().add_filter("HTML", &["html"]).set_file_name("report.html").save_file() { if let Err(e) = session.export_html(&p) { session.log(aaiyyan_core::LogLevel::Error, e); } } }
            UiAction::ExportMarkdown => { if let Some(p) = rfd::FileDialog::new().add_filter("Markdown", &["md"]).set_file_name("report.md").save_file() { if let Err(e) = session.export_markdown(&p) { session.log(aaiyyan_core::LogLevel::Error, e); } } }
            UiAction::ExportTypst => { if let Some(d) = rfd::FileDialog::new().pick_folder() { match session.export_typst(&d) { Ok(p) => session.log(aaiyyan_core::LogLevel::Info, format!("wrote {}", p.display())), Err(e) => session.log(aaiyyan_core::LogLevel::Error, e) } } }
            UiAction::ExportPdf => { if let Some(d) = rfd::FileDialog::new().pick_folder() { match session.export_pdf(&d) { Ok(p) => session.log(aaiyyan_core::LogLevel::Info, format!("wrote {}", p.display())), Err(e) => session.log(aaiyyan_core::LogLevel::Error, e) } } }
            UiAction::ResetPython => session.reset_python(),
        }
    }
    let _ = ctx;
}

fn save_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("AaiyyanMath project", &["json"]).set_file_name("project.aaiyyan.json").save_file()
}
