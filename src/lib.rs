use std::result;

mod chars;
mod strings;
mod json;

#[derive(Debug, PartialEq)]
pub enum Error {
    Committed,
    Recoverable
}

pub enum Expectation {
    Anything,
    Satisfy,
    Just(char)
}

pub struct Context;

pub type Result<T> = result::Result<T, Error>;

pub struct State<'a> {
    source: &'a str,
    offset: usize,
    furthest: usize,
    expected: Vec<Expectation>
}

impl<'a> State<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            furthest: 0,
            expected: Vec::new()
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn goto(&mut self, offset: usize) {
        self.offset = offset;
    }

    pub fn advance(&mut self, n: usize) {
        self.offset += n;
    }

    pub fn remainder(&self) -> &str {
        &self.source[self.offset..]
    }

    pub fn peek(&self) -> Option<char> {
        self
            .remainder()
            .chars()
            .next()
    }

    pub fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.advance(ch.len_utf8());
        
        Some(ch)
    }

    pub fn fail(&mut self, expected: Expectation) -> Error {
        if self.offset > self.furthest {
            self.furthest = self.offset;
            self.expected.clear();
        }

        if self.offset == self.furthest {
            self.expected.push(expected);
        }

        Error::Recoverable
    }
}

pub trait Parser: Sized {
    type Output;

    fn parse(&self, state: &mut State<'_>) -> Result<Self::Output>;

    fn run(&self, input: &str) -> Result<Self::Output> {
        self.parse(&mut State::new(input))
    }

    fn cut(self) -> impl Parser<Output = Self::Output> {
        move |state: &mut State<'_>| {
            match self.parse(state) {
                Err(Error::Recoverable) => Err(Error::Committed),
                otherwise => otherwise
            }
        }
    }

    fn many(self) -> impl Parser<Output = Vec<Self::Output>> {
        move |state: &mut State<'_>| {
            let mut outputs = Vec::new();

            loop {
                let start = state.offset();

                match self.parse(state) {
                    Ok(output) => outputs.push(output),
                    Err(Error::Recoverable) => {
                        state.goto(start);
                        break;
                    },
                    Err(Error::Committed) => return Err(Error::Committed)
                }
            }

            Ok(outputs)
        }
    }

    fn many_once(self) -> impl Parser<Output = Vec<Self::Output>> {
        move |state: &mut State<'_>| {
            let start = state.offset();

            let mut outputs = match self.parse(state) {
                Ok(output) => vec![output],
                Err(Error::Recoverable) => {
                    state.goto(start);
                    return Err(Error::Recoverable);
                }
                Err(Error::Committed) => return Err(Error::Committed),
            };

            loop {
                let start = state.offset();

                match self.parse(state) {
                    Ok(output) => outputs.push(output),
                    Err(Error::Recoverable) => {
                        state.goto(start);
                        break;
                    }
                    Err(Error::Committed) => return Err(Error::Committed)
                }
            }

            Ok(outputs)
        }
    }

    fn map<F, B>(self, op: F) -> impl Parser<Output = B>
    where
        F: Fn(Self::Output) -> B
    {
        move |state: &mut State<'_>| {
            let start = state.offset();
            match self.parse(state) {
                Ok(output) => Ok(op(output)),
                Err(Error::Recoverable) => {
                    state.goto(start);
                    Err(Error::Recoverable)
                }
                Err(Error::Committed) => Err(Error::Committed)
            }
        }
    }

    fn optional(self) -> impl Parser<Output = Option<Self::Output>> {
        move |state: &mut State<'_>| {
            let start = state.offset();
            match self.parse(state) {
                Ok(output) => Ok(Some(output)),
                Err(Error::Recoverable) => {
                    state.goto(start);
                    Ok(None)
                }
                Err(Error::Committed) => Err(Error::Committed),
            }
        }
    }

    fn or<P>(self, other: P) -> impl Parser<Output = Self::Output>
    where
        P: Parser<Output = Self::Output>
    {
        move |state: &mut State<'_>| {
            let start = state.offset();
            
            match self.parse(state) {
                Err(Error::Recoverable) => {
                    state.goto(start);
                    other.parse(state)
                }
                otherwise => otherwise
            }
        }
    }

    fn repeat<const N: usize>(self) -> impl Parser<Output = [Self::Output; N]> {
        move |state: &mut State<'_>| {
            let mut outputs = Vec::with_capacity(N);

            for _ in 0..N {
                outputs.push(self.parse(state)?);
            }

            Ok(unsafe { outputs.try_into().unwrap_unchecked() })
        }
    }

    fn satisfy<F>(self, predicate: F) -> impl Parser<Output = Self::Output>
    where
        F: Fn(&Self::Output) -> bool
    {
        move |state: &mut State<'_>| {
            let start = state.offset();
            let output = self.parse(state)?;

            if predicate(&output) {
                Ok(output)
            } else {
                state.goto(start);
                Err(state.fail(Expectation::Satisfy))
            }
        }
    }

    fn then<P>(self, next: P) -> impl Parser<Output = (Self::Output, P::Output)>
    where
        P: Parser
    {
        move |state: &mut State<'_>| {
            let start = state.offset();
            let first = self.parse(state)?;

            match next.parse(state) {
                Ok(second) => Ok((first, second)),
                Err(Error::Recoverable) => {
                    state.goto(start);
                    Err(Error::Recoverable)
                }
                Err(Error::Committed) => Err(Error::Committed)
            }
        }
    }
    
    fn value<T>(self, what: T) -> impl Parser<Output = T> 
    where
        T: Clone
    {
        move |state: &mut State<'_>| {
            let start = state.offset();
            match self.parse(state) {
                Ok(_) => Ok(what.clone()),
                Err(Error::Recoverable) => {
                    state.goto(start);
                    Err(Error::Recoverable)
                }
                Err(Error::Committed) => Err(Error::Committed)
            }
        }
    }
}

impl<T, O> Parser for T
where
    T: Fn(&mut State) -> Result<O>
{
    type Output = O;

    fn parse(&self, state: &mut State<'_>) -> Result<Self::Output> {
        self(state)
    }
}