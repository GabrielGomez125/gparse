//! A small parser combinator library.
//!
//! `gparse` builds parsers for `&str` input out of small, reusable pieces.
//! Simple parsers such as [`chars::char()`] or [`numbers::integer`] are combined
//! with methods on the [`Parser`] trait ([`then`](Parser::then),
//! [`or`](Parser::or), [`many`](Parser::many), [`map`](Parser::map), ...) to
//! build parsers for larger structures.
//!
//! # Example
//!
//! ```
//! use gparse::Parser;
//! use gparse::chars::char;
//! use gparse::numbers::integer;
//!
//! // A parser for points such as "(3,-4)".
//! let point = integer
//!     .then_ignore(char(','))
//!     .then(integer)
//!     .delimited(char('('), char(')'));
//!
//! // On success, a parser returns its output and the input it did not consume.
//! let (output, rest, _) = point.parse("(3,-4) and more").unwrap();
//! assert_eq!(output, (3, -4));
//! assert_eq!(rest, " and more");
//!
//! // On failure, the error describes what the parser expected to find.
//! let error = point.parse("(3;-4)").unwrap_err();
//! assert_eq!(error.to_string(), "Expected ','");
//! ```
//!
//! # Parse results
//!
//! Every parser returns a [`Result`]. A successful parse produces a tuple of
//! three values:
//!
//! 1. the **output** of the parser,
//! 2. the **rest** of the input, which the parser did not consume, and
//! 3. a **residual** [`Failure`], if any. This is the furthest recoverable failure
//!    seen while parsing, even though the parse as a whole succeeded. For example,
//!    `char('a').optional()` succeeds on `"b"`, but still reports that it
//!    expected `'a'` as a residual.
//!
//! # Recoverable and committed errors
//!
//! A failed parse returns an [`Error`], which is either
//! [recoverable](Error::Recoverable) or [committed](Error::Committed).
//!
//! Parsers fail recoverably by default. Combinators that try alternatives, such
//! as [`or`](Parser::or) and [`optional`](Parser::optional), only try the
//! alternative after a recoverable error. Calling [`cut`](Parser::cut) on a
//! parser turns its recoverable errors into committed ones. Use it when the input
//! cannot match anything else once a certain point has been reached, so the
//! error is reported right away and no alternatives are tried.
//!
//! # Writing your own parsers
//!
//! Any function or closure with the signature
//! `for<'a> Fn(&'a str) -> gparse::Result<'a, O>` is a [`Parser`] with output
//! `O`, so it can use every combinator:
//!
//! ```
//! use gparse::{Expectation, Failure, Parser};
//!
//! fn vowel(input: &str) -> gparse::Result<'_, char> {
//!     match input.chars().next() {
//!         Some(ch) if "aeiou".contains(ch) => Ok((ch, &input[ch.len_utf8()..], None)),
//!         _ => Err(Failure::new(input, Expectation::Label("a vowel".to_string())).recover()),
//!     }
//! }
//!
//! let (output, rest, _) = vowel.many().parse("aeixyz").unwrap();
//! assert_eq!(output, vec!['a', 'e', 'i']);
//! assert_eq!(rest, "xyz");
//!
//! assert_eq!(vowel.parse("xyz").unwrap_err().to_string(), "Expected a vowel");
//! ```
//!
//! # Modules
//!
//! - [`chars`]: parsers for single characters.
//! - [`strings`]: parsers for fixed strings.
//! - [`numbers`]: parsers for digits, integers, and floating-point numbers.

use std::{cmp::Ordering, error, fmt, result};

pub mod chars;
pub mod numbers;
pub mod strings;

