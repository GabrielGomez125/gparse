use crate::{Expectation, Failure, Parser};

pub fn anything<'a>(input: &'a str) -> crate::Result<'a, char> {
    let ch = input
        .chars()
        .next()
        .ok_or(Failure::new(input, Expectation::Anything).recover())?;
    let rest = &input[ch.len_utf8()..];

    Ok((ch, rest, None))
}

pub struct Char(char);

impl Parser for Char {
    type Output = char;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        anything
            .satisfy(move |&output| self.0 == output)
            .expect(Expectation::Char(self.0))
            .parse(input)
    }
}

pub fn char(ch: char) -> Char {
    Char(ch)
}

pub struct OneOf<'a>(&'a str);

impl Parser for OneOf<'_> {
    type Output = char;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        anything
            .satisfy(move |&output| self.0.chars().any(|ch| ch == output))
            .expect(Expectation::OneOf(self.0.to_string()))
            .parse(input)
    }
}

pub fn one_of<'a>(what: &'a str) -> OneOf<'a> {
    OneOf(what)
}

pub struct NoneOf<'a>(&'a str);

impl Parser for NoneOf<'_> {
    type Output = char;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        anything
            .satisfy(move |&output| self.0.chars().all(|ch| ch != output))
            .expect(Expectation::NoneOf(self.0.to_string()))
            .parse(input)
    }
}

pub fn none_of<'a>(what: &'a str) -> NoneOf<'a> {
    NoneOf(what)
}
