//! Number formatting kept local so the unit engine has no AST dependency.
pub fn format_number(v: f64) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let a = v.abs();
    if a >= 1e6 || a < 1e-4 {
        let s = format!("{:.5e}", v);
        let mut parts = s.split('e');
        let mant = parts.next().unwrap_or("0").trim_end_matches('0').trim_end_matches('.').to_string();
        let exp: i32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        return format!("{mant}e{exp}");
    }
    let decimals = (6.0 - a.log10().floor() - 1.0).max(0.0).min(12.0) as usize;
    let s = format!("{:.*}", decimals, v);
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}