/// A description of what a parser expected to find in the input.
///
/// Expectations are stored in a [`Failure`] and used to build error messages.
/// The [`Display`](fmt::Display) implementation gives the text that appears
/// after "Expected" in those messages.
///
/// # Example
///
/// ```
/// use gparse::Expectation;
///
/// assert_eq!(Expectation::Char('x').to_string(), "'x'");
/// assert_eq!(Expectation::OneOf("abc".to_string()).to_string(), "one of abc");
/// assert_eq!(Expectation::Label("an identifier".to_string()).to_string(), "an identifier");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Expectation {
    /// Any character. Produced when the input ends too soon.
    Anything,
    /// A value that satisfies a predicate. Produced by [`Parser::satisfy`].
    Satisfy,
    /// A specific character. Produced by [`chars::char()`].
    Char(char),
    /// Any one of the characters in the string. Produced by [`chars::one_of`].
    OneOf(String),
    /// Any character except those in the string. Produced by [`chars::none_of`].
    NoneOf(String),
    /// A custom description. Produced by [`Parser::label`].
    Label(String),
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anything => write!(f, "anything"),
            Self::Satisfy => write!(f, "something satisfying some predicate"),
            Self::Char(ch) => write!(f, "\'{}\'", ch),
            Self::OneOf(what) => write!(f, "one of {}", what),
            Self::NoneOf(what) => write!(f, "none of {}", what),
            Self::Label(what) => write!(f, "{}", what),
        }
    }
}

/// Information about where and why a parser failed.
///
/// A `Failure` records the position in the input where parsing failed and
/// what was expected there. It is shown as a message such as
/// `Expected 'a' or 'b'`.
///
/// When two failures are combined, for example by [`Parser::or`], the failure
/// that got further into the input is kept. If both failed at the same
/// position, their expectations are joined.
///
/// A `Failure` is usually wrapped in an [`Error`] with [`recover`](Failure::recover)
/// or [`commit`](Failure::commit).
///
/// # Example
///
/// ```
/// use gparse::{Expectation, Failure};
///
/// let failure = Failure::new("xyz", Expectation::Char('a'));
/// assert_eq!(failure.to_string(), "Expected 'a'");
///
/// let failure = failure.expect(Expectation::Label("a greeting".to_string()));
/// assert_eq!(failure.to_string(), "Expected a greeting");
/// ```
#[derive(Debug, PartialEq)]
pub struct Failure {
    /// The length of the input left when the failure happened. A smaller value
    /// means the parser got further into the input.
    remaining: usize,
    expected: Vec<Expectation>,
}

impl Failure {
    /// Creates a failure at the start of `source` that expected `expected`.
    ///
    /// `source` should be the input that was left when the parser failed,
    /// which is usually the input the failing parser was given.
    pub fn new(source: &str, expected: Expectation) -> Self {
        Self {
            remaining: source.len(),
            expected: vec![expected],
        }
    }

    /// Replaces all of this failure's expectations with `expected`.
    ///
    /// The position of the failure is kept.
    pub fn expect(mut self, expected: Expectation) -> Self {
        self.expected.clear();
        self.expected.push(expected);
        self
    }

    /// Wraps this failure in [`Error::Recoverable`].
    pub fn recover(self) -> Error {
        Error::Recoverable(self)
    }

    /// Wraps this failure in [`Error::Committed`].
    pub fn commit(self) -> Error {
        Error::Committed(self)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.expected[..] {
            [expectation] => write!(f, "Expected {}", expectation),
            [expectation0, expectation1] => {
                write!(f, "Expected {} or {}", expectation0, expectation1)
            }
            expectations => {
                write!(f, "Expected ")?;

                for (i, expectation) in expectations.iter().enumerate() {
                    if i < expectations.len() - 1 {
                        write!(f, "{} or ", expectation)?;
                    } else {
                        write!(f, "{}", expectation)?;
                    }
                }

                Ok(())
            }
        }
    }
}

/// Combines two failures into one.
trait Merge {
    /// Keeps whichever failure got further into the input. If both got equally
    /// far, joins their expectations.
    fn merge(self, other: Self) -> Self;
}

impl Merge for Failure {
    fn merge(mut self, mut other: Self) -> Self {
        match self.remaining.cmp(&other.remaining) {
            Ordering::Equal => {
                self.expected.append(&mut other.expected);
                self
            }
            Ordering::Greater => other,
            Ordering::Less => self,
        }
    }
}

impl Merge for Option<Failure> {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Some(failure0), Some(failure1)) => Some(failure0.merge(failure1)),
            (residual0, None) => residual0,
            (None, residual1) => residual1,
        }
    }
}

