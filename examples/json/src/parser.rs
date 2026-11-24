use std::collections::HashMap;

use gparse::{
    chars::char,
    numbers::number_scientific,
    strings::{literal, manyspace, take_while},
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

pub fn json<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    json_value.padded(manyspace).parse(input)
}

pub fn json_value<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    json_null
        .or(json_boolean)
        .or(json_number)
        .or(json_string)
        .or(json_array)
        .or(json_object)
        .parse(input)
}

pub fn json_null<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    literal("null").value(JSONValue::Null).parse(input)
}

pub fn json_boolean<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    literal("true")
        .value(JSONValue::Boolean(true))
        .or(literal("false").value(JSONValue::Boolean(false)))
        .parse(input)
}

pub fn json_number<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    number_scientific.map(JSONValue::Number).parse(input)
}

pub fn json_string<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    take_while(|c: char| c != '"')
        .map(JSONValue::String)
        .padded(char('"'))
        .parse(input)
}

pub fn json_array<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    json_value
        .then_ignore(manyspace)
        .then(
            json_value
                .delimited(char(',').then(manyspace), manyspace)
                .many(),
        )
        .optional()
        .delimited(
            (char('[')).padded(manyspace),
            char(']').then(manyspace),
        )
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

pub fn json_member<'a>(input: &'a str) -> gparse::Result<'a, (String, JSONValue)> {
    json_string
        .map(|value| value.try_into().unwrap())
        .then_ignore(char(':').padded(manyspace))
        .then(json_value)
        .parse(input)
}

pub fn json_object<'a>(input: &'a str) -> gparse::Result<'a, JSONValue> {
    json_member
        .then_ignore(manyspace)
        .then(
            json_member
                .delimited(char(',').then(manyspace), manyspace)
                .many(),
        )
        .optional()
        .delimited(
            char('{').padded(manyspace),
            char('}').then(manyspace),
        )
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
