use std::{cmp::Ordering, error, fmt, result};

pub mod chars;
pub mod numbers;
pub mod strings;

#[derive(Clone, Debug, PartialEq)]
pub enum Expectation {
    Anything,
    Satisfy,
    Char(char),
    OneOf(String),
    NoneOf(String),
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

#[derive(Debug)]
pub struct Failure {
    remaining: usize,
    expected: Vec<Expectation>,
}

impl Failure {
    pub fn new(source: &str, expected: Expectation) -> Self {
        Self {
            remaining: source.len(),
            expected: vec![expected],
        }
    }

    pub fn expect(mut self, expected: Expectation) -> Self {
        self.expected.clear();
        self.expected.push(expected);
        self
    }

    pub fn recover(self) -> Error {
        Error::Recoverable(self)
    }

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

trait Merge {
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
            (failure0, None) => failure0,
            (None, failure1) => failure1,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Recoverable(Failure),
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

pub type Result<'a, T> = result::Result<(T, &'a str, Option<Failure>), Error>;

pub trait Parser: Sized {
    type Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output>;

    //---------- Error combinators ----------

    fn cut(self) -> Cut<Self> {
        Cut(self)
    }

    fn expect(self, expected: Expectation) -> Expect<Self> {
        Expect(self, expected)
    }

    fn label(self, label: &str) -> Expect<Self> {
        Expect(self, Expectation::Label(label.to_string()))
    }

    //---------- Repetition combinators ----------

    fn many(self) -> Many<Self> {
        Many(self)
    }

    fn many_once(self) -> ManyOnce<Self> {
        ManyOnce(self)
    }

    fn repeat(self, n: usize) -> Repeat<Self> {
        Repeat(self, n)
    }

    //---------- Sequence combinators ----------

    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<Output = Self::Output>,
    {
        Or(self, other)
    }

    fn then<P>(self, next: P) -> Then<Self, P>
    where
        P: Parser,
    {
        Then(self, next)
    }

    fn then_ignore<P>(self, next: P) -> ThenIgnore<Self, P>
    where
        P: Parser,
    {
        ThenIgnore(self, next)
    }

    fn ignore_then<P>(self, next: P) -> IgnoreThen<Self, P>
    where
        P: Parser,
    {
        IgnoreThen(self, next)
    }

    fn delimited<L, R>(self, left: L, right: R) -> Delimited<Self, L, R>
    where
        L: Parser,
        R: Parser,
    {
        Delimited(self, left, right)
    }

    //---------- Other combinators ----------

    fn map<F, O>(self, op: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> O,
    {
        Map(self, op)
    }

    fn optional(self) -> Optional<Self> {
        Optional(self)
    }

    fn satisfy<F>(self, predicate: F) -> Satisfy<Self, F>
    where
        F: Fn(&Self::Output) -> bool,
    {
        Satisfy(self, predicate)
    }

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

pub struct ManyOnce<P>(P);

impl<P> Parser for ManyOnce<P>
where
    P: Parser,
{
    type Output = Vec<P::Output>;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, next, failure) = self.0.parse(input)?;
        let mut outputs = vec![output];
        let mut rest = next;
        let mut latent = failure;

        while let Ok((output, next, residual)) = self.0.parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

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
            let (output, next, residual) = self.0.parse(input)?;
            outputs.push(output);
            rest = next;
            latent = latent.merge(residual);
        }

        Ok((outputs, rest, latent))
    }
}

//---------- Sequence combinators ----------

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
                Err(Error::Recoverable(failure1)) => Err(Error::Recoverable(failure0.merge(failure1))),
                otherwise => otherwise,
            },
            otherwise => otherwise,
        }
    }
}

pub struct Then<A, B>(A, B);

impl<A, B> Parser for Then<A, B>
where
    A: Parser,
    B: Parser,
{
    type Output = (A::Output, B::Output);

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output0, rest, failure0) = self.0.parse(input)?;
        let (output1, rest, failure1) = self.1.parse(rest)?;

        Ok(((output0, output1), rest, failure0.merge(failure1)))
    }
}

pub struct ThenIgnore<A, B>(A, B);

impl<A, B> Parser for ThenIgnore<A, B>
where
    A: Parser,
    B: Parser,
{
    type Output = A::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, rest, failure0) = self.0.parse(input)?;
        let (_, rest, failure1) = self.1.parse(rest)?;

        Ok((output, rest, failure0.merge(failure1)))
    }
}

pub struct IgnoreThen<A, B>(A, B);

impl<A, B> Parser for IgnoreThen<A, B>
where
    A: Parser,
    B: Parser,
{
    type Output = B::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (_, rest, failure0) = self.0.parse(input)?;
        let (output, rest, failure1) = self.1.parse(rest)?;

        Ok((output, rest, failure0.merge(failure1)))
    }
}

pub struct Delimited<P, L, R>(P, L, R);

impl<P, L, R> Parser for Delimited<P, L, R>
where
    P: Parser,
    L: Parser,
    R: Parser,
{
    type Output = P::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (_, rest, failure0) = self.1.parse(input)?;
        let (output, rest, failure1) = self.0.parse(rest)?;
        let (_, rest, failure2) = self.2.parse(rest)?;

        Ok((output, rest, failure0.merge(failure1).merge(failure2)))
    }
}

//---------- Other combinators ----------

pub struct Map<P, F>(P, F);

impl<P, F, O> Parser for Map<P, F>
where
    P: Parser,
    F: Fn(P::Output) -> O,
{
    type Output = O;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, rest, failure) = self.0.parse(input)?;

        Ok((self.1(output), rest, failure))
    }
}

pub struct Optional<P>(P);

impl<P> Parser for Optional<P>
where
    P: Parser,
{
    type Output = Option<P::Output>;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Ok((output, rest, failure)) => Ok((Some(output), rest, failure)),
            Err(Error::Recoverable(failure)) => Ok((None, input, Some(failure))),
            Err(error) => Err(error),
        }
    }
}

pub struct Satisfy<P, F>(P, F);

impl<P, F> Parser for Satisfy<P, F>
where
    P: Parser,
    F: Fn(&P::Output) -> bool,
{
    type Output = P::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (output, rest, failure) = self.0.parse(input)?;

        if self.1(&output) {
            Ok((output, rest, failure))
        } else {
            Err(Failure::new(input, Expectation::Satisfy).recover())
        }
    }
}

pub struct Value<P, T>(P, T);

impl<P, T> Parser for Value<P, T>
where
    P: Parser,
    T: Clone,
{
    type Output = T;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Ok((_, rest, failure)) => Ok((self.1.clone(), rest, failure)),
            Err(error) => Err(error),
        }
    }
}
