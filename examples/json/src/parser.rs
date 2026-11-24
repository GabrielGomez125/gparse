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
    everything_but("\"")
        .map(JSONValue::String)
        .padded(char('"'))
        .parse(input)
}

pub fn json_array(input: &str) -> gparse::Result<'_, JSONValue> {
    json_value
        .separated(
            manyspace,
            json_value
                .delimited(char(',').then(manyspace), manyspace)
                .many(),
        )
        .optional()
        .delimited((char('[')).padded(manyspace), char(']').then(manyspace))
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
        .map(|value| value.try_into().unwrap())
        .separated(char(':').padded(manyspace), json_value)
        .parse(input)
}

pub fn json_object(input: &str) -> gparse::Result<'_, JSONValue> {
    json_member
        .separated(
            manyspace,
            json_member
                .delimited(char(',').then(manyspace), manyspace)
                .many(),
        )
        .optional()
        .delimited(char('{').padded(manyspace), char('}').then(manyspace))
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
