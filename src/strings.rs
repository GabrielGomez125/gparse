//! Parsers that match whole strings.

use crate::{chars::char, Merge, Parser};

/// A parser that matches a fixed string.
///
/// This `struct` is created by [`literal`]. See its documentation for more.
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

        Ok((self.0.to_string(), rest, latent))
    }
}

/// Matches the exact string `source` and outputs it as a `String`.
///
/// The input is checked one character at a time. If it doesn't match, the
/// error points to the first character that differs and names the character
/// that was expected there.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::literal;
///
/// let (output, rest, _) = literal("let").parse("let x = 1;").unwrap();
/// assert_eq!(output, "let");
/// assert_eq!(rest, " x = 1;");
///
/// // The error names the first character that doesn't match.
/// assert_eq!(literal("let").parse("lex").unwrap_err().to_string(), "Expected 't'");
/// ```
pub fn literal<'a>(source: &'a str) -> Literal<'a> {
    Literal(source)
}
