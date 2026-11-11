use std::{cmp::Ordering, result};

pub mod chars;
pub mod json;
pub mod numbers;
pub mod strings;

#[derive(Clone, Debug)]
pub enum Expectation {
    Anything,
    Satisfy,
    Char(char),
    Literal(String),
}

#[derive(Debug)]
pub struct Failure {
    remaining: usize,
    expected: Vec<Expectation>,
}

impl Failure {
    pub fn from_source(source: &str, expected: Expectation) -> Self {
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
            (Some(first), Some(second)) => Some(first.merge(second)),
            (first, None) => first,
            (None, second) => second,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Recoverable(Failure),
    Committed(Failure),
}

impl From<Failure> for Error {
    fn from(failure: Failure) -> Self {
        Self::Recoverable(failure)
    }
}

pub type Result<'a, T> = result::Result<(T, &'a str, Option<Failure>), Error>;

pub trait Parser: Sized {
    type Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output>;

    // Error combinators

    fn cut(self) -> Cut<Self> {
        Cut(self)
    }

    fn expect(self, expected: Expectation) -> Expect<Self> {
        Expect(self, expected)
    }

    // Repetition combinators

    fn many(self) -> Many<Self> {
        Many(self)
    }

    fn many_once(self) -> ManyOnce<Self> {
        ManyOnce(self)
    }

    // Branch combinators

    fn optional(self) -> Optional<Self> {
        Optional(self)
    }

    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<Output = Self::Output>,
    {
        Or(self, other)
    }

    // Sequence combinators

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

    fn delimited<L, R>(self, left: L, right: R) -> impl Parser<Output = Self::Output>
    where
        L: Parser,
        R: Parser,
    {
        left.ignore_then(self).then_ignore(right)
    }

    // Other combinators

    fn map<F, O>(self, op: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> O,
    {
        Map(self, op)
    }

    fn satisfy<F>(self, predicate: F) -> Satisfy<Self, F>
    where
        F: Fn(&Self::Output) -> bool,
    {
        Satisfy(self, predicate)
    }

    fn value<T>(self, what: T) -> impl Parser<Output = T>
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
            Err(Error::Recoverable(failure)) => {
                Err(Error::Recoverable(failure.expect(self.1.clone())))
            }
            Err(Error::Committed(failure)) => Err(Error::Committed(failure.expect(self.1.clone()))),
            otherwise => otherwise,
        }
    }
}

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

        while let Ok((output, next, failure)) = self.0.parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(failure);
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

        while let Ok((output, next, failure)) = self.0.parse(rest) {
            outputs.push(output);
            rest = next;
            latent = latent.merge(failure);
        }

        Ok((outputs, rest, latent))
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
            Err(failure) => Err(failure),
        }
    }
}

pub struct Or<A, B>(A, B);

impl<A, B> Parser for Or<A, B>
where
    A: Parser,
    B: Parser<Output = A::Output>,
{
    type Output = A::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Err(Error::Recoverable(first)) => match self.1.parse(input) {
                Ok((output, rest, latent)) => Ok((output, rest, Some(first).merge(latent))),
                Err(Error::Recoverable(second)) => Err(Error::Recoverable(first.merge(second))),
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
        let (first, rest, failure0) = self.0.parse(input)?;
        let (second, rest, failure1) = self.1.parse(rest)?;

        Ok(((first, second), rest, failure0.merge(failure1)))
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
            Err(Failure::from_source(input, Expectation::Satisfy).into())
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
            Err(failure) => Err(failure),
        }
    }
}
