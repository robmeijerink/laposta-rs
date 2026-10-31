use serde_json::Value;
use url::form_urlencoded;

/// Encodes a JSON object as `application/x-www-form-urlencoded`, using the
/// bracket notation the Laposta API expects for nested values.
pub(crate) fn encode(value: &Value) -> String {
    let mut pairs = Vec::new();
    if let Value::Object(map) = value {
        for (key, value) in map {
            flatten(key, value, &mut pairs);
        }
    }
    let mut out = form_urlencoded::Serializer::new(String::new());
    out.extend_pairs(pairs);
    out.finish()
}

fn flatten(key: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Null => {}
        Value::Bool(b) => out.push((key.to_owned(), b.to_string())),
        Value::Number(n) => out.push((key.to_owned(), n.to_string())),
        Value::String(s) => out.push((key.to_owned(), s.clone())),
        Value::Array(items) if items.is_empty() => out.push((key.to_owned(), String::new())),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                match item {
                    Value::Object(_) | Value::Array(_) => {
                        flatten(&format!("{key}[{i}]"), item, out)
                    }
                    _ => flatten(&format!("{key}[]"), item, out),
                }
            }
        }
        Value::Object(map) => {
            for (name, item) in map {
                flatten(&format!("{key}[{name}]"), item, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn decoded(value: Value) -> Vec<(String, String)> {
        form_urlencoded::parse(encode(&value).as_bytes())
            .into_owned()
            .collect()
    }

    fn pair(k: &str, v: &str) -> (String, String) {
        (k.to_owned(), v.to_owned())
    }

    #[test]
    fn scalars() {
        assert_eq!(
            decoded(json!({"name": "A & B", "locked": true, "n": 3})),
            vec![
                pair("locked", "true"),
                pair("n", "3"),
                pair("name", "A & B")
            ]
        );
    }

    #[test]
    fn nested_objects_use_brackets() {
        assert_eq!(
            decoded(json!({"from": {"name": "Max", "email": "max@example.net"}})),
            vec![
                pair("from[email]", "max@example.net"),
                pair("from[name]", "Max")
            ]
        );
    }

    #[test]
    fn arrays_of_scalars() {
        assert_eq!(
            decoded(json!({"custom_fields": {"prefs": ["a", "b"]}})),
            vec![
                pair("custom_fields[prefs][]", "a"),
                pair("custom_fields[prefs][]", "b")
            ]
        );
    }

    #[test]
    fn empty_array_clears_value() {
        assert_eq!(
            decoded(json!({"custom_fields": {"labels": []}})),
            vec![pair("custom_fields[labels]", "")]
        );
    }

    #[test]
    fn nulls_are_skipped_and_empty_strings_kept() {
        assert_eq!(
            decoded(json!({"a": null, "previewtext": ""})),
            vec![pair("previewtext", "")]
        );
    }

    #[test]
    fn arrays_of_objects_are_indexed() {
        assert_eq!(
            decoded(json!({"x": [{"id": 1}]})),
            vec![pair("x[0][id]", "1")]
        );
    }
}