/// The error returned when a parser fails.
///
/// Both variants hold a [`Failure`] that describes what went wrong. The variant
/// decides whether combinators that try alternatives, such as [`Parser::or`]
/// and [`Parser::optional`], may try another option.
///
/// # Example
///
/// ```
/// use gparse::{Error, Parser};
/// use gparse::chars::char;
///
/// let error = char('a').parse("b").unwrap_err();
/// assert!(matches!(error, Error::Recoverable(_)));
/// assert_eq!(error.to_string(), "Expected 'a'");
///
/// let error = char('a').cut().parse("b").unwrap_err();
/// assert!(matches!(error, Error::Committed(_)));
/// ```
#[derive(Debug, PartialEq)]
pub enum Error {
    /// The parser failed, and other alternatives may still be tried.
    Recoverable(Failure),
    /// The parser failed after committing to this way of parsing the input,
    /// so no other alternatives are tried. Produced by [`Parser::cut`].
    Committed(Failure),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Recoverable(failure) => failure.fmt(f),
            Error::Committed(failure) => failure.fmt(f),
        }
    }
}

impl error::Error for Error {}

/// The result of running a parser on input with lifetime `'a`.
///
/// On success, holds `(output, rest, residual)`:
///
/// - `output`: the value the parser produced.
/// - `rest`: the part of the input that was not consumed.
/// - `residual`: the furthest recoverable [`Failure`] seen during the parse, if
///   any. See the [crate-level documentation](crate#parse-results) for details.
///
/// On failure, holds an [`Error`].
pub type Result<'a, T> = result::Result<(T, &'a str, Option<Failure>), Error>;

/// A parser that turns the start of a `&str` into a value of type
/// [`Output`](Parser::Output).
///
/// This trait's methods build bigger parsers out of smaller ones. Each method
/// takes ownership of `self` and returns a new parser wrapping it.
///
/// `Parser` is implemented for every function or closure with the signature
/// `for<'a> Fn(&'a str) -> Result<'a, O>`, so plain functions such as
/// [`chars::anything`] and [`numbers::integer`] can use these methods directly.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::chars::{char, one_of};
///
/// // Either "yes" or "no", ignoring case on the first letter.
/// let answer = one_of("Yy").then(char('e')).then(char('s')).value(true)
///     .or(one_of("Nn").then(char('o')).value(false));
///
/// assert_eq!(answer.parse("Yes!").unwrap().0, true);
/// assert_eq!(answer.parse("no").unwrap().0, false);
/// assert_eq!(answer.parse("maybe").unwrap_err().to_string(), "Expected one of Yy or one of Nn");
/// ```
pub trait Parser: Sized {
    /// The type of value this parser produces.
    type Output;

    /// Runs this parser on the start of `input`.
    ///
    /// Returns the output, the unconsumed rest of the input, and any residual
    /// failure. See [`Result`] for details.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    ///
    /// let (output, rest, _) = char('h').parse("hello").unwrap();
    /// assert_eq!(output, 'h');
    /// assert_eq!(rest, "ello");
    ///
    /// assert!(char('h').parse("goodbye").is_err());
    /// ```
    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output>;

    //---------- Error combinators ----------

    /// Makes every error from this parser a committed error.
    ///
    /// The new parser behaves like `self`, but turns any
    /// [`Error::Recoverable`] into [`Error::Committed`]. A committed error stops
    /// [`or`](Parser::or) and [`optional`](Parser::optional) from trying
    /// alternatives, so the error is reported right away.
    ///
    /// Use this once enough input has been seen to know that nothing else can
    /// match. In the example below, once a `'-'` has been read, a missing number
    /// after it is a real error. It is not a reason to try the next alternative.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::{Error, Parser};
    /// use gparse::chars::char;
    /// use gparse::numbers::positive_integer;
    ///
    /// assert!(matches!(char('a').parse("hello"), Err(Error::Recoverable(_))));
    /// assert!(matches!(char('a').cut().parse("hello"), Err(Error::Committed(_))));
    ///
    /// // Without `cut`, "-x" falls through to the second option and succeeds with -1.
    /// let flag = char('-').ignore_then(positive_integer).map(|n| n as i64)
    ///     .or(char('-').value(-1));
    /// assert_eq!(flag.parse("-x").unwrap().0, -1);
    ///
    /// // With `cut`, a '-' that isn't followed by a number is reported as an error.
    /// let flag = char('-').ignore_then(positive_integer.cut()).map(|n| n as i64)
    ///     .or(char('-').value(-1));
    /// assert!(matches!(flag.parse("-x"), Err(Error::Committed(_))));
    /// ```
    fn cut(self) -> Cut<Self> {
        Cut(self)
    }

