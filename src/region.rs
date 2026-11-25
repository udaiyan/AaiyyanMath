use serde::{Serialize, Deserialize};
use crate::expression::Expression;
use crate::plot::PlotPane;

#[derive(Serialize, Deserialize)]
pub enum Region {
    Text(String),
    Math(Expression),
    Plot(PlotPane),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Dim;

    #[test]
    fn test_region_evaluation_chain() {
        let mut regions = vec![
            Region::Math(Expression::new("a", "5 m")),
            Region::Text("This is a comment".to_string()),
            Region::Math(Expression::new("b", "a * 2")),
        ];

        let mut bindings = std::collections::HashMap::new();
        for region in &mut regions {
            if let Region::Math(expr) = region {
                let snapshot = bindings.clone();
                expr.evaluate(&snapshot);
                if let Some(q) = &expr.result {
                    bindings.insert(expr.label.clone(), q.clone());
                }
            }
        }

        assert_eq!(bindings.get("a").unwrap().value, 5.0);
        assert_eq!(bindings.get("b").unwrap().value, 10.0);
        assert_eq!(bindings.get("b").unwrap().dim, Dim::LENGTH);
    }
}