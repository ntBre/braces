use std::fs::read_to_string;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1},
    combinator::opt,
    multi::{many0, many1},
    sequence::{delimited, pair, tuple},
    IResult,
};

fn element(s: &str) -> IResult<&str, &str> {
    alt((tag("H"), tag("He"), tag("C")))(s)
}

fn real_atom(s: &str) -> IResult<&str, Atom> {
    delimited(char('['), tuple((element, tag(":"), digit1)), char(']'))(s)
        .map(|(inp, tup)| (inp, Atom::Atom(tup)))
}

fn label(s: &str) -> IResult<&str, Atom> {
    digit1(s).map(|(inp, d)| (inp, Atom::Label(d)))
}

fn atom(s: &str) -> IResult<&str, Atom> {
    alt((real_atom, label))(s)
}

fn bond(s: &str) -> IResult<&str, &str> {
    alt((
        tag("."),
        tag("-"),
        tag("="),
        tag("#"),
        tag("$"),
        tag(":"),
        tag("/"),
        tag("\\"),
    ))(s)
}

#[allow(unused)]
#[derive(Debug)]
enum Atom<'a> {
    Atom((&'a str, &'a str, &'a str)),
    Label(&'a str),
}

type Bond<'a> = &'a str;

type Molecule<'a> = Vec<(Option<Bond<'a>>, Atom<'a>)>;

/// I guess at any position there is not just an ATOM or BOND, there can also be
/// a BRANCH, which is itself a delimited sequence of ATOM and BOND:
///
/// (ATOM | BRANCH)
fn smiles(s: &str) -> IResult<&str, Molecule> {
    many0(pair(opt(bond), atom))(s)
}

/// a smiles is an atom followed by additional bond, atom pairs, but the
/// explicit bond is optional (indicating a single bond)
///
/// TODO where do ring labels go? they're like atoms I think
///
/// SMILES := ATOM [[BOND] ATOM]*
fn main() {
    let smi = read_to_string("test.smi").unwrap().trim().to_string();
    dbg!(&smi);
    dbg!(smiles(&smi).unwrap());
}
