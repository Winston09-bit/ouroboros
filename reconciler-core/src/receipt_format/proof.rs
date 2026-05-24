use serde_json::Value;
use sha2::{Digest, Sha256};

/// Canonicalize JSON för deterministisk hashing (RFC 8785 förenklad).
/// Sorterar objektnycklar alfabetiskt rekursivt och producerar kompakt JSON utan whitespace.
pub fn canonicalize_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            // Escape according to JSON spec
            let mut out = String::with_capacity(s.len() + 2);
            out.push('"');
            for ch in s.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => {
                        out.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(canonicalize_json).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(map) => {
            // Sort keys alphabetically (BTreeMap guarantees insertion order; serde_json::Map is ordered by insertion,
            // so we must collect + sort manually)
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
            let entries: Vec<String> = pairs
                .into_iter()
                .map(|(k, v)| {
                    let key_str = canonicalize_json(&Value::String(k.clone()));
                    let val_str = canonicalize_json(v);
                    format!("{}:{}", key_str, val_str)
                })
                .collect();
            format!("{{{}}}", entries.join(","))
        }
    }
}

/// Beräkna SHA-256 hash av canonical JSON (returnerar hex-sträng).
pub fn canonical_hash(value: &Value) -> String {
    let canonical = canonicalize_json(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect::<String>()
}

/// Räkna ut canonical hash av en VerifiedReceipt UTAN proof-fältet.
pub fn receipt_canonical_hash(receipt: &super::schema::VerifiedReceipt) -> String {
    // Serialize to Value, strip proof field, then canonicalize + hash
    let mut value = serde_json::to_value(receipt).expect("receipt serialization failed");
    if let Value::Object(ref mut map) = value {
        map.remove("proof");
    }
    canonical_hash(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_canonicalize_sorts_keys() {
        let v = json!({"z": 1, "a": 2, "m": 3});
        let canon = canonicalize_json(&v);
        assert_eq!(canon, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn test_canonicalize_nested() {
        let v = json!({"b": {"z": true, "a": false}, "a": [3, 1, 2]});
        let canon = canonicalize_json(&v);
        assert_eq!(canon, r#"{"a":[3,1,2],"b":{"a":false,"z":true}}"#);
    }

    #[test]
    fn test_canonical_hash_deterministic() {
        let v = json!({"amount": "100.00", "currency": "SEK"});
        let h1 = canonical_hash(&v);
        let h2 = canonical_hash(&v);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_canonical_hash_key_order_invariant() {
        let v1 = json!({"b": 2, "a": 1});
        let v2 = json!({"a": 1, "b": 2});
        assert_eq!(canonical_hash(&v1), canonical_hash(&v2));
    }
}
