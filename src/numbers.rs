use crate::{Parser, chars::{char, one_of}};

fn zero() -> impl Parser<Output = u8> {
    char('0')
    .value(0)
}

fn nonzero_digit() -> impl Parser<Output = u8> {
    one_of("123456789")
    .map(|ch| ch.to_digit(10).unwrap() as u8)
}

fn digit() -> impl Parser<Output = u8> {
    zero()
    .or(nonzero_digit())
}

fn positive_integer() -> impl Parser<Output = u32> {
    nonzero_digit()
    .then(digit().many())
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
}

fn nonnegative_integer() -> impl Parser<Output = u32> {
    zero()
    .map(u32::from)
    .or(positive_integer())
}

fn integer() -> impl Parser<Output = i32> {
    char('-')
    .optional()
    .then(nonnegative_integer())
    .map(|(sign, int)| match sign {
        Some(_) => -(int as i32),
        None => int as i32
    })
}

fn number() -> impl Parser<Output = f64> {
    char('-')
    .optional()
    .then(nonnegative_integer())
    .then(
        char('.')
        .discard_to(
            digit()
            .many_once()
        )
        .optional()
    )
    .map(|((sign, int_part), decimal_part)| {
        let mut acc = int_part as f64;
        let mut pow = 0.1;

        let Some(digits) = decimal_part else { return acc; };

        for digit in digits.into_iter() {
            acc += digit as f64 * pow;
            pow *= 0.1;
        }

        match sign {
            Some(_) => -acc,
            None => acc
        }
    })
}

fn number_scientific() -> impl Parser<Output = f64> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balls() {
        let x = integer().parse("0.123");
        let y = number().parse("0.1234");

        println!("{:?}", x);
        println!("{:?}", y);
    }
}