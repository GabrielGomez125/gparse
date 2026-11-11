use crate::{
    chars::{char, one_of},
    Parser,
};

pub fn zero<'a>(input: &'a str) -> crate::Result<'a, u8> {
    char('0').value(0).parse(input)
}

pub fn nonzero_digit<'a>(input: &'a str) -> crate::Result<'a, u8> {
    one_of("123456789")
        .map(|ch| ch.to_digit(10).unwrap() as u8)
        .parse(input)
}

pub fn digit<'a>(input: &'a str) -> crate::Result<'a, u8> {
    zero.or(nonzero_digit).parse(input)
}

pub fn positive_integer<'a>(input: &'a str) -> crate::Result<'a, u32> {
    nonzero_digit
        .then(digit.many())
        .map(|(first, rest)| {
            let mut acc = 0u32;
            let mut pow = 1u32;

            for digit in rest.into_iter().rev() {
                acc += digit as u32 * pow;
                pow *= 10;
            }

            acc += first as u32 * pow;
            acc
        })
        .parse(input)
}

pub fn nonnegative_integer<'a>(input: &'a str) -> crate::Result<'a, u32> {
    zero.map(u32::from).or(positive_integer).parse(input)
}

pub fn integer<'a>(input: &'a str) -> crate::Result<'a, i32> {
    char('-')
        .optional()
        .then(nonnegative_integer)
        .map(|(sign, int)| match sign {
            Some(_) => -(int as i32),
            None => int as i32,
        })
        .parse(input)
}

pub fn number<'a>(input: &'a str) -> crate::Result<'a, f64> {
    char('-')
        .optional()
        .then(nonnegative_integer)
        .then(char('.').ignore_then(digit.many_once()).optional())
        .map(|((sign, int_part), decimal_part)| {
            let mut acc = int_part as f64;
            let mut pow = 0.1;

            let Some(digits) = decimal_part else {
                return acc;
            };

            for digit in digits.into_iter() {
                acc += digit as f64 * pow;
                pow *= 0.1;
            }

            match sign {
                Some(_) => -acc,
                None => acc,
            }
        })
        .parse(input)
}

pub fn number_scientific<'a>(input: &'a str) -> crate::Result<'a, f64> {
    number
        .then(
            one_of("Ee")
                .ignore_then(one_of("-+").optional().then(positive_integer))
                .optional(),
        )
        .map(|(number_part, exp_part)| {
            let exp = exp_part.map_or(1, |(sign, exp)| match sign {
                Some('-') => -(exp as i32),
                _ => exp as i32,
            });

            number_part * 10f64.powi(exp)
        })
        .parse(input)
}
