//! Native Python editor with palette-matched syntax colouring.
use crate::theme::{self, Theme};
use egui::text::LayoutJob;

const KEYWORDS: &[&str] = &[
    "def", "return", "if", "elif", "else", "for", "while", "import", "from", "as", "class",
    "try", "except", "finally", "with", "lambda", "yield", "pass", "break", "continue",
    "and", "or", "not", "in", "is", "None", "True", "False", "global", "nonlocal", "raise", "assert",
];

pub fn highlight(code: &str, font: egui::FontId, t: &Theme) -> LayoutJob {
    let mut job = LayoutJob::default();
    let base = t.ink;
    let kw = if t.night { egui::Color32::from_rgb(186, 130, 245) } else { egui::Color32::from_rgb(124, 58, 173) };
    let string_color = if t.night { egui::Color32::from_rgb(62, 198, 140) } else { egui::Color32::from_rgb(18, 128, 92) };
    let comment = t.ink_faint;
    let number = if t.night { egui::Color32::from_rgb(232, 178, 74) } else { egui::Color32::from_rgb(178, 106, 0) };
    let builtin = t.accent;

    for line in code.split_inclusive('\n') {
        if line.trim_start().starts_with('#') {
            job.append(line, 0.0, egui::TextFormat { font_id: font.clone(), color: comment, ..Default::default() });
            continue;
        }
        let mut token = String::new();
        let mut in_string: Option<char> = None;
        let mut commented = false;
        for ch in line.chars() {
            if commented {
                job.append(&ch.to_string(), 0.0, egui::TextFormat { font_id: font.clone(), color: comment, ..Default::default() });
                continue;
            }
            if let Some(q) = in_string {
                token.push(ch);
                if ch == q {
                    job.append(&token, 0.0, egui::TextFormat { font_id: font.clone(), color: string_color, ..Default::default() });
                    token.clear(); in_string = None;
                }
                continue;
            }
            match ch {
                '"' | '\'' => { flush(&mut token, &mut job, &font, base, kw, number, builtin); in_string = Some(ch); token.push(ch); }
                '#' => {
                    flush(&mut token, &mut job, &font, base, kw, number, builtin);
                    commented = true;
                    job.append("#", 0.0, egui::TextFormat { font_id: font.clone(), color: comment, ..Default::default() });
                }
                c if c.is_alphanumeric() || c == '_' => token.push(c),
                c => {
                    flush(&mut token, &mut job, &font, base, kw, number, builtin);
                    job.append(&c.to_string(), 0.0, egui::TextFormat { font_id: font.clone(), color: t.ink_soft, ..Default::default() });
                }
            }
        }
        flush(&mut token, &mut job, &font, base, kw, number, builtin);
    }
    job
}

fn flush(token: &mut String, job: &mut LayoutJob, font: &egui::FontId, base: egui::Color32,
         kw: egui::Color32, number: egui::Color32, builtin: egui::Color32) {
    if token.is_empty() { return; }
    let color = if KEYWORDS.contains(&token.as_str()) { kw }
        else if token.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) { number }
        else if matches!(token.as_str(), "np" | "sp" | "pd" | "math" | "aaiyyan" | "ureg" | "scipy") { builtin }
        else { base };
    job.append(token, 0.0, egui::TextFormat { font_id: font.clone(), color, ..Default::default() });
    token.clear();
}

pub fn code_editor(ui: &mut egui::Ui, id: egui::Id, code: &mut String, rows: usize, read_only: bool) -> egui::Response {
    let t = theme::current(ui.ctx());
    let code_bg = ui.visuals().code_bg_color;
    egui::Frame::none()
        .fill(code_bg)
        .stroke(egui::Stroke::new(1.0_f32, t.card_edge))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(9.0, 7.0))
        .show(ui, |ui| {
            let font = theme::mono(12.0);
            let layout_font = font.clone();
            let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                let mut job = highlight(text, layout_font.clone(), &t);
                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
            };
            ui.add(egui::TextEdit::multiline(code)
                .id(id).desired_rows(rows).desired_width(f32::INFINITY)
                .font(font).interactive(!read_only).layouter(&mut layouter))
        }).inner
}
