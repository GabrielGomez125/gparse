//! Parsers that match a single character.
//!
//! Each parser here consumes exactly one `char` (which may be several bytes
//! long) and outputs it.
//!
//! # Example
//!
//! ```
//! use gparse::Parser;
//! use gparse::chars::{char, none_of, one_of};
//!
//! // A simple identifier: a letter or underscore, then any characters that
//! // aren't whitespace or punctuation.
//! let identifier = one_of("abcdefghijklmnopqrstuvwxyz_")
//!     .then(none_of(" \t\n(),;").many())
//!     .map(|(first, rest)| std::iter::once(first).chain(rest).collect::<String>());
//!
//! let call = identifier.then_ignore(char('(')).then_ignore(char(')'));
//!
//! let (output, rest, _) = call.parse("print_all();").unwrap();
//! assert_eq!(output, "print_all");
//! assert_eq!(rest, ";");
//! ```

use crate::{Expectation, Failure, Parser};

/// Matches any single character.
///
/// Fails only when `input` is empty, with [`Expectation::Anything`].
///
/// This is the basic building block for the other parsers in this module. Pair
/// it with [`Parser::satisfy`] to match characters by any rule.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::chars::anything;
///
/// let (output, rest, _) = anything.parse("héllo").unwrap();
/// assert_eq!(output, 'h');
/// assert_eq!(rest, "éllo");
///
/// // Characters that take more than one byte are handled correctly.
/// let (output, rest, _) = anything.parse("éllo").unwrap();
/// assert_eq!(output, 'é');
/// assert_eq!(rest, "llo");
///
/// assert_eq!(anything.parse("").unwrap_err().to_string(), "Expected anything");
/// ```
pub fn anything<'a>(input: &'a str) -> crate::Result<'a, char> {
    let ch = input
        .chars()
        .next()
        .ok_or(Failure::new(input, Expectation::Anything).recover())?;
    let rest = &input[ch.len_utf8()..];

    Ok((ch, rest, None))
}

/// A parser that matches one specific character.
///
/// This `struct` is created by [`char()`]. See its documentation for more.
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

/// Matches the character `ch`.
///
/// Fails with [`Expectation::Char`] if the input does not start with `ch`.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::chars::char;
///
/// let (output, rest, _) = char('a').parse("abc").unwrap();
/// assert_eq!(output, 'a');
/// assert_eq!(rest, "bc");
///
/// assert_eq!(char('a').parse("xyz").unwrap_err().to_string(), "Expected 'a'");
/// assert_eq!(char('a').parse("").unwrap_err().to_string(), "Expected 'a'");
/// ```
pub fn char(ch: char) -> Char {
    Char(ch)
}

/// A parser that matches any character from a set.
///
/// This `struct` is created by [`one_of`]. See its documentation for more.
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

/// Matches any one character that appears in `what`.
///
/// Fails with [`Expectation::OneOf`] if the next character is not in `what`,
/// or if the input is empty.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::chars::one_of;
///
/// let sign = one_of("+-");
///
/// assert_eq!(sign.parse("+5").unwrap().0, '+');
/// assert_eq!(sign.parse("-5").unwrap().0, '-');
/// assert_eq!(sign.parse("5").unwrap_err().to_string(), "Expected one of +-");
/// ```
pub fn one_of<'a>(what: &'a str) -> OneOf<'a> {
    OneOf(what)
}

/// A parser that matches any character not in a set.
///
/// This `struct` is created by [`none_of`]. See its documentation for more.
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

/// Matches any one character that does **not** appear in `what`.
///
/// Fails with [`Expectation::NoneOf`] if the next character is in `what`, or
/// if the input is empty.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::chars::none_of;
///
/// // Read one comma-separated field.
/// let field = none_of(",\n").many();
///
/// let (output, rest, _) = field.parse("alice,30\n").unwrap();
/// assert_eq!(output.into_iter().collect::<String>(), "alice");
/// assert_eq!(rest, ",30\n");
///
/// assert_eq!(none_of(",").parse(",").unwrap_err().to_string(), "Expected none of ,");
/// ```
pub fn none_of<'a>(what: &'a str) -> NoneOf<'a> {
    NoneOf(what)
}

/// Matches any whitespace character
///
/// Fails with [`Expectation::Label`] if the next character is not
/// whitespace, or if the input is empty.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::chars::whitespace;
///
/// let (output, rest, _) = whitespace.parse("   ").unwrap();
/// assert_eq!(output, ' ');
/// assert_eq!(rest, "  ");
///
/// assert_eq!(whitespace.parse("alice").unwrap_err().to_string(), "Expected any whitespace character");
/// ```
pub fn whitespace<'a>(input: &'a str) -> crate::Result<'a, char> {
    anything
        .satisfy(|&ch| ch.is_whitespace())
        .label("any whitespace character")
        .parse(input)
}
