use crate::{Parser, State, chars::char};

pub fn literal(what: &str) -> impl Parser<Output = String> {
    move |state: &mut State<'_>| {
        for ch in what.chars() {
            char(ch).parse(state)?;
        }

        Ok(what.to_string())
    }
}