//! `serde_json::Value` ⇄ `Variant` (ADR 1.10.26ab D3): objects ⇄
//! `Dictionary`, arrays ⇄ `Array`, integers ⇄ `int`, strings ⇄ `String`.
//! The one piece only the in-engine tests reach.
//!
//! One rule beyond the table: a whole `float` crosses as an integer. GDScript's
//! `JSON.parse_string` reads every number as a `float`, and K1 has no float
//! field, so `{"level": 2.0}` from a parsed case must reach K1 as `2`.

use godot::builtin::{GString, StringName, VarArray, VarDictionary, Variant, VariantType};
use godot::meta::ToGodot;
use serde_json::{Map, Number, Value};

pub fn to_variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => number(n),
        Value::String(s) => GString::from(s.as_str()).to_variant(),
        Value::Array(items) => {
            let mut array = VarArray::new();
            for item in items {
                array.push(&to_variant(item));
            }
            array.to_variant()
        }
        Value::Object(map) => to_dictionary(map).to_variant(),
    }
}

pub fn to_dictionary(map: &Map<String, Value>) -> VarDictionary {
    let mut dict = VarDictionary::new();
    for (key, value) in map {
        dict.set(&GString::from(key.as_str()).to_variant(), &to_variant(value));
    }
    dict
}

/// K1's ids are at most `i64::MAX`, so every integer K1 sends is an `int`.
fn number(n: &Number) -> Variant {
    match (n.as_i64(), n.as_f64()) {
        (Some(i), _) => i.to_variant(),
        (None, Some(f)) => f.to_variant(),
        (None, None) => Variant::nil(),
    }
}

/// A whole float in `i64`'s range is an integer; any other is a float.
fn float(f: f64) -> Value {
    let whole = f.is_finite() && f.fract() == 0.0 && f.abs() < 9.0e18;
    if whole { Value::from(f as i64) } else { Number::from_f64(f).map_or(Value::Null, Value::Number) }
}

pub fn from_variant(variant: &Variant) -> Value {
    match variant.get_type() {
        VariantType::NIL => Value::Null,
        VariantType::BOOL => Value::Bool(variant.to::<bool>()),
        VariantType::INT => Value::from(variant.to::<i64>()),
        VariantType::FLOAT => float(variant.to::<f64>()),
        VariantType::STRING => Value::from(variant.to::<GString>().to_string()),
        VariantType::STRING_NAME => Value::from(variant.to::<StringName>().to_string()),
        VariantType::DICTIONARY => from_dictionary(&variant.to::<VarDictionary>()),
        VariantType::ARRAY => Value::Array(variant.to::<VarArray>().iter_shared().map(|v| from_variant(&v)).collect()),
        // Packed arrays and the rest: as GDScript prints them.
        _ => Value::from(variant.to_string()),
    }
}

/// A GDScript string argument as JSON text.
pub fn text(s: &GString) -> Value {
    s.to_string().into()
}

/// Keys become strings, as JSON's must.
pub fn from_dictionary(dict: &VarDictionary) -> Value {
    let map = dict.iter_shared().map(|(k, v)| (key(&k), from_variant(&v))).collect();
    Value::Object(map)
}

fn key(variant: &Variant) -> String {
    match variant.get_type() {
        VariantType::STRING => variant.to::<GString>().to_string(),
        _ => variant.to_string(),
    }
}
