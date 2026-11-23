#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Tauri shell: document state, undo, and the two-way canvas ⇄ Python bridge.
mod latex;
mod math;

use math::{eval, Env, Stmt};
use serde::Serialize;
use std::sync::Mutex;

#[derive(Clone)]
struct Region { id: u32, latex: String, x: f32, y: f32 }

#[derive(Clone, Default)]
struct Doc { regions: Vec<Region>, next: u32 }

struct App { doc: Doc, past: Vec<Doc>, future: Vec<Doc> }

#[derive(Serialize)]
struct RegionDto {
    id: u32, latex: String, python: String, x: f32, y: f32,
    value: Option<String>, error: Option<String>, assigns: bool,
}

#[derive(Serialize)]
struct StateDto { regions: Vec<RegionDto>, python: String, errors: usize, ms: f64 }

fn build(doc: &Doc) -> StateDto {
    let t0 = std::time::Instant::now();
    let mut env = Env::new();
    let mut lines = Vec::new();
    let mut errors = 0;
    let mut out = Vec::new();
    for r in &doc.regions {
        let parsed = latex::parse_latex(&r.latex);
        let (stmt, mut err): (Option<Stmt>, Option<String>) = match parsed {
            Ok(s) => (s, None),
            Err(e) if e == "incomplete" => (None, None),
            Err(e) => (None, Some(e)),
        };
        let mut value = None;
        let mut python = String::new();
        let mut assigns = false;
        if let Some(s) = &stmt {
            python = math::to_python(s);
            assigns = s.name.is_some();
            if err.is_none() {
                match eval(&s.expr, &mut env) {
                    Ok(v) => {
                        if let Some(n) = &s.name { env.insert(n.clone(), v); }
                        else { value = Some(math::fmt_num(v)); }
                    }
                    Err(e) => err = Some(e),
                }
            }
        }
        if err.is_some() { errors += 1; }
        lines.push(python.clone());
        out.push(RegionDto { id: r.id, latex: r.latex.clone(), python, x: r.x, y: r.y,
                             value, error: err, assigns });
    }
    StateDto { regions: out, python: lines.join("\n"), errors, ms: t0.elapsed().as_secs_f64() * 1000.0 }
}

fn snapshot(a: &App) -> StateDto { build(&a.doc) }

fn record(a: &mut App) {
    a.past.push(a.doc.clone());
    if a.past.len() > 100 { a.past.remove(0); }
    a.future.clear();
}

type S<'a> = tauri::State<'a, Mutex<App>>;

#[tauri::command] fn state(app: S) -> StateDto { snapshot(&app.lock().unwrap()) }

#[tauri::command]
fn set_latex(app: S, id: u32, latex: String) -> StateDto {
    let mut a = app.lock().unwrap();
    record(&mut a);
    if let Some(r) = a.doc.regions.iter_mut().find(|r| r.id == id) { r.latex = latex; }
    snapshot(&a)
}

#[tauri::command]
fn add_region(app: S, latex: String, x: f32, y: f32) -> StateDto {
    let mut a = app.lock().unwrap();
    record(&mut a);
    let id = a.doc.next; a.doc.next += 1;
    a.doc.regions.push(Region { id, latex, x, y });
    snapshot(&a)
}

#[tauri::command]
fn remove_region(app: S, id: u32) -> StateDto {
    let mut a = app.lock().unwrap();
    record(&mut a);
    a.doc.regions.retain(|r| r.id != id);
    snapshot(&a)
}

#[tauri::command]
fn move_region(app: S, id: u32, x: f32, y: f32) -> StateDto {
    let mut a = app.lock().unwrap();
    if let Some(r) = a.doc.regions.iter_mut().find(|r| r.id == id) { r.x = x; r.y = y; }
    snapshot(&a)
}

/// Python pane edited: one line becomes one region, positions kept by index.
#[tauri::command]
fn set_python(app: S, text: String) -> StateDto {
    let mut a = app.lock().unwrap();
    record(&mut a);
    let old = a.doc.regions.clone();
    let mut next = Vec::new();
    let mut id = a.doc.next;
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() { continue; }
        let latex = match math::parse_python(line) {
            Ok(Some(s)) => latex::to_latex(&s),
            _ => line.trim().to_string(),
        };
        let (rid, x, y) = match old.get(i) {
            Some(r) => (r.id, r.x, r.y),
            None => { let v = id; id += 1; (v, 60.0, 60.0 + i as f32 * 64.0) }
        };
        next.push(Region { id: rid, latex, x, y });
    }
    a.doc.next = id.max(a.doc.next);
    a.doc.regions = next;
    snapshot(&a)
}

#[tauri::command]
fn undo(app: S) -> StateDto {
    let mut a = app.lock().unwrap();
    let popped = a.past.pop();
    if let Some(d) = popped {
        let cur = a.doc.clone();
        a.future.push(cur);
        a.doc = d;
    }
    snapshot(&a)
}

#[tauri::command]
fn redo(app: S) -> StateDto {
    let mut a = app.lock().unwrap();
    let popped = a.future.pop();
    if let Some(d) = popped {
        let cur = a.doc.clone();
        a.past.push(cur);
        a.doc = d;
    }
    snapshot(&a)
}

#[tauri::command]
fn script(app: S) -> String {
    let a = app.lock().unwrap();
    format!("{}{}\n", math::PRELUDE, build(&a.doc).python)
}

fn main() {
    let mut doc = Doc { regions: Vec::new(), next: 0 };
    for (i, l) in ["x:= 3", "\\frac{1}{2}+\\frac{1}{3}", "\\int_{0}^{1}x^{2}\\,dx"].iter().enumerate() {
        doc.regions.push(Region { id: doc.next, latex: l.to_string(), x: 60.0, y: 48.0 + i as f32 * 76.0 });
        doc.next += 1;
    }
    tauri::Builder::default()
        .manage(Mutex::new(App { doc, past: Vec::new(), future: Vec::new() }))
        .invoke_handler(tauri::generate_handler![
            state, set_latex, add_region, remove_region, move_region, set_python, undo, redo, script
        ])
        .run(tauri::generate_context!())
        .expect("failed to start AaiyyanMath");
}
