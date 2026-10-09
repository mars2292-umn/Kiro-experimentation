//! A JSON Schema validator for the subset the rsk schemas use (R25.1,
//! R25.6, R28.1): `type`, `properties`, `required`, `additionalProperties:
//! false`, `items`, `enum`, `const`, `minimum`, `maximum`, `minItems`,
//! `pattern` (as a literal-prefix check), `oneOf` over the above, and
//! `$ref` to `$defs`. Each violation carries the JSON path of the value
//! and the rule that failed. Draft 2020-12 keywords outside this subset
//! are not supported and make the schema itself invalid, so a schema
//! author cannot write a rule that the validator silently ignores.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    /// JSON pointer-like path of the offending value (`/tasks/0/period_ticks`).
    pub path: String,
    /// The schema rule that failed, in words.
    pub rule: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", if self.path.is_empty() { "/" } else { &self.path }, self.rule)
    }
}

const KNOWN: &[&str] = &[
    "$schema",
    "$id",
    "$defs",
    "$ref",
    "title",
    "description",
    "type",
    "properties",
    "required",
    "additionalProperties",
    "items",
    "enum",
    "const",
    "minimum",
    "maximum",
    "minItems",
    "maxItems",
    "pattern",
    "oneOf",
    "examples",
];

/// Validates `value` against `schema`; the empty list means conformance.
pub fn validate(schema: &Value, value: &Value) -> Vec<Violation> {
    let mut out = Vec::new();
    check(schema, schema, value, "", &mut out);
    out
}

fn push(out: &mut Vec<Violation>, path: &str, rule: impl Into<String>) {
    out.push(Violation {
        path: path.to_string(),
        rule: rule.into(),
    });
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    if let Some(Value::String(r)) = schema.get("$ref") {
        if let Some(name) = r.strip_prefix("#/$defs/") {
            if let Some(def) = root.get("$defs").and_then(|d| d.get(name)) {
                return def;
            }
        }
    }
    schema
}

fn check(root: &Value, schema: &Value, value: &Value, path: &str, out: &mut Vec<Violation>) {
    let schema = resolve(root, schema);
    let Some(obj) = schema.as_object() else {
        if schema == &Value::Bool(true) {
            return;
        }
        push(out, path, "schema is not an object");
        return;
    };
    for key in obj.keys() {
        if !KNOWN.contains(&key.as_str()) {
            push(out, path, format!("schema uses the unsupported keyword `{key}`"));
        }
    }
    if let Some(one_of) = obj.get("oneOf").and_then(Value::as_array) {
        let matching = one_of.iter().filter(|s| {
            let mut sub = Vec::new();
            check(root, s, value, path, &mut sub);
            sub.is_empty()
        }).count();
        if matching != 1 {
            push(out, path, format!("oneOf: {matching} alternatives match, exactly one must"));
        }
    }
    if let Some(t) = obj.get("type") {
        let allowed: Vec<&str> = match t {
            Value::String(s) => vec![s.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        let actual = type_name(value);
        let ok = allowed.iter().any(|&a| a == actual || (a == "number" && actual == "integer"));
        if !ok {
            push(out, path, format!("type: expected {}, found {actual}", allowed.join(" or ")));
            return;
        }
    }
    if let Some(e) = obj.get("enum").and_then(Value::as_array) {
        if !e.contains(value) {
            push(out, path, format!("enum: value is not one of {}", serde_json::to_string(e).unwrap_or_default()));
        }
    }
    if let Some(c) = obj.get("const") {
        if c != value {
            push(out, path, format!("const: expected {}", serde_json::to_string(c).unwrap_or_default()));
        }
    }
    if let Some(n) = value.as_f64() {
        if let Some(min) = obj.get("minimum").and_then(Value::as_f64) {
            if n < min {
                push(out, path, format!("minimum: {n} < {min}"));
            }
        }
        if let Some(max) = obj.get("maximum").and_then(Value::as_f64) {
            if n > max {
                push(out, path, format!("maximum: {n} > {max}"));
            }
        }
    }
    if let (Some(s), Some(pat)) = (value.as_str(), obj.get("pattern").and_then(Value::as_str)) {
        // Supported form: `^literal` (a required prefix) or `^literal$` (exact).
        let body = pat.trim_start_matches('^');
        let ok = if let Some(exact) = body.strip_suffix('$') { s == exact } else { s.starts_with(body) };
        if !ok {
            push(out, path, format!("pattern: `{s}` does not match `{pat}`"));
        }
    }
    if let Some(arr) = value.as_array() {
        if let Some(min) = obj.get("minItems").and_then(Value::as_u64) {
            if (arr.len() as u64) < min {
                push(out, path, format!("minItems: {} < {min}", arr.len()));
            }
        }
        if let Some(max) = obj.get("maxItems").and_then(Value::as_u64) {
            if (arr.len() as u64) > max {
                push(out, path, format!("maxItems: {} > {max}", arr.len()));
            }
        }
        if let Some(items) = obj.get("items") {
            for (i, item) in arr.iter().enumerate() {
                check(root, items, item, &format!("{path}/{i}"), out);
            }
        }
    }
    if let Some(map) = value.as_object() {
        let props = obj.get("properties").and_then(Value::as_object);
        if let Some(required) = obj.get("required").and_then(Value::as_array) {
            for r in required.iter().filter_map(Value::as_str) {
                if !map.contains_key(r) {
                    push(out, path, format!("required: missing property `{r}`"));
                }
            }
        }
        if let Some(props) = props {
            for (k, sub) in props {
                if let Some(v) = map.get(k) {
                    check(root, sub, v, &format!("{path}/{k}"), out);
                }
            }
        }
        if obj.get("additionalProperties") == Some(&Value::Bool(false)) {
            for k in map.keys() {
                if !props.is_some_and(|p| p.contains_key(k)) {
                    push(out, &format!("{path}/{k}"), "additionalProperties: unknown property");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reports_each_violation_with_its_path() {
        let schema = json!({
            "type": "object",
            "required": ["a", "b"],
            "additionalProperties": false,
            "properties": {
                "a": {"type": "integer", "minimum": 1},
                "b": {"type": "array", "items": {"$ref": "#/$defs/item"}},
                "k": {"enum": ["x", "y"]}
            },
            "$defs": {"item": {"type": "string", "pattern": "^id:"}}
        });
        let v = json!({"a": 0, "b": ["id:1", "nope", 3], "k": "z", "extra": 1});
        let errs = validate(&schema, &v);
        let paths: Vec<&str> = errs.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["/a", "/b/1", "/b/2", "/k", "/extra"], "{errs:?}");
        assert!(validate(&schema, &json!({"a": 1, "b": []})).is_empty());
        assert_eq!(validate(&schema, &json!({"a": 1})).len(), 1);
    }

    #[test]
    fn unsupported_keywords_are_reported() {
        let schema = json!({"type": "object", "patternProperties": {}});
        assert_eq!(validate(&schema, &json!({})).len(), 1);
    }
}
