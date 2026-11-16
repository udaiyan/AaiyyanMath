//! Mathcad-framed 2D plots and a shaded 3D surface viewer.
use crate::state::View3d;
use crate::theme::{self, Theme};
use aaiyyan_plotting::{Plot2DSpec, SeriesData, SeriesKind, SurfaceData};
use egui::{Color32, Pos2, Sense, Stroke, Ui, Vec2};

pub fn show_2d(ui: &mut Ui, id: egui::Id, spec: &Plot2DSpec, series: &[SeriesData]) {
    let t: Theme = theme::current(ui.ctx());
    egui::Frame::none().fill(t.page).stroke(Stroke::new(1.0_f32, t.card_edge))
        .rounding(4.0).inner_margin(egui::Margin::symmetric(10.0, 9.0))
        .show(ui, |ui| {
            let w = ui.available_width();
            ui.set_min_width(w);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&spec.title).size(12.0).strong().color(t.ink));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&spec.y_label).size(10.5).color(t.unit));
                });
            });
            ui.add_space(5.0);
            let mut plot = egui_plot::Plot::new(id).height(250.0).allow_zoom(true).allow_drag(true);
            if spec.show_legend { plot = plot.legend(egui_plot::Legend::default()); }
            plot.show(ui, |plot_ui| {
                for (i, s) in series.iter().enumerate() {
                    if s.points.is_empty() { continue; }
                    let color = s.color.map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                        .unwrap_or_else(|| theme::series_color(i, t.night));
                    let points = egui_plot::PlotPoints::from(s.points.clone());
                    let name = if s.label.is_empty() { format!("series {}", i + 1) } else { s.label.clone() };
                    match s.kind {
                        SeriesKind::Scatter => { plot_ui.points(egui_plot::Points::new(points).color(color).radius(2.6_f32).name(name)); }
                        _ => { plot_ui.line(egui_plot::Line::new(points).color(color).width(1.9_f32).name(name)); }
                    }
                }
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(theme::num(spec.x_min)).size(10.0).color(t.ink_faint));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(theme::num(spec.x_max)).size(10.0).color(t.ink_faint));
                    ui.label(egui::RichText::new(&spec.x_label).size(10.5).color(t.ink_soft));
                });
            });
        });
}

