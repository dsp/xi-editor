use serde_json::Value;

pub fn value_to_string(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}