    /// Replaces the expectations in this parser's errors with `expected`.
    ///
    /// Errors stay recoverable or committed, as before. Successful parses are
    /// not affected. [`label`](Parser::label) is a shorthand for
    /// `expect(Expectation::Label(..))`.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::{Expectation, Parser};
    /// use gparse::chars::anything;
    ///
    /// let digit = anything.satisfy(|ch| ch.is_ascii_digit());
    /// assert_eq!(
    ///     digit.parse("x").unwrap_err().to_string(),
    ///     "Expected something satisfying some predicate"
    /// );
    ///
    /// let digit = digit.expect(Expectation::OneOf("0123456789".to_string()));
    /// assert_eq!(digit.parse("x").unwrap_err().to_string(), "Expected one of 0123456789");
    /// ```
    fn expect(self, expected: Expectation) -> Expect<Self> {
        Expect(self, expected)
    }

    /// Gives this parser a readable name to use in error messages.
    ///
    /// Same as `self.expect(Expectation::Label(label.to_string()))`.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::one_of;
    ///
    /// let hex = one_of("0123456789abcdefABCDEF");
    /// assert_eq!(
    ///     hex.parse("g").unwrap_err().to_string(),
    ///     "Expected one of 0123456789abcdefABCDEF"
    /// );
    ///
    /// let hex = one_of("0123456789abcdefABCDEF").label("a hexadecimal digit");
    /// assert_eq!(hex.parse("g").unwrap_err().to_string(), "Expected a hexadecimal digit");
    /// ```
    fn label(self, label: &str) -> Expect<Self> {
        Expect(self, Expectation::Label(label.to_string()))
    }

    //---------- Repetition combinators ----------

    /// Runs this parser zero or more times and collects the outputs in a `Vec`.
    ///
    /// The parser is run until it fails. The parse ends just before the input
    /// that caused the failure. `many` never fails. If `self` fails right away,
    /// the output is an empty `Vec` and no input is consumed.
    ///
    /// Use [`many_once`](Parser::many_once) if at least one match is needed.
    ///
    /// # Infinite loops
    ///
    /// If `self` can succeed without consuming any input, as with
    /// `p.optional()`, then `many` will never stop.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    ///
    /// let (output, rest, _) = char('a').many().parse("aaab").unwrap();
    /// assert_eq!(output, vec!['a', 'a', 'a']);
    /// assert_eq!(rest, "b");
    ///
    /// // `many` still succeeds when there are no matches.
    /// let (output, rest, _) = char('a').many().parse("baaa").unwrap();
    /// assert_eq!(output, Vec::<char>::new());
    /// assert_eq!(rest, "baaa");
    /// ```
    fn many(self) -> Many<Self> {
        Many(self)
    }

    /// Runs this parser one or more times and collects the outputs in a `Vec`.
    ///
    /// Works like [`many`](Parser::many), except that the first run of `self`
    /// must succeed. If it fails, its error is returned. A successful output
    /// always has at least one element.
    ///
    /// # Infinite loops
    ///
    /// As with `many`, this never stops if `self` can succeed without consuming
    /// any input.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    ///
    /// let (output, rest, _) = char('a').many_once().parse("aaab").unwrap();
    /// assert_eq!(output, vec!['a', 'a', 'a']);
    /// assert_eq!(rest, "b");
    ///
    /// // Unlike `many`, `many_once` fails when there are no matches.
    /// assert_eq!(char('a').many_once().parse("baaa").unwrap_err().to_string(), "Expected 'a'");
    /// ```
    fn many_once(self) -> ManyOnce<Self> {
        ManyOnce(self)
    }

