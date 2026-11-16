//! Professional report generation (spec section 23).
//! Typst is the native document pipeline; PDF compilation uses the `typst` CLI
//! when available and degrades gracefully otherwise.
use aaiyyan_project::Project;
use aaiyyan_worksheet::BlockKind;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Typst CLI not available: {0}")]
    TypstMissing(String),
    #[error("Typst compilation failed: {0}")]
    TypstFailed(String),
}

pub struct ReportInput<'a> {
    pub project: &'a Project,
    pub results: &'a BTreeMap<String, String>,
    pub latex: &'a BTreeMap<String, String>,
    pub python: &'a str,
    pub plots: Vec<(String, String)>,
}

/// Parse the block source with the real parser so reports carry proper math.
fn math_of(source: &str) -> (String, String) {
    match aaiyyan_parser::parse_program(source) {
        Ok(stmts) if !stmts.is_empty() => {
            let math = stmts.iter().map(aaiyyan_ast::mathstr::statement_to_string).collect::<Vec<_>>().join("\n");
            let tex = stmts.iter().map(aaiyyan_ast::statement_to_latex).collect::<Vec<_>>().join(" \\\\\n");
            (math, tex)
        }
        _ => (source.to_string(), source.to_string()),
    }
}

fn result_lines(math: &str, results: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in math.lines() {
        if let Some(name) = line.split(":=").next() {
            let name = name.trim();
            if let Some(value) = results.get(name) {
                out.push((name.to_string(), value.clone()));
            }
        }
    }
    out
}

pub fn export_typst(input: &ReportInput) -> String {
    let p = input.project;
    let mut out = String::new();
    out.push_str(&format!(
        "#set document(title: \"{}\", author: \"{}\")\n", typst_esc(&p.metadata.name), typst_esc(&p.metadata.author)
    ));
    out.push_str("#set page(paper: \"a4\", margin: 2cm, numbering: \"1\")\n");
    out.push_str("#set text(size: 11pt)\n");
    out.push_str("#set heading(numbering: \"1.1\")\n\n");
    out.push_str(&format!("#align(center)[#text(size: 20pt)[{}]\\\n", typst_esc(&p.metadata.name)));
    out.push_str(&format!("#text(size: 11pt)[{}]\\\n", typst_esc(&p.metadata.author)));
    out.push_str(&format!("#text(size: 9pt)[{}]]\n\n", typst_esc(&p.metadata.modified)));
    if !p.metadata.description.is_empty() {
        out.push_str(&format!("{}\n\n", typst_esc(&p.metadata.description)));
    }
    for ws in &p.worksheets {
        out.push_str(&format!("= {}\n\n", typst_esc(&ws.name)));
        for block in &ws.blocks {
            match &block.kind {
                BlockKind::Heading { level, text } => {
                    out.push_str(&format!("{} {}\n\n", "=".repeat((*level as usize + 1).min(6)), typst_esc(text)));
                }
                BlockKind::Text { text } => out.push_str(&format!("{}\n\n", typst_esc(text))),
                BlockKind::Equation { source } | BlockKind::Result { source } => {
                    let (math, _tex) = math_of(source);
                    out.push_str(&format!("$ {} $\n\n", math.replace('$', "")));
                    for (name, value) in result_lines(&math, input.results) {
                        out.push_str(&format!("Result: `{}` = {}\n\n", typst_esc(&name), typst_esc(&value)));
                    }
                }
                BlockKind::Python { source, .. } => {
                    out.push_str("```python\n");
                    out.push_str(source);
                    out.push_str("\n```\n\n");
                }
                BlockKind::Table { columns, rows } => {
                    out.push_str(&format!("#table(columns: {},\n", columns.len().max(1)));
                    for c in columns { out.push_str(&format!("  [{}],\n", typst_esc(c))); }
                    for r in rows { for c in r { out.push_str(&format!("  [{}],\n", typst_esc(c))); } }
                    out.push_str(")\n\n");
                }
                BlockKind::Plot2D(spec) => {
                    if input.plots.iter().any(|(t, _)| *t == spec.title) {
                        let file = format!("plots/{}.svg", sanitize(&spec.title));
                        out.push_str(&format!(
                            "#figure(image(\"{file}\", width: 90%), caption: [{}])\n\n", typst_esc(&spec.title)
                        ));
                    }
                }
                BlockKind::Plot3D(spec) => {
                    out.push_str(&format!("_3D view: {} (interactive in application)_\n\n", typst_esc(&spec.title)));
                }
                BlockKind::Control(c) => {
                    out.push_str(&format!("Parameter `{}` (interactive control)\n\n", typst_esc(&c.variable)));
                }
                BlockKind::Image { path, caption } => {
                    out.push_str(&format!(
                        "#figure(image(\"{}\"), caption: [{}])\n\n", typst_esc(path), typst_esc(caption)
                    ));
                }
            }
        }
    }
    out.push_str("= Generated Python\n\n```python\n");
    out.push_str(input.python);
    out.push_str("\n```\n");
    out
}

pub fn write_typst_report(dir: &Path, input: &ReportInput) -> Result<PathBuf, ReportError> {
    fs::create_dir_all(dir)?;
    let plots_dir = dir.join("plots");
    fs::create_dir_all(&plots_dir)?;
    for (title, svg) in &input.plots {
        fs::write(plots_dir.join(format!("{}.svg", sanitize(title))), svg)?;
    }
    let path = dir.join("report.typ");
    fs::write(&path, export_typst(input))?;
    Ok(path)
}

