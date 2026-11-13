//! Parsers for digits, integers, and decimal numbers.
//!
//! Only ASCII decimal digits are recognized. The integer parsers do not accept
//! leading zeros: `"007"` parses as `0`, and `"07"` is left unconsumed.
//!
//! # Example
//!
//! ```
//! use gparse::Parser;
//! use gparse::chars::char;
//! use gparse::numbers::{integer, number};
//!
//! // A "key=value" pair where the value is a number.
//! let setting = integer.then_ignore(char('=')).then(number);
//!
//! let (output, rest, _) = setting.parse("3=-2.5").unwrap();
//! assert_eq!(output, (3, -2.5));
//! assert_eq!(rest, "");
//! ```

use crate::{
    chars::{char, one_of},
    Parser,
};

/// Matches the digit `'0'` and outputs `0`.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::zero;
///
/// assert_eq!(zero.parse("0").unwrap().0, 0);
/// assert!(zero.parse("1").is_err());
/// ```
pub fn zero<'a>(input: &'a str) -> crate::Result<'a, u8> {
    char('0').value(0).parse(input)
}

/// Matches one digit from `'1'` to `'9'` and outputs its value.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::nonzero_digit;
///
/// assert_eq!(nonzero_digit.parse("7").unwrap().0, 7);
/// assert!(nonzero_digit.parse("0").is_err());
/// ```
pub fn nonzero_digit<'a>(input: &'a str) -> crate::Result<'a, u8> {
    one_of("123456789")
        .map(|ch| ch.to_digit(10).unwrap() as u8)
        .parse(input)
}

/// Matches one digit from `'0'` to `'9'` and outputs its value.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::digit;
///
/// let (output, rest, _) = digit.many().parse("2024!").unwrap();
/// assert_eq!(output, vec![2, 0, 2, 4]);
/// assert_eq!(rest, "!");
///
/// assert_eq!(digit.parse("x").unwrap_err().to_string(), "Expected '0' or one of 123456789");
/// ```
pub fn digit<'a>(input: &'a str) -> crate::Result<'a, u8> {
    zero.or(nonzero_digit).parse(input)
}

/// Matches an integer greater than zero, such as `"42"`.
///
/// The first digit must be `'1'` to `'9'`. Every digit after it is consumed.
///
/// # Panics
///
/// In debug builds, panics if the value does not fit in a `u32`.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::positive_integer;
///
/// let (output, rest, _) = positive_integer.parse("1024px").unwrap();
/// assert_eq!(output, 1024);
/// assert_eq!(rest, "px");
///
/// // Zero is not positive.
/// assert!(positive_integer.parse("0").is_err());
/// ```
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

/// Matches `"0"` or a [`positive_integer`].
///
/// # Panics
///
/// In debug builds, panics if the value does not fit in a `u32`.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::nonnegative_integer;
///
/// assert_eq!(nonnegative_integer.parse("0").unwrap().0, 0);
/// assert_eq!(nonnegative_integer.parse("365").unwrap().0, 365);
///
/// // Leading zeros are not consumed.
/// let (output, rest, _) = nonnegative_integer.parse("007").unwrap();
/// assert_eq!(output, 0);
/// assert_eq!(rest, "07");
/// ```
pub fn nonnegative_integer<'a>(input: &'a str) -> crate::Result<'a, u32> {
    zero.map(u32::from).or(positive_integer).parse(input)
}

/// Matches an integer with an optional leading `'-'`, such as `"-17"`.
///
/// A leading `'+'` is not accepted.
///
/// # Panics
///
/// In debug builds, panics if the digits do not fit in a `u32`. Values that fit
/// in a `u32` but not in an `i32` wrap around.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::integer;
///
/// assert_eq!(integer.parse("42").unwrap().0, 42);
/// assert_eq!(integer.parse("-17").unwrap().0, -17);
/// assert_eq!(integer.parse("0").unwrap().0, 0);
///
/// assert!(integer.parse("+5").is_err());
/// ```
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

/// Matches a decimal number with an optional `'-'` and optional fractional
/// part, such as `"3"`, `"-0.5"`, or `"12.75"`.
///
/// The integer part follows the rules of [`nonnegative_integer`]. If a `'.'`
/// is present, at least one digit must follow it. Otherwise the `'.'` is not
/// consumed and only the integer part is parsed. Exponents are not accepted.
/// Use [`number_scientific`] for those.
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::number;
///
/// assert_eq!(number.parse("12.75").unwrap().0, 12.75);
/// assert_eq!(number.parse("-0.5").unwrap().0, -0.5);
/// assert_eq!(number.parse("-3").unwrap().0, -3.0);
///
/// // A '.' with no digits after it is left unconsumed.
/// let (output, rest, _) = number.parse("3.x").unwrap();
/// assert_eq!(output, 3.0);
/// assert_eq!(rest, ".x");
/// ```
pub fn number<'a>(input: &'a str) -> crate::Result<'a, f64> {
    char('-')
        .optional()
        .then(nonnegative_integer)
        .then(char('.').ignore_then(digit.many_once()).optional())
        .map(|((sign, int_part), decimal_part)| {
            let mut acc = int_part as f64;
            let mut pow = 0.1;

            for digit in decimal_part.into_iter().flatten() {
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

/// Matches a [`number`] with an optional exponent, such as `"6.02e23"` or
/// `"1.5E-3"`.
///
/// The exponent is an `'e'` or `'E'`, an optional `'+'` or `'-'`, and a
/// [`positive_integer`].
///
/// # Example
///
/// ```
/// use gparse::Parser;
/// use gparse::numbers::number_scientific;
///
/// assert_eq!(number_scientific.parse("1.5e3").unwrap().0, 1500.0);
/// assert_eq!(number_scientific.parse("-2E+2").unwrap().0, -200.0);
/// assert_eq!(number_scientific.parse("42").unwrap().0, 42.0);
///
/// let (output, _, _) = number_scientific.parse("25e-1").unwrap();
/// assert!((output - 2.5).abs() < 1e-12);
/// ```
pub fn number_scientific<'a>(input: &'a str) -> crate::Result<'a, f64> {
    number
        .then(
            one_of("Ee")
                .ignore_then(one_of("-+").optional().then(positive_integer))
                .optional(),
        )
        .map(|(number_part, exp_part)| {
            let exp = exp_part.map_or(0, |(sign, exp)| match sign {
                Some('-') => -(exp as i32),
                _ => exp as i32,
            });

            number_part * 10f64.powi(exp)
        })
        .parse(input)
}