    /// Runs this parser exactly `n` times and collects the outputs in a `Vec`.
    ///
    /// If any of the `n` runs fails, that error is returned. Otherwise the
    /// output has exactly `n` elements. Parsing stops after `n` matches, even if
    /// more would follow.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::anything;
    ///
    /// let (output, rest, _) = anything.repeat(4).parse("goodbye").unwrap();
    /// assert_eq!(output, vec!['g', 'o', 'o', 'd']);
    /// assert_eq!(rest, "bye");
    ///
    /// // "goodbye" has only seven characters.
    /// assert!(anything.repeat(8).parse("goodbye").is_err());
    /// ```
    fn repeat(self, n: usize) -> Repeat<Self> {
        Repeat(self, n)
    }

    //---------- Sequence combinators ----------

    /// Tries this parser, then tries `other` if this one fails recoverably.
    ///
    /// Both parsers must have the same output type, and both start from the
    /// same input. `other` is only tried if `self` returns
    /// [`Error::Recoverable`]. A committed error from `self` is returned as is.
    ///
    /// If both fail recoverably, the error from the parser that got further
    /// into the input is returned. If they got equally far, their expectations
    /// are joined, e.g. `Expected 'a' or 'b'`.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::{Error, Parser};
    /// use gparse::chars::char;
    ///
    /// let a_or_b = char('a').or(char('b'));
    ///
    /// assert_eq!(a_or_b.parse("apple").unwrap().0, 'a');
    /// assert_eq!(a_or_b.parse("banana").unwrap().0, 'b');
    ///
    /// let error = a_or_b.parse("coconut").unwrap_err();
    /// assert!(matches!(error, Error::Recoverable(_)));
    /// assert_eq!(error.to_string(), "Expected 'a' or 'b'");
    ///
    /// // A committed error from the first parser means the second is never tried.
    /// let error = char('a').cut().or(char('b')).parse("banana").unwrap_err();
    /// assert!(matches!(error, Error::Committed(_)));
    /// ```
    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<Output = Self::Output>,
    {
        Or(self, other)
    }

    /// Runs this parser, then runs `next` on the rest of the input. The output
    /// is a tuple of both outputs.
    ///
    /// Fails if either parser fails. To keep only one of the two outputs, use
    /// [`then_ignore`](Parser::then_ignore) or
    /// [`ignore_then`](Parser::ignore_then).
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    ///
    /// let hi = char('h').then(char('i'));
    ///
    /// let (output, rest, _) = hi.parse("hi mom").unwrap();
    /// assert_eq!(output, ('h', 'i'));
    /// assert_eq!(rest, " mom");
    ///
    /// // Both parsers must succeed.
    /// assert_eq!(hi.parse("bye mom").unwrap_err().to_string(), "Expected 'h'");
    /// assert_eq!(hi.parse("hey mom").unwrap_err().to_string(), "Expected 'i'");
    /// ```
    fn then<P>(self, next: P) -> Then<Self, P>
    where
        P: Parser,
    {
        Then(self, next)
    }

    /// Runs this parser, then runs `next`, and keeps only this parser's output.
    ///
    /// Both parsers must succeed. This is useful for skipping input that comes
    /// after a value, such as a separator or terminator.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    /// use gparse::numbers::integer;
    ///
    /// // An integer followed by a semicolon.
    /// let statement = integer.then_ignore(char(';'));
    ///
    /// let (output, rest, _) = statement.parse("42; 7;").unwrap();
    /// assert_eq!(output, 42);
    /// assert_eq!(rest, " 7;");
    ///
    /// // The semicolon is still required.
    /// assert_eq!(statement.parse("42").unwrap_err().to_string(), "Expected ';'");
    /// ```
    fn then_ignore<P>(self, next: P) -> ThenIgnore<Self, P>
    where
        P: Parser,
    {
        ThenIgnore(self, next)
    }