pub fn compile_pdf(typst_file: &Path, pdf_file: &Path) -> Result<(), ReportError> {
    let output = std::process::Command::new("typst")
        .arg("compile")
        .arg(typst_file)
        .arg(pdf_file)
        .output()
        .map_err(|e| ReportError::TypstMissing(e.to_string()))?;
    if !output.status.success() {
        return Err(ReportError::TypstFailed(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    Ok(())
}

pub fn export_html(input: &ReportInput) -> String {
    let p = input.project;
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n");
    out.push_str(&format!("<title>{}</title>\n", esc(&p.metadata.name)));
    out.push_str("<style>body{font-family:system-ui,sans-serif;max-width:900px;margin:2rem auto;line-height:1.5;color:#1a1a1a}code,pre{font-family:ui-monospace,monospace;background:#f5f5f5;padding:2px 4px;border-radius:3px}pre{padding:1rem;overflow-x:auto}h1,h2{border-bottom:1px solid #ddd;padding-bottom:.3rem}.result{color:#0a6}table{border-collapse:collapse}td,th{border:1px solid #ccc;padding:4px 8px}</style>\n");
    out.push_str("</head><body>\n");
    out.push_str(&format!(
        "<h1>{}</h1>\n<p><em>{}</em></p>\n<p>{}</p>\n",
        esc(&p.metadata.name), esc(&p.metadata.author), esc(&p.metadata.description)
    ));
    for ws in &p.worksheets {
        out.push_str(&format!("<h2>{}</h2>\n", esc(&ws.name)));
        for block in &ws.blocks {
            match &block.kind {
                BlockKind::Heading { level, text } => {
                    let l = (*level + 2).min(6);
                    out.push_str(&format!("<h{0}>{1}</h{0}>\n", l, esc(text)));
                }
                BlockKind::Text { text } => out.push_str(&format!("<p>{}</p>\n", esc(text))),
                BlockKind::Equation { source } | BlockKind::Result { source } => {
                    let (math, tex) = math_of(source);
                    out.push_str(&format!("<pre data-latex=\"{}\">{}</pre>\n", esc(&tex), esc(&math)));
                    for (name, value) in result_lines(&math, input.results) {
                        out.push_str(&format!("<p class=\"result\">{} = {}</p>\n", esc(&name), esc(&value)));
                    }
                }
                BlockKind::Python { source, .. } => out.push_str(&format!("<pre>{}</pre>\n", esc(source))),
                BlockKind::Table { columns, rows } => {
                    out.push_str("<table><tr>");
                    for c in columns { out.push_str(&format!("<th>{}</th>", esc(c))); }
                    out.push_str("</tr>");
                    for r in rows {
                        out.push_str("<tr>");
                        for c in r { out.push_str(&format!("<td>{}</td>", esc(c))); }
                        out.push_str("</tr>");
                    }
                    out.push_str("</table>\n");
                }
                BlockKind::Plot2D(spec) => {
                    if let Some((_, svg)) = input.plots.iter().find(|(t, _)| *t == spec.title) {
                        out.push_str(svg);
                    }
                }
                BlockKind::Plot3D(spec) => out.push_str(&format!("<p><em>3D view: {}</em></p>\n", esc(&spec.title))),
                BlockKind::Control(c) => out.push_str(&format!("<p>Parameter <code>{}</code></p>\n", esc(&c.variable))),
                BlockKind::Image { path, caption } => out.push_str(&format!(
                    "<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>\n",
                    esc(path), esc(caption), esc(caption)
                )),
            }
        }
    }
    out.push_str("<h2>Generated Python</h2>\n<pre>");
    out.push_str(&esc(input.python));
    out.push_str("</pre>\n</body></html>\n");
    out
}

pub fn export_markdown(input: &ReportInput) -> String {
    let p = input.project;
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", p.metadata.name));
    if !p.metadata.author.is_empty() { out.push_str(&format!("*{}*\n\n", p.metadata.author)); }
    for ws in &p.worksheets {
        out.push_str(&format!("## {}\n\n", ws.name));
        for block in &ws.blocks {
            match &block.kind {
                BlockKind::Heading { level, text } => {
                    out.push_str(&format!("{} {}\n\n", "#".repeat((*level as usize + 2).min(6)), text));
                }
                BlockKind::Text { text } => out.push_str(&format!("{text}\n\n")),
                BlockKind::Equation { source } | BlockKind::Result { source } => {
                    let (math, tex) = math_of(source);
                    out.push_str(&format!("$$\n{tex}\n$$\n\n"));
                    for (name, value) in result_lines(&math, input.results) {
                        out.push_str(&format!("- `{name}` = {value}\n"));
                    }
                    out.push('\n');
                }
                BlockKind::Python { source, .. } => out.push_str(&format!("```python\n{source}\n```\n\n")),
                BlockKind::Table { columns, rows } => {
                    out.push_str(&format!("| {} |\n", columns.join(" | ")));
                    out.push_str(&format!("|{}\n", " --- |".repeat(columns.len().max(1))));
                    for r in rows { out.push_str(&format!("| {} |\n", r.join(" | "))); }
                    out.push('\n');
                }
                BlockKind::Plot2D(spec) => out.push_str(&format!("*Plot: {}*\n\n", spec.title)),
                BlockKind::Plot3D(spec) => out.push_str(&format!("*3D view: {}*\n\n", spec.title)),
                BlockKind::Control(c) => out.push_str(&format!("*Parameter `{}`*\n\n", c.variable)),
                BlockKind::Image { path, caption } => out.push_str(&format!("![{caption}]({path})\n\n")),
            }
        }
    }
    out.push_str("## Generated Python\n\n```python\n");
    out.push_str(input.python);
    out.push_str("\n```\n");
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn typst_esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}
