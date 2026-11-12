use crate::{chars::char, Merge, Parser};

pub struct Literal<'a>(&'a str);

impl Parser for Literal<'_> {
    type Output = String;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        let mut rest = input;
        let mut latent = None;

        for ch in self.0.chars() {
            let (_, next, residual) = char(ch).parse(rest)?;
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((input.to_string(), rest, latent))
    }
}

pub fn literal<'a>(source: &'a str) -> Literal<'a> {
    Literal(source)
}
