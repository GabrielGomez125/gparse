use std::collections::HashMap;

use crate::{Parser, State, chars::{char, one_of}, strings::literal};

#[derive(Clone)]
pub enum JSONValue {
    JSONNull,
    JSONBoolean(bool),
    JSONNumber(f32),
    JSONString(String),
    JSONArray(Vec<JSONValue>),
    JSONObject(HashMap<JSONValue, JSONValue>)
}

fn json_null() -> impl Parser<Output = JSONValue> {
    literal("null")
    .value(JSONValue::JSONNull)
}

fn json_boolean() -> impl Parser<Output = JSONValue> {
    literal("true")
    .value(JSONValue::JSONBoolean(true))
    .or(
        literal("false")
        .value(JSONValue::JSONBoolean(false))   
    )
}

fn zero() -> impl Parser<Output = u8> {
    char('0')
    .value(0)
}

fn nonzero_digit() -> impl Parser<Output = u8> {
    one_of("123456789")
    .map(|ch| ch.to_digit(10).unwrap() as u8)
}

fn digit() -> impl Parser<Output = u8> {
    zero()
    .or(nonzero_digit())
}

fn positive_number() -> impl Parser<Output = u32> {
    nonzero_digit()
    .then(
        digit()
        .many()
    )
    .map(|(first, rest)| {
        let mut acc = 0u32;
        let mut pow = 1u32;

        for digit in rest.into_iter().rev() {
            acc += digit as u32 * pow;
            pow *= 10;
        }

        acc += first as u32 * pow;  
        acc
    })
}

fn natural_number() -> impl Parser<Output = u32> {
    zero()
    .map(u32::from)
    .or(positive_number())
}

fn number() -> impl Parser<Output = i32> {
    char('-')
    .optional()
    .then(natural_number())
    .map(|(sign, number)| {
        match sign {
            Some(_) => -(number as i32),
            None => number as i32
        }
    })
}

fn json_number() -> impl Parser<Output = JSONValue> {
    number()
    .then(
        char('.')
        .then(natural_number())
        .optional()
    )
    .then(
        char('E')
        .or(char('e'))
        .then(
            char('-')
            .or(char('+'))
            .optional()
        )
        .then(natural_number())
    )
    .map(|(((int_part), decimal_part), scientific_part)| {
        
    })
}
