//! Worksheet document model: independently editable blocks (spec section 19).
use aaiyyan_plotting::{Plot2DSpec, Plot3DSpec};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ControlKind {
    Slider { min: f64, max: f64, step: f64, value: f64 },
    Number { value: f64 },
    Checkbox { value: bool },
    Dropdown { options: Vec<String>, selected: usize },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlSpec {
    pub variable: String,
    pub label: String,
    pub kind: ControlKind,
}

impl ControlSpec {
    pub fn slider(variable: &str, min: f64, max: f64) -> ControlSpec {
        ControlSpec {
            variable: variable.into(),
            label: variable.into(),
            kind: ControlKind::Slider { min, max, step: (max - min) / 100.0, value: min },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BlockKind {
    Equation { source: String },
    Result { source: String },
    Text { text: String },
    Heading { level: u8, text: String },
    Python { source: String, trusted: bool },
    Plot2D(Plot2DSpec),
    Plot3D(Plot3DSpec),
    Table { columns: Vec<String>, rows: Vec<Vec<String>> },
    Control(ControlSpec),
    Image { path: String, caption: String },
}

impl BlockKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            BlockKind::Equation { .. } => "equation",
            BlockKind::Result { .. } => "result",
            BlockKind::Text { .. } => "text",
            BlockKind::Heading { .. } => "heading",
            BlockKind::Python { .. } => "python",
            BlockKind::Plot2D(_) => "2D plot",
            BlockKind::Plot3D(_) => "3D plot",
            BlockKind::Table { .. } => "table",
            BlockKind::Control(_) => "control",
            BlockKind::Image { .. } => "image",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BlockKind::Equation { .. } => "f(x)",
            BlockKind::Result { .. } => "=",
            BlockKind::Text { .. } => "T",
            BlockKind::Heading { .. } => "H",
            BlockKind::Python { .. } => "py",
            BlockKind::Plot2D(_) => "2D",
            BlockKind::Plot3D(_) => "3D",
            BlockKind::Table { .. } => "#",
            BlockKind::Control(_) => "<>",
            BlockKind::Image { .. } => "img",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub kind: BlockKind,
    pub enabled: bool,
    /// Canvas position in worksheet units (Mathcad-style free placement).
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
}

impl Block {
    pub fn new(kind: BlockKind) -> Block { Block { id: Uuid::new_v4(), kind, enabled: true, x: 0.0, y: 0.0 } }
    pub fn equation(source: &str) -> Block { Block::new(BlockKind::Equation { source: source.into() }) }
    pub fn text(text: &str) -> Block { Block::new(BlockKind::Text { text: text.into() }) }
    pub fn heading(level: u8, text: &str) -> Block { Block::new(BlockKind::Heading { level, text: text.into() }) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Worksheet {
    pub id: Uuid,
    pub name: String,
    pub blocks: Vec<Block>,
}

impl Worksheet {
    pub fn new(name: &str) -> Worksheet {
        Worksheet { id: Uuid::new_v4(), name: name.into(), blocks: Vec::new() }
    }

    pub fn index_of(&self, id: Uuid) -> Option<usize> { self.blocks.iter().position(|b| b.id == id) }
    pub fn block(&self, id: Uuid) -> Option<&Block> { self.blocks.iter().find(|b| b.id == id) }
    pub fn block_mut(&mut self, id: Uuid) -> Option<&mut Block> { self.blocks.iter_mut().find(|b| b.id == id) }

    pub fn insert(&mut self, index: Option<usize>, block: Block) {
        match index {
            Some(i) if i <= self.blocks.len() => self.blocks.insert(i, block),
            _ => self.blocks.push(block),
        }
    }

    pub fn remove(&mut self, id: Uuid) -> Option<Block> {
        self.index_of(id).map(|i| self.blocks.remove(i))
    }

    pub fn move_block(&mut self, id: Uuid, delta: i32) -> bool {
        if let Some(i) = self.index_of(id) {
            let target = i as i32 + delta;
            if target < 0 || target as usize >= self.blocks.len() { return false; }
            let b = self.blocks.remove(i);
            self.blocks.insert(target as usize, b);
            return true;
        }
        false
    }
}

/// Snapshot-based undo/redo. Adequate for document-sized worksheets and simple
/// to keep correct; a command/diff model can replace it later.
pub struct History {
    past: Vec<Worksheet>,
    future: Vec<Worksheet>,
    limit: usize,
}

impl History {
    pub fn new(limit: usize) -> History { History { past: Vec::new(), future: Vec::new(), limit } }

    pub fn record(&mut self, snapshot: &Worksheet) {
        self.past.push(snapshot.clone());
        if self.past.len() > self.limit { self.past.remove(0); }
        self.future.clear();
    }

    pub fn undo(&mut self, current: &Worksheet) -> Option<Worksheet> {
        let prev = self.past.pop()?;
        self.future.push(current.clone());
        Some(prev)
    }

    pub fn redo(&mut self, current: &Worksheet) -> Option<Worksheet> {
        let next = self.future.pop()?;
        self.past.push(current.clone());
        Some(next)
    }

    pub fn can_undo(&self) -> bool { !self.past.is_empty() }
    pub fn can_redo(&self) -> bool { !self.future.is_empty() }
    pub fn clear(&mut self) { self.past.clear(); self.future.clear(); }
}