    /// Runs this parser, then runs `next`, and keeps only `next`'s output.
    ///
    /// Both parsers must succeed. This is useful for skipping input that comes
    /// before a value, such as a prefix or keyword.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    /// use gparse::numbers::nonnegative_integer;
    ///
    /// // A number with a '#' in front of it, like "#12".
    /// let issue = char('#').ignore_then(nonnegative_integer);
    ///
    /// let (output, rest, _) = issue.parse("#12 is fixed").unwrap();
    /// assert_eq!(output, 12);
    /// assert_eq!(rest, " is fixed");
    ///
    /// // The '#' is still required.
    /// assert_eq!(issue.parse("12").unwrap_err().to_string(), "Expected '#'");
    /// ```
    fn ignore_then<P>(self, next: P) -> IgnoreThen<Self, P>
    where
        P: Parser,
    {
        IgnoreThen(self, next)
    }

    /// Runs `left`, then this parser, then `right`, and keeps only this
    /// parser's output.
    ///
    /// Useful for values wrapped in brackets, quotes, and so on. All three
    /// parsers must succeed.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::{char, none_of};
    ///
    /// // A double-quoted string with no escape sequences.
    /// let quoted = none_of("\"")
    ///     .many()
    ///     .map(|chars| chars.into_iter().collect::<String>())
    ///     .delimited(char('"'), char('"'));
    ///
    /// let (output, rest, _) = quoted.parse("\"hello world\" rest").unwrap();
    /// assert_eq!(output, "hello world");
    /// assert_eq!(rest, " rest");
    ///
    /// // A missing closing quote is an error.
    /// assert_eq!(quoted.parse("\"unterminated").unwrap_err().to_string(), "Expected '\"'");
    /// ```
    fn delimited<L, R>(self, left: L, right: R) -> Delimited<Self, L, R>
    where
        L: Parser,
        R: Parser,
    {
        Delimited(self, left, right)
    }

    //---------- Other combinators ----------

    /// Changes this parser's output with the function `op`.
    ///
    /// `op` is only called when the parse succeeds. Errors are passed through
    /// unchanged.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::anything;
    ///
    /// let hex = anything.map(|ch| ch.to_digit(16));
    ///
    /// assert_eq!(hex.parse("7F").unwrap().0, Some(7));
    /// assert_eq!(hex.parse("D6").unwrap().0, Some(13));
    ///
    /// // `map` can't make a parse fail. Here the parse succeeds and `op` returns `None`.
    /// assert_eq!(hex.parse("G4").unwrap().0, None);
    ///
    /// // Use `satisfy` to reject input instead.
    /// let strict_hex = anything
    ///     .satisfy(|ch| ch.is_ascii_hexdigit())
    ///     .map(|ch| ch.to_digit(16).unwrap());
    /// assert!(strict_hex.parse("G4").is_err());
    /// ```
    fn map<F, O>(self, op: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> O,
    {
        Map(self, op)
    }

    /// Makes this parser optional.
    ///
    /// If `self` succeeds with output `x`, this succeeds with `Some(x)`. If
    /// `self` fails recoverably, this succeeds with `None` and consumes no input.
    /// If `self` returns a committed error, that error is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::char;
    ///
    /// let maybe_a = char('a').optional();
    ///
    /// let (output, rest, _) = maybe_a.parse("apple").unwrap();
    /// assert_eq!(output, Some('a'));
    /// assert_eq!(rest, "pple");
    ///
    /// let (output, rest, _) = maybe_a.parse("banana").unwrap();
    /// assert_eq!(output, None);
    /// assert_eq!(rest, "banana");
    ///
    /// // Committed errors are not turned into `None`.
    /// assert!(char('a').cut().optional().parse("banana").is_err());
    /// ```
    fn optional(self) -> Optional<Self> {
        Optional(self)
    }

    /// Accepts this parser's output only if it satisfies `predicate`.
    ///
    /// If `self` succeeds but `predicate` returns `false` for its output, the
    /// parse fails recoverably with [`Expectation::Satisfy`]. Use
    /// [`label`](Parser::label) to give that error a more helpful message.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::chars::anything;
    ///
    /// let uppercase = anything.satisfy(|ch| ch.is_uppercase()).label("an uppercase letter");
    ///
    /// let (output, rest, _) = uppercase.parse("Hello").unwrap();
    /// assert_eq!(output, 'H');
    /// assert_eq!(rest, "ello");
    ///
    /// assert_eq!(uppercase.parse("hello").unwrap_err().to_string(), "Expected an uppercase letter");
    /// ```
    fn satisfy<F>(self, predicate: F) -> Satisfy<Self, F>
    where
        F: Fn(&Self::Output) -> bool,
    {
        Satisfy(self, predicate)
    }

    /// Replaces this parser's output with a clone of `what`.
    ///
    /// Useful when the matched text is only needed to choose a value, as with
    /// keywords or symbols.
    ///
    /// # Example
    ///
    /// ```
    /// use gparse::Parser;
    /// use gparse::strings::literal;
    ///
    /// let boolean = literal("true").value(true).or(literal("false").value(false));
    ///
    /// let (output, rest, _) = boolean.parse("false, true").unwrap();
    /// assert_eq!(output, false);
    /// assert_eq!(rest, ", true");
    /// ```
    fn value<T>(self, what: T) -> Value<Self, T>
    where
        T: Clone,
    {
        Value(self, what)
    }
}

impl<F, O> Parser for F
where
    F: for<'a> Fn(&'a str) -> Result<'a, O>,
{
    type Output = O;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        self(input)
    }
}

