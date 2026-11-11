use std::collections::HashMap;

use crate::{Parser, numbers::number_scientific, strings::literal};

#[derive(Clone)]
pub enum JSONValue {
    Null,
    Boolean(bool),
    Number(f64),
    Array(Vec<JSONValue>),
    String(String),
    Object(HashMap<JSONValue, JSONValue>),
}

pub fn json_value() -> impl Parser<Output = JSONValue> {
    json_null().or(json_boolean()).or(json_number())
}

pub fn json_null() -> impl Parser<Output = JSONValue> {
    literal("null").value(JSONValue::Null)
}

pub fn json_boolean() -> impl Parser<Output = JSONValue> {
    literal("true")
        .value(JSONValue::Boolean(true))
        .or(literal("false").value(JSONValue::Boolean(false)))
}

pub fn json_number() -> impl Parser<Output = JSONValue> {
    number_scientific().map(JSONValue::Number)
}
