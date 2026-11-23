use gparse::Parser;

use crate::parser::json;

mod parser;

pub fn main() {
    let valid = json.parse(include_str!("../data/valid.json")).unwrap();
    dbg!(valid.0);

    let missing_colon = json
        .parse(include_str!("../data/missing_colon.json"))
        .unwrap_err();
    dbg!(missing_colon);

    let unterminated = json
        .parse(include_str!("../data/unterminated.json"))
        .unwrap_err();
    dbg!(unterminated);
}
