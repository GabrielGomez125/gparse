//! Parsers whose outputs are strings.

use crate::{
    chars::{anything, char},
    Merge, Parser,
};

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

/// A parser that matches some number of characters.
///
/// This `struct` is constructed by [`take`]. See its documentation for more.
pub struct Take(usize);

impl Parser for Take {
    type Output = String;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        let mut outputs = String::new();
        let mut rest = input;
        let mut latent = None;

        for _ in 0..self.0 {
            let (output, next, residual) = anything
                .label(&format!("exactly {} characters", self.0))
                .parse(rest)?;
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

/// Matches exactly `n` characters and outputs them as a `String`.
///
/// This parser behaves similarly to [`repeat`](crate::Parser::repeat), but
/// matches any character instead of some given parser, and collects its outputs
/// into a `String` instead of a `Vec`.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::take;
///
/// let four_chars = take(4);
///
/// let (output, rest, _) = four_chars.parse("whatsoever").unwrap();
/// assert_eq!(output, "what");
/// assert_eq!(rest, "soever");
///
/// assert_eq!(four_chars.parse("let").unwrap_err().to_string(), "Expected exactly 4 characters")
/// ```
pub fn take(n: usize) -> Take {
    Take(n)
}

/// A parser that matches zero or more characters satisfying some predicate.
///
/// This `struct` is constructed by [`take_while`]. See its documentation for more.
pub struct TakeWhile<F>(F);

impl<F> Parser for TakeWhile<F>
where
    F: Fn(char) -> bool,
{
    type Output = String;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        let mut outputs = String::new();
        let mut rest = input;
        let mut latent = None;

        loop {
            let Ok((output, next, residual)) = anything.parse(rest) else {
                break;
            };

            if !self.0(output) {
                break;
            }

            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

/// Matches zero or more characters satisfying some predicate.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::take_while;
///
/// let letters = take_while(char::is_alphabetic);
///
/// let (output, rest, _) = letters.parse("abc123").unwrap();
/// assert_eq!(output, "abc");
/// assert_eq!(rest, "123");
///
/// let (output, rest, _) = letters.parse("123abc").unwrap();
/// assert_eq!(output, "");
/// assert_eq!(rest, "123abc");
/// ```
pub fn take_while<F>(predicate: F) -> TakeWhile<F>
where
    F: Fn(char) -> bool,
{
    TakeWhile(predicate)
}

/// Matches zero or more whitespace characters.
///
/// This parser is equivalent to [`take_while`] with predicate [`char::is_whitespace`]
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::manyspace;
///
/// let (output, rest, _) = manyspace.parse("   abc").unwrap();
/// assert_eq!(output, "   ");
/// assert_eq!(rest, "abc");
///
/// let (output, rest, _) = manyspace.parse("123").unwrap();
/// assert_eq!(output, "");
/// assert_eq!(rest, "123");
/// ```
pub fn manyspace<'a>(input: &'a str) -> crate::Result<'a, String> {
    take_while(char::is_whitespace).parse(input)
}
