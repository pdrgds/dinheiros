pub mod history;
pub mod import;
pub mod income;
pub mod manual;
pub mod overview;
pub mod positions;
pub mod settings;

/// Format a float as Brazilian Real: dots for thousands, comma for decimals.
/// E.g. 142387.52 → "142.387,52"
pub fn format_brl(v: f64) -> String {
    let abs = v.abs();
    let sign = if v < 0.0 { "-" } else { "" };

    let integer = abs.trunc() as u64;
    let frac = ((abs - abs.trunc()) * 100.0).round() as u64;

    // Format integer part with dot separators
    let int_str = integer.to_string();
    let mut grouped = String::new();
    for (i, ch) in int_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();

    format!("{sign}{grouped},{frac:02}")
}
