use crate::{Merge, Parser, chars::char};

pub struct Literal(String);

impl Parser for Literal {
    type Output = String;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        let mut rest = input;
        let mut latent = None;

        for ch in self.0.chars() {
            let (_, next, failure) = char(ch).parse(rest)?;
            rest = next;
            latent = latent.merge(failure);
        }

        Ok((input.to_string(), rest, latent))
    }
}

pub fn literal(string: &str) -> Literal {    
    Literal(string.to_string())
}