//---------- Error combinators ----------

/// A parser that turns recoverable errors into committed errors.
///
/// This `struct` is created by [`Parser::cut`]. See its documentation for more.
pub struct Cut<P>(P);

impl<P> Parser for Cut<P>
where
    P: Parser,
{
    type Output = P::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Err(Error::Recoverable(failure)) => Err(Error::Committed(failure)),
            otherwise => otherwise,
        }
    }
}

/// A parser that replaces the expectations in its errors.
///
/// This `struct` is created by [`Parser::expect`] and [`Parser::label`]. See
/// their documentation for more.
pub struct Expect<P>(P, Expectation);

impl<P> Parser for Expect<P>
where
    P: Parser,
{
    type Output = P::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Err(Error::Recoverable(failure)) => Err(failure.expect(self.1.clone()).recover()),
            Err(Error::Committed(failure)) => Err(failure.expect(self.1.clone()).commit()),
            otherwise => otherwise,
        }
    }
}

//---------- Repetition combinators ----------

/// A parser that runs another parser zero or more times.
///
/// This `struct` is created by [`Parser::many`]. See its documentation for more.
pub struct Many<P>(P);

impl<P> Parser for Many<P>
where
    P: Parser,
{
    type Output = Vec<P::Output>;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let mut outputs = Vec::new();
        let mut rest = input;
        let mut latent = None;

        while let Ok((output, next, residual)) = self.0.parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

/// A parser that runs another parser one or more times.
///
/// This `struct` is created by [`Parser::many_once`]. See its documentation for
/// more.
pub struct ManyOnce<P>(P);

impl<P> Parser for ManyOnce<P>
where
    P: Parser,
{
    type Output = Vec<P::Output>;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, next, residual) = self.0.parse(input)?;
        let mut outputs = vec![output];
        let mut rest = next;
        let mut latent = residual;

        while let Ok((output, next, residual)) = self.0.parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

/// A parser that runs another parser a fixed number of times.
///
/// This `struct` is created by [`Parser::repeat`]. See its documentation for
/// more.
pub struct Repeat<P>(P, usize);

impl<P> Parser for Repeat<P>
where
    P: Parser,
{
    type Output = Vec<P::Output>;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let mut outputs = Vec::with_capacity(self.1);
        let mut rest = input;
        let mut latent = None;

        for _ in 0..self.1 {
            let (output, next, residual) = self.0.parse(rest)?;
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

//---------- Sequence combinators ----------

/// A parser that tries one parser, then another if the first fails.
///
/// This `struct` is created by [`Parser::or`]. See its documentation for more.
pub struct Or<A, B>(A, B);

impl<A, B> Parser for Or<A, B>
where
    A: Parser,
    B: Parser<Output = A::Output>,
{
    type Output = A::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Err(Error::Recoverable(failure0)) => match self.1.parse(input) {
                Ok((output, rest, residual)) => Ok((output, rest, Some(failure0).merge(residual))),
                Err(Error::Recoverable(failure1)) => {
                    Err(Error::Recoverable(failure0.merge(failure1)))
                }
                otherwise => otherwise,
            },
            otherwise => otherwise,
        }
    }
}

/// A parser that runs two parsers in a row and keeps both outputs.
///
/// This `struct` is created by [`Parser::then`]. See its documentation for more.
pub struct Then<A, B>(A, B);

impl<A, B> Parser for Then<A, B>
where
    A: Parser,
    B: Parser,
{
    type Output = (A::Output, B::Output);

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output0, rest, residual0) = self.0.parse(input)?;
        let (output1, rest, residual1) = self.1.parse(rest)?;

        Ok(((output0, output1), rest, residual0.merge(residual1)))
    }
}

