//! A canonical JSON writer (R25.4, R24.4): objects keep the key order the
//! caller builds them in (the printer builds every object in the schema's
//! canonical order), numbers are integers, strings are escaped per RFC
//! 8259, and the output is pretty-printed with two-space indentation so
//! that equal models give byte-identical documents.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(u64),
    Str(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn obj() -> Value {
        Value::Object(Vec::new())
    }

    /// Appends a key (objects are built in canonical order by the caller).
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Value {
        if let Value::Object(entries) = &mut self {
            entries.push((key.to_string(), value.into()));
        }
        self
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Int(n) => out.push_str(&n.to_string()),
            Value::Str(s) => escape(s, out),
            Value::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    pad(out, indent + 1);
                    item.write(out, indent + 1);
                }
                out.push('\n');
                pad(out, indent);
                out.push(']');
            }
            Value::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    pad(out, indent + 1);
                    escape(k, out);
                    out.push_str(": ");
                    v.write(out, indent + 1);
                }
                out.push('\n');
                pad(out, indent);
                out.push('}');
            }
        }
    }
}

fn pad(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}
impl From<u64> for Value {
    fn from(n: u64) -> Value {
        Value::Int(n)
    }
}
impl From<u32> for Value {
    fn from(n: u32) -> Value {
        Value::Int(n as u64)
    }
}
impl From<u16> for Value {
    fn from(n: u16) -> Value {
        Value::Int(n as u64)
    }
}
impl From<u8> for Value {
    fn from(n: u8) -> Value {
        Value::Int(n as u64)
    }
}
impl From<usize> for Value {
    fn from(n: usize) -> Value {
        Value::Int(n as u64)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::Str(s.to_string())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Str(s)
    }
}
impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Value {
        Value::Array(v)
    }
}
impl From<Vec<String>> for Value {
    fn from(v: Vec<String>) -> Value {
        Value::Array(v.into_iter().map(Value::Str).collect())
    }
}
impl From<Vec<u64>> for Value {
    fn from(v: Vec<u64>) -> Value {
        Value::Array(v.into_iter().map(Value::Int).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_canonically() {
        let v = Value::obj()
            .with("b", 1u64)
            .with("a", "x\"y")
            .with("list", vec![Value::Int(1), Value::Bool(false)])
            .with("empty", Vec::<Value>::new());
        assert_eq!(
            v.render(),
            "{\n  \"b\": 1,\n  \"a\": \"x\\\"y\",\n  \"list\": [\n    1,\n    false\n  ],\n  \"empty\": []\n}\n"
        );
    }
}