pub fn show_3d(ui: &mut Ui, surface: &SurfaceData, view: &mut View3d) {
    let t = theme::current(ui.ctx());
    let size = Vec2::new(ui.available_width().max(220.0), 300.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    if response.dragged() {
        let d = response.drag_delta();
        view.yaw += d.x * 0.01;
        view.pitch = (view.pitch + d.y * 0.01).clamp(-1.45, 1.45);
    }
    if response.hovered() {
        let z = ui.input(|i| i.zoom_delta());
        if (z - 1.0).abs() > 1e-3 { view.zoom = (view.zoom * z).clamp(0.25, 6.0); }
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, t.page);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, t.card_edge));

    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    for row in &surface.z { for v in row { if v.is_finite() { zmin = zmin.min(*v); zmax = zmax.max(*v); } } }
    if !zmin.is_finite() { zmin = -1.0; zmax = 1.0; }
    if (zmax - zmin).abs() < 1e-12 { zmax = zmin + 1.0; }
    let xmin = surface.xs.first().copied().unwrap_or(-1.0);
    let xmax = surface.xs.last().copied().unwrap_or(1.0);
    let ymin = surface.ys.first().copied().unwrap_or(-1.0);
    let ymax = surface.ys.last().copied().unwrap_or(1.0);
    let norm = |v: f64, lo: f64, hi: f64| -> f32 {
        if (hi - lo).abs() < 1e-12 { 0.0 } else { ((v - lo) / (hi - lo) * 2.0 - 1.0) as f32 }
    };
    let (sy, cy) = view.yaw.sin_cos();
    let (sp, cp) = view.pitch.sin_cos();
    let scale = rect.width().min(rect.height()) * 0.30 * view.zoom;
    let center = rect.center();
    let project = |x: f32, y: f32, z: f32| -> (Pos2, f32) {
        let x1 = x * cy - y * sy;
        let y1 = x * sy + y * cy;
        (Pos2::new(center.x + x1 * scale, center.y - (y1 * cp - z * sp) * scale), y1 * sp + z * cp)
    };
    let floor_pt = |x: f32, y: f32| project(x, y, -1.05).0;
    for k in 0..=4 {
        let f = -1.0 + k as f32 * 0.5;
        painter.line_segment([floor_pt(f, -1.0), floor_pt(f, 1.0)], Stroke::new(0.7_f32, t.grid_major));
        painter.line_segment([floor_pt(-1.0, f), floor_pt(1.0, f)], Stroke::new(0.7_f32, t.grid_major));
    }
    let n = surface.z.len();
    let mut quads: Vec<(f32, Vec<Pos2>, Color32)> = Vec::new();
    for j in 0..n.saturating_sub(1) {
        for i in 0..surface.z[j].len().saturating_sub(1) {
            let (z00, z10, z01, z11) = (surface.z[j][i], surface.z[j][i+1], surface.z[j+1][i], surface.z[j+1][i+1]);
            if ![z00, z10, z01, z11].iter().all(|v| v.is_finite()) { continue; }
            let p0 = project(norm(surface.xs[i], xmin, xmax), norm(surface.ys[j], ymin, ymax), norm(z00, zmin, zmax));
            let p1 = project(norm(surface.xs[i+1], xmin, xmax), norm(surface.ys[j], ymin, ymax), norm(z10, zmin, zmax));
            let p2 = project(norm(surface.xs[i+1], xmin, xmax), norm(surface.ys[j+1], ymin, ymax), norm(z11, zmin, zmax));
            let p3 = project(norm(surface.xs[i], xmin, xmax), norm(surface.ys[j+1], ymin, ymax), norm(z01, zmin, zmax));
            let depth = (p0.1 + p1.1 + p2.1 + p3.1) * 0.25;
            let h = (((z00 - zmin) / (zmax - zmin)) as f32).clamp(0.0, 1.0);
            let shade = (0.84 + 0.22 * ((z10 - z00) as f32).clamp(-1.0, 1.0)).clamp(0.58, 1.12);
            let base = Color32::from_rgb((20.0 + 190.0 * h) as u8, (110.0 + 40.0 * (1.0 - (h - 0.5).abs() * 2.0)) as u8, (215.0 - 170.0 * h) as u8);
            let color = Color32::from_rgb(
                (base.r() as f32 * shade).min(255.0) as u8,
                (base.g() as f32 * shade).min(255.0) as u8,
                (base.b() as f32 * shade).min(255.0) as u8);
            quads.push((depth, vec![p0.0, p1.0, p2.0, p3.0], color));
        }
    }
    quads.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let edge = Stroke::new(0.35_f32, Color32::from_black_alpha(if t.night { 90 } else { 34 }));
    for (_, pts, c) in quads { painter.add(egui::Shape::convex_polygon(pts, c, edge)); }
    for (o, d) in &surface.vectors {
        let a = project(norm(o[0], xmin, xmax), norm(o[1], ymin, ymax), 0.0);
        let b = project(norm(o[0] + d[0] * 0.2, xmin, xmax), norm(o[1] + d[1] * 0.2, ymin, ymax), norm(d[2], zmin, zmax));
        painter.line_segment([a.0, b.0], Stroke::new(1.3_f32, t.warn));
    }
    for p in &surface.points {
        let s = project(norm(p[0], xmin, xmax), norm(p[1], ymin, ymax), norm(p[2], zmin, zmax));
        painter.circle_filled(s.0, 2.2_f32, t.ok);
    }
    let axis = Stroke::new(1.1_f32, t.ink_faint);
    let o = project(-1.0, -1.0, -1.0);
    painter.line_segment([o.0, project(1.25, -1.0, -1.0).0], axis);
    painter.line_segment([o.0, project(-1.0, 1.25, -1.0).0], axis);
    painter.line_segment([o.0, project(-1.0, -1.0, 1.25).0], axis);
    for (l, p) in [("x", project(1.32, -1.0, -1.0).0), ("y", project(-1.0, 1.32, -1.0).0), ("z", project(-1.0, -1.0, 1.32).0)] {
        painter.text(p, egui::Align2::CENTER_CENTER, l, theme::math_italic(12.5), t.ink_soft);
    }
    painter.text(Pos2::new(rect.left() + 9.0, rect.bottom() - 8.0), egui::Align2::LEFT_BOTTOM,
        format!("z  {} … {}   ·   drag to rotate", theme::num(zmin), theme::num(zmax)),
        egui::FontId::proportional(10.0), t.ink_faint);
}