/// A parser that runs two parsers in a row and keeps the first output.
///
/// This `struct` is created by [`Parser::then_ignore`]. See its documentation
/// for more.
pub struct ThenIgnore<A, B>(A, B);

impl<A, B> Parser for ThenIgnore<A, B>
where
    A: Parser,
    B: Parser,
{
    type Output = A::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, rest, resiudal0) = self.0.parse(input)?;
        let (_, rest, residual1) = self.1.parse(rest)?;

        Ok((output, rest, resiudal0.merge(residual1)))
    }
}

/// A parser that runs two parsers in a row and keeps the second output.
///
/// This `struct` is created by [`Parser::ignore_then`]. See its documentation
/// for more.
pub struct IgnoreThen<A, B>(A, B);

impl<A, B> Parser for IgnoreThen<A, B>
where
    A: Parser,
    B: Parser,
{
    type Output = B::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (_, rest, residual0) = self.0.parse(input)?;
        let (output, rest, residual1) = self.1.parse(rest)?;

        Ok((output, rest, residual0.merge(residual1)))
    }
}

/// A parser that runs between two other parsers and keeps only its own output.
///
/// This `struct` is created by [`Parser::delimited`]. See its documentation for
/// more.
pub struct Delimited<P, L, R>(P, L, R);

impl<P, L, R> Parser for Delimited<P, L, R>
where
    P: Parser,
    L: Parser,
    R: Parser,
{
    type Output = P::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (_, rest, residual0) = self.1.parse(input)?;
        let (output, rest, residual1) = self.0.parse(rest)?;
        let (_, rest, residual2) = self.2.parse(rest)?;

        Ok((output, rest, residual0.merge(residual1).merge(residual2)))
    }
}

//---------- Other combinators ----------

/// A parser that changes another parser's output with a function.
///
/// This `struct` is created by [`Parser::map`]. See its documentation for more.
pub struct Map<P, F>(P, F);

impl<P, F, O> Parser for Map<P, F>
where
    P: Parser,
    F: Fn(P::Output) -> O,
{
    type Output = O;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, rest, residual) = self.0.parse(input)?;

        Ok((self.1(output), rest, residual))
    }
}

/// A parser that succeeds with `None` when another parser fails recoverably.
///
/// This `struct` is created by [`Parser::optional`]. See its documentation for
/// more.
pub struct Optional<P>(P);

impl<P> Parser for Optional<P>
where
    P: Parser,
{
    type Output = Option<P::Output>;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Ok((output, rest, residual)) => Ok((Some(output), rest, residual)),
            Err(Error::Recoverable(failure)) => Ok((None, input, Some(failure))),
            Err(error) => Err(error),
        }
    }
}

/// A parser that only accepts outputs that satisfy a predicate.
///
/// This `struct` is created by [`Parser::satisfy`]. See its documentation for
/// more.
pub struct Satisfy<P, F>(P, F);

impl<P, F> Parser for Satisfy<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    type Output = P::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, rest, residual) = self.0.parse(input)?;

        if self.1(&output) {
            Ok((output, rest, residual))
        } else {
            Err(Failure::new(input, Expectation::Satisfy).recover())
        }
    }
}

/// A parser that replaces another parser's output with a fixed value.
///
/// This `struct` is created by [`Parser::value`]. See its documentation for
/// more.
pub struct Value<P, T>(P, T);

impl<P, T> Parser for Value<P, T>
where
    P: Parser,
    T: Clone,
{
    type Output = T;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Ok((_, rest, residual)) => Ok((self.1.clone(), rest, residual)),
            Err(error) => Err(error),
        }
    }
}
