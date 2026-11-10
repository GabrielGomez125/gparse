use std::{cmp::Ordering, result};

pub enum Expectation {
    Anything,
}

pub struct Failure {
    remaining: usize,
    expected: Vec<Expectation>
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
            },
            Ordering::Greater => other,
            Ordering::Less => self
        }
    }
}

impl Merge for Option<Failure> {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Some(first), Some(second)) => Some(first.merge(second)),
            (first, None) => first,
            (None, second) => second
        }
    }
}

pub enum Error {
    Committed(Failure),
    Recoverable(Failure)    
}

pub type Result<'a, T> = result::Result<(T, &'a str, Option<Failure>), Error>;

pub trait Parser: Sized {
    type Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output>;

    fn many(self) -> Many<Self> {
        Many(self)
    }

    fn many_once(self) -> ManyOnce<Self> {
        ManyOnce(self)
    }

    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<Output = Self::Output>
    {
        Or(self, other)
    }

    fn then<P>(self, next: P) -> Then<Self, P>
    where
        P: Parser
    {
        Then(self, next)
    }
}

pub struct Many<P>(P);

impl<P> Parser for Many<P>
where
    P: Parser
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
    P: Parser
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

pub struct Or<A, B>(A, B);

impl<A, B> Parser for Or<A, B>
where
    A: Parser,
    B: Parser<Output = A::Output>
{
    type Output = A::Output;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        match self.0.parse(input) {
            Err(Error::Recoverable(first)) => match self.1.parse(input) {
                Ok((output, rest, latent)) => 
                    Ok((output, rest, Some(first).merge(latent))),

                Err(Error::Recoverable(second)) => Err(Error::Recoverable(first.merge(second))),

                otherwise => otherwise
            },

            otherwise => otherwise
        }
    }
}

pub struct Then<A, B>(A, B);

impl<A, B> Parser for Then<A, B>
where
    A: Parser,
    B: Parser
{
    type Output = (A::Output, B::Output);

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        let (first, rest, failure0) = self.0.parse(input)?;
        let (second, rest, failure1) = self.1.parse(rest)?;

        Ok(((first, second), rest, failure0.merge(failure1)))
    }
}

impl<T, O> Parser for T
where
    T: for<'a> Fn(&'a str) -> Result<'a, O>
{
    type Output = O;

    fn parse<'a>(&self, input: &'a str) -> Result<'a, Self::Output> {
        self(input)
    }
}