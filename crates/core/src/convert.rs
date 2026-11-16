//! Rust <-> Python value interchange. SI base value + unit descriptor is the
//! canonical representation on both sides (spec sections 8 and 10).
use aaiyyan_evaluator::{Cell, Matrix, Table, Value};
use aaiyyan_python_runtime::ValueDto;
use aaiyyan_units::{parse_unit_string, Quantity, Unit};

pub fn dto_to_value(dto: &ValueDto) -> Option<Value> {
    match dto.kind.as_str() {
        "scalar" | "number" | "int" | "float" => dto.number.map(Value::Scalar),
        "bool" => dto.number.map(|v| Value::Bool(v != 0.0)),
        "str" | "text" => Some(Value::Str(dto.text.clone().unwrap_or_else(|| dto.display.clone()))),
        "quantity" => {
            let si = dto.si.or(dto.number)?;
            let unit = match dto.unit.as_deref() {
                Some(name) if !name.is_empty() => parse_unit_string(name).unwrap_or_else(|_| Unit::dimensionless()),
                _ => Unit::dimensionless(),
            };
            Some(Value::Quantity(Quantity { si, unit }))
        }
        "complex" => {
            let re = dto.number.unwrap_or(0.0);
            let im = dto.data.first().copied().unwrap_or(0.0);
            Some(Value::Complex { re, im })
        }
        "vector" => Some(Value::Vector(dto.data.clone())),
        "matrix" => {
            if dto.shape.len() == 2 {
                Some(Value::Matrix(Matrix::new(dto.shape[0], dto.shape[1], dto.data.clone())))
            } else {
                Some(Value::Vector(dto.data.clone()))
            }
        }
        "symbolic" => Some(Value::Symbolic {
            expr: dto.text.clone().unwrap_or_else(|| dto.display.clone()),
            latex: dto.latex.clone().unwrap_or_default(),
        }),
        "table" => Some(Value::Table(Table {
            columns: dto.columns.clone(),
            rows: dto.rows.iter().map(|r| r.iter().map(|c| match c.parse::<f64>() {
                Ok(v) => Cell::Num(v),
                Err(_) => Cell::Text(c.clone()),
            }).collect()).collect(),
        })),
        "function" => None,
        _ => dto.number.map(Value::Scalar),
    }
}
