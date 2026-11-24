//! Parsers whose outputs are strings.

use crate::{
    chars::{anything, char, none_of, one_of},
    Merge, Parser,
};

/// Matches all of the remaining input and outputs it as a `String`.
///
/// `everything` never fails. On an empty input, the output is an empty
/// `String`.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::everything;
///
/// let (output, rest, _) = everything.parse("alice").unwrap();
/// assert_eq!(output, "alice");
/// assert_eq!(rest, "");
///
/// let (output, rest, _) = everything.parse("").unwrap();
/// assert_eq!(output, "");
/// assert_eq!(rest, "");
/// ```
pub fn everything<'a>(input: &'a str) -> crate::Result<'a, String> {
    Ok((input.to_string(), "", None))
}

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

/// A parser that matches zero or more characters from a set.
///
/// This `struct` is created by [`everything_in`]. See its documentation for more.
pub struct EverythingIn<'a>(&'a str);

impl Parser for EverythingIn<'_> {
    type Output = String;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        let mut outputs = String::new();
        let mut rest = input;
        let mut latent = None;

        while let Ok((output, next, residual)) = one_of(self.0).parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual)
        }

        Ok((outputs, rest, latent))
    }
}

/// Matches zero or more characters that appear in `what` and outputs them as a
/// `String`.
///
/// This is the repeated form of [`one_of`], collected into a `String` instead
/// of a `Vec`. Like [`take_while`], it never fails. If the first character is
/// not in `what`, the output is an empty `String` and no input is consumed.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::everything_in;
///
/// let hex = everything_in("0123456789abcdef");
///
/// let (output, rest, _) = hex.parse("ff00cc;").unwrap();
/// assert_eq!(output, "ff00cc");
/// assert_eq!(rest, ";");
///
/// let (output, rest, _) = hex.parse("xyz").unwrap();
/// assert_eq!(output, "");
/// assert_eq!(rest, "xyz");
/// ```
pub fn everything_in(what: &str) -> EverythingIn<'_> {
    EverythingIn(what)
}

/// A parser that matches zero or more characters not in a set.
///
/// This `struct` is created by [`everything_but`]. See its documentation for more.
pub struct EverythingBut<'a>(&'a str);

impl Parser for EverythingBut<'_> {
    type Output = String;

    fn parse<'a>(&self, input: &'a str) -> crate::Result<'a, Self::Output> {
        let mut outputs = String::new();
        let mut rest = input;
        let mut latent = None;

        while let Ok((output, next, residual)) = none_of(self.0).parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual)
        }

        Ok((outputs, rest, latent))
    }
}

/// Matches zero or more characters that do **not** appear in `what` and
/// outputs them as a `String`.
///
/// This is the repeated form of [`none_of`], collected into a `String` instead
/// of a `Vec`. It never fails. The parse stops just before the first character
/// in `what`, or at the end of the input.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::strings::everything_but;
///
/// // Read one comma-separated field.
/// let field = everything_but(",\n");
///
/// let (output, rest, _) = field.parse("alice,30\n").unwrap();
/// assert_eq!(output, "alice");
/// assert_eq!(rest, ",30\n");
///
/// // An empty field is still a match.
/// let (output, rest, _) = field.parse(",30\n").unwrap();
/// assert_eq!(output, "");
/// assert_eq!(rest, ",30\n");
/// ```
pub fn everything_but(what: &str) -> EverythingBut<'_> {
    EverythingBut(what)
}

/// A parser that matches some number of characters.
///
/// This `struct` is created by [`take`]. See its documentation for more.
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
/// Behaves like [`repeat`](crate::Parser::repeat) on [`anything`], but
/// collects the outputs into a `String` instead of a `Vec`.
///
/// Fails with [`Expectation::Label`](crate::Expectation::Label) if fewer than
/// `n` characters are left.
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
/// assert_eq!(four_chars.parse("let").unwrap_err().to_string(), "Expected exactly 4 characters");
/// ```
pub fn take(n: usize) -> Take {
    Take(n)
}

/// A parser that matches zero or more characters satisfying some predicate.
///
/// This `struct` is created by [`take_while`]. See its documentation for more.
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

        while let Ok((output, next, residual)) = anything.parse(rest) {
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

/// Matches zero or more characters that satisfy `predicate` and outputs them
/// as a `String`.
///
/// `take_while` never fails. If the first character does not satisfy
/// `predicate`, the output is an empty `String` and no input is consumed.
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

/// Matches zero or more whitespace characters and outputs them as a `String`.
///
/// Same as `take_while(char::is_whitespace)`. Like [`take_while`], it never
/// fails.
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
