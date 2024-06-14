use std::{fmt::Display, fs::read_to_string};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1},
    error::context,
    multi::many1,
    sequence::{delimited, separated_pair},
    IResult,
};

fn element(s: &str) -> IResult<&str, &str> {
    context(
        "element",
        alt((
            tag("H"),
            tag("He"),
            tag("B"),
            tag("C"),
            tag("N"),
            tag("O"),
            tag("F"),
            tag("Ne"),
            tag("S"),
            tag("c"),
            tag("n"),
        )),
    )(s)
}

fn atom(s: &str) -> IResult<&str, Expr> {
    context(
        "atom",
        delimited(
            char('['),
            separated_pair(element, tag(":"), digit1),
            char(']'),
        ),
    )(s)
    .map(|(inp, (sym, idx))| (inp, Expr::Atom(sym, idx.parse().unwrap())))
}

fn label(s: &str) -> IResult<&str, Expr> {
    context("label", digit1)(s).map(|(inp, d)| (inp, Expr::Label(d)))
}

fn bond(s: &str) -> IResult<&str, Expr> {
    context(
        "bond",
        alt((
            tag("."),
            tag("-"),
            tag("="),
            tag("#"),
            tag("$"),
            tag(":"),
            tag("/"),
            tag("\\"),
        )),
    )(s)
    .map(|(i, o)| (i, Expr::Bond(o)))
}

// let me just simplify this for now. at each position, I can have an ATOM, a
// BOND, a LABEL, or a BRANCH, where a BRANCH is itself a delimited sequence of
// ATOM | BOND | LABEL | BRANCH

#[allow(unused)]
#[derive(Debug)]
enum Expr<'a> {
    Atom(&'a str, usize),
    Bond(&'a str),
    Label(&'a str),
    Branch(Vec<Expr<'a>>),
}

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Atom(sym, num) => write!(f, "[{sym}:{num}]"),
            Expr::Bond(s) => write!(f, "{s}"),
            Expr::Label(l) => write!(f, "{l}"),
            Expr::Branch(exprs) => {
                write!(f, "(")?;
                for expr in exprs {
                    write!(f, "{expr}")?;
                }
                write!(f, ")")
            }
        }
    }
}

fn branch(s: &str) -> IResult<&str, Expr> {
    context("branch", delimited(char('('), smiles, char(')')))(s)
        .map(|(i, o)| (i, Expr::Branch(o)))
}

fn smiles(s: &str) -> IResult<&str, Vec<Expr>> {
    context("smiles", many1(alt((atom, bond, label, branch))))(s)
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
    let (rest, got) = smiles(&smi).unwrap();
    assert!(rest.is_empty());

    println!("output:");
    let mut s = String::new();
    use std::fmt::Write;
    for g in got {
        write!(s, "{g}").unwrap();
    }
    assert_eq!(s, smi);
    println!("{s}");
}
