use crate::{Error, Parser, State};

pub fn anything(state: &mut State) -> crate::Result<char> {
    state
        .next()
        .ok_or(Error::Recoverable)
}

pub fn char(ch: char) -> impl Parser<Output = char> {
    anything
        .satisfy(move |&output| ch == output)
}

pub fn one_of(what: &str) -> impl Parser<Output = char> {
    anything
        .satisfy(move |&output| {
            what
                .chars()
                .any(|ch| ch == output)
        })
}

pub fn none_of(what: &str) -> impl Parser<Output = char> {
    anything
        .satisfy(move |&output| {
            what
                .chars()
                .all(|ch| ch != output)  
        })
}