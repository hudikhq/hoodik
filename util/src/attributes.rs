//! Field projection for listing endpoints.
//!
//! Listings default to full rows so older clients keep working. A client
//! that wants a leaner payload passes `attributes` — a comma-separated
//! whitelist of top-level fields — and every row is trimmed to those
//! keys. Unknown names are ignored, so a client may request fields that
//! only newer servers emit.

use std::collections::HashSet;

/// Parse the comma-separated `attributes` value into a key set.
///
/// Returns `None` when the parameter is absent or names no keys — the
/// caller serializes rows untouched.
pub fn parse(attributes: Option<&str>) -> Option<HashSet<String>> {
    let keys: HashSet<String> = attributes?
        .split(',')
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
        .collect();

    (!keys.is_empty()).then_some(keys)
}

/// Retain only the requested keys on every object in a JSON array of
/// rows. Non-array values and non-object rows are left untouched.
pub fn project_rows(rows: &mut serde_json::Value, keys: &HashSet<String>) {
    if let Some(rows) = rows.as_array_mut() {
        for row in rows {
            if let Some(object) = row.as_object_mut() {
                object.retain(|key, _| keys.contains(key));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_ignores_blank_and_absent_values() {
        assert!(parse(None).is_none());
        assert!(parse(Some("")).is_none());
        assert!(parse(Some(" , ,")).is_none());

        let keys = parse(Some("id, encrypted_name ,size")).unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains("encrypted_name"));
    }

    #[test]
    fn project_rows_trims_each_object_to_requested_keys() {
        let mut rows = json!([
            { "id": 1, "encrypted_thumbnail": "blob", "size": 10 },
            { "id": 2, "size": 20 }
        ]);
        let keys = parse(Some("id,size")).unwrap();

        project_rows(&mut rows, &keys);

        assert_eq!(rows, json!([{ "id": 1, "size": 10 }, { "id": 2, "size": 20 }]));
    }
}
