use crate::{Expectation, Failure, Parser};

pub fn anything<'a>(input: &'a str) -> crate::Result<'a, char> {
    let (first, rest) = input
        .split_at_checked(1)
        .ok_or(Failure::from_source(input, Expectation::Anything))?;

    let ch = first.chars().next().unwrap();

    Ok((ch, rest, None))
}

pub fn char(ch: char) -> impl Parser<Output = char> {
    anything
        .satisfy(move |&output| ch == output)
        .expect(Expectation::Char(ch))
}

pub fn one_of(what: &str) -> impl Parser<Output = char> {
    anything.satisfy(move |&output| what.chars().any(|ch| ch == output))
}

pub fn none_of(what: &str) -> impl Parser<Output = char> {
    anything.satisfy(move |&output| what.chars().all(|ch| ch != output))
}
