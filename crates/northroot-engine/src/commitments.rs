use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256:{:x}", h.finalize())
}

pub fn jcs(value: &Value) -> String {
    fn sort(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut bm = BTreeMap::new();
                for (k, v) in m {
                    bm.insert(k.clone(), sort(v));
                }
                // Convert BTreeMap to serde_json::Map
                let json_map: serde_json::Map<String, Value> = bm.into_iter().collect();
                Value::Object(json_map)
            }
            Value::Array(a) => Value::Array(a.iter().map(sort).collect()),
            _ => v.clone(),
        }
    }
    serde_json::to_string(&sort(value)).unwrap()
}

pub fn commit_set_root(parts: &[String]) -> String {
    let mut v = parts.to_vec();
    v.sort();
    sha256_prefixed(v.join("|").as_bytes())
}

pub fn commit_seq_root(parts: &[String]) -> String {
    sha256_prefixed(parts.join("|").as_bytes())
}
