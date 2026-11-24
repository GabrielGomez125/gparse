use std::collections::HashMap;

use gparse::{
    chars::char,
    numbers::number_scientific,
    strings::{everything_but, literal, manyspace},
    Parser,
};

#[derive(Debug, Clone)]
pub enum JSONValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JSONValue>),
    Object(HashMap<String, JSONValue>),
}

impl TryFrom<JSONValue> for String {
    type Error = ();

    fn try_from(value: JSONValue) -> Result<Self, Self::Error> {
        match value {
            JSONValue::String(string) => Ok(string),
            _ => Err(()),
        }
    }
}

pub fn json(input: &str) -> gparse::Result<'_, JSONValue> {
    json_value.padded(manyspace).parse(input)
}

pub fn json_value(input: &str) -> gparse::Result<'_, JSONValue> {
    json_null
        .or(json_boolean)
        .or(json_number)
        .or(json_string)
        .or(json_array)
        .or(json_object)
        .parse(input)
}

pub fn json_null(input: &str) -> gparse::Result<'_, JSONValue> {
    literal("null").value(JSONValue::Null).parse(input)
}

pub fn json_boolean(input: &str) -> gparse::Result<'_, JSONValue> {
    literal("true")
        .value(JSONValue::Boolean(true))
        .or(literal("false").value(JSONValue::Boolean(false)))
        .parse(input)
}

pub fn json_number(input: &str) -> gparse::Result<'_, JSONValue> {
    number_scientific.map(JSONValue::Number).parse(input)
}

pub fn json_string(input: &str) -> gparse::Result<'_, JSONValue> {
    // Once a string is opened, it must be closed.
    char('"')
        .ignore_then(everything_but("\""))
        .then_ignore(char('"').cut())
        .map(JSONValue::String)
        .parse(input)
}

pub fn json_array(input: &str) -> gparse::Result<'_, JSONValue> {
    // A ',' must be followed by another value.
    let next = json_value
        .cut()
        .delimited(char(',').then(manyspace), manyspace);

    let body = json_value
        .separated(manyspace, next.many())
        .optional()
        .then_ignore(char(']'));

    // Once a '[' is read, this must be an array.
    char('[')
        .padded(manyspace)
        .ignore_then(body.cut())
        .then_ignore(manyspace)
        .map(|values| match values {
            Some((first, mut rest)) => {
                let mut values = vec![first];
                values.append(&mut rest);

                JSONValue::Array(values)
            }
            None => JSONValue::Array(Vec::new()),
        })
        .parse(input)
}

pub fn json_member(input: &str) -> gparse::Result<'_, (String, JSONValue)> {
    json_string
        .try_map(String::try_from)
        // A key must be followed by a ':' and a value.
        .separated(char(':').padded(manyspace).cut(), json_value.cut())
        .parse(input)
}

pub fn json_object(input: &str) -> gparse::Result<'_, JSONValue> {
    // A ',' must be followed by another member.
    let next = json_member
        .cut()
        .delimited(char(',').then(manyspace), manyspace);

    let body = json_member
        .separated(manyspace, next.many())
        .optional()
        .then_ignore(char('}'));

    // Once a '{' is read, this must be an object.
    char('{')
        .padded(manyspace)
        .ignore_then(body.cut())
        .then_ignore(manyspace)
        .map(|members| match members {
            Some(((key, value), members)) => {
                let mut object = HashMap::new();
                object.insert(key, value);

                for (key, value) in members.into_iter() {
                    object.insert(key, value);
                }

                JSONValue::Object(object)
            }

            None => JSONValue::Object(HashMap::new()),
        })
        .parse(input)
}
