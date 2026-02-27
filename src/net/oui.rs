use std::collections::HashMap;
use std::sync::OnceLock;

/// Embedded IEEE OUI database (~38K entries)
const OUI_CSV: &str = include_str!("oui.csv");

/// Global OUI prefix map (loaded once on first lookup)
fn oui_map() -> &'static HashMap<String, String> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();
        let mut first = true;
        for line in OUI_CSV.lines() {
            // Skip header row
            if first {
                first = false;
                continue;
            }
            // CSV format: Registry,Assignment,Organization Name,Organization Address
            // e.g.: MA-L,CCBABD,TP-Link Systems Inc.,"10 Mauchly ..."
            if let Some((prefix, org)) = parse_oui_line(line) {
                map.entry(prefix).or_insert(org);
            }
        }
        map
    })
}

/// Parse a single CSV line, handling quoted fields
fn parse_oui_line(line: &str) -> Option<(String, String)> {
    // Split carefully — org name or address may contain commas inside quotes
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);

    if fields.len() < 3 {
        return None;
    }

    let mac_field = fields[1].trim().to_uppercase();
    let org = fields[2].trim().to_string();

    if mac_field.is_empty() || org.is_empty() {
        return None;
    }

    // Normalize to 6-char hex prefix
    let prefix: String = mac_field
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(6)
        .collect();

    if prefix.len() < 6 {
        return None;
    }

    Some((prefix, org))
}

/// Normalize a MAC address to a 6-character uppercase hex prefix for lookup
fn normalize_mac_prefix(mac: &str) -> Option<String> {
    let prefix: String = mac
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(6)
        .collect();

    if prefix.len() == 6 {
        Some(prefix)
    } else {
        None
    }
}

/// Look up a manufacturer name from a MAC address using the IEEE OUI database.
pub fn lookup_manufacturer(mac: &str) -> Option<String> {
    let prefix = normalize_mac_prefix(mac)?;
    oui_map().get(&prefix).cloned()
}
