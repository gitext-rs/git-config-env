use std::borrow::Cow;

use itertools::Itertools;
use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;

pub fn sq_dequote_step(input: &str) -> IResult<&str, Cow<str>> {
    // See git's quote.c's `sq_dequote_step`
    alt((sq_dequote_escaped, sq_dequote_no_escaped))(input)
}

fn sq_dequote_escaped(input: &str) -> IResult<&str, Cow<str>> {
    map(
        tuple((
            sq_dequote_section,
            sq_dequote_trail,
            many0(sq_dequote_trail),
        )),
        |(start, trail, mut trails)| {
            trails.insert(0, trail);
            trails.insert(0, [start, ""]);
            let value = trails.into_iter().flatten().join("");
            Cow::Owned(value)
        },
    )(input)
}

fn sq_dequote_no_escaped(input: &str) -> IResult<&str, Cow<str>> {
    map(sq_dequote_section, Cow::Borrowed)(input)
}

fn sq_dequote_section(input: &str) -> IResult<&str, &str> {
    terminated(preceded(char('\''), take_while(|c| c != '\'')), char('\''))(input)
}

fn sq_dequote_trail(input: &str) -> IResult<&str, [&str; 2]> {
    map(pair(escaped, sq_dequote_section), |(e, s)| [e, s])(input)
}

fn escaped(input: &str) -> IResult<&str, &str> {
    preceded(char('\\'), alt((tag("'"), tag("!"))))(input)
}

#[cfg(test)]
mod test_sq_dequote_step {
    use super::*;

    #[test]
    fn word() {
        let fixture = "'name'";
        let expected = Cow::Borrowed("name");
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn space() {
        let fixture = "'a b'";
        let expected = Cow::Borrowed("a b");
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn sq_escaped() {
        let fixture = "'a'\\''b'";
        let expected: Cow<str> = Cow::Owned("a'b".into());
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn exclamation_escaped() {
        let fixture = "'a'\\!'b'";
        let expected: Cow<str> = Cow::Owned("a!b".into());
        let (_, actual) = sq_dequote_step(fixture).unwrap();
        assert_eq!(actual, expected);
    }
}
