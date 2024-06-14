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
    // this is safe to unwrap because we know it's only digits
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

struct Smiles<'a> {
    exprs: Vec<Expr<'a>>,
}

impl<'a> TryFrom<&'a str> for Smiles<'a> {
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (rest, got) = smiles(value)?;
        if !rest.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                rest,
                nom::error::ErrorKind::TooLarge,
            )));
        }
        Ok(Self { exprs: got })
    }
}

impl Display for Smiles<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for g in &self.exprs {
            write!(f, "{g}")?;
        }
        Ok(())
    }
}

#[allow(unused)]
fn get_atoms<'a>(exprs: &'a Vec<Expr<'a>>) -> Vec<(&'a &'a str, &'a usize)> {
    let mut ret = Vec::new();
    for e in exprs {
        match e {
            Expr::Atom(s, u) => ret.push((s, u)),
            Expr::Bond(_) => (),
            Expr::Label(_) => (),
            Expr::Branch(b) => ret.extend(get_atoms(b)),
        }
    }
    ret
}

fn get_atoms_mut<'a, 'b>(
    exprs: &'b mut Vec<Expr<'a>>,
) -> Vec<(&'b mut &'a str, &'b mut usize)> {
    let mut ret = Vec::new();
    for e in exprs.iter_mut() {
        match e {
            Expr::Atom(s, u) => ret.push((s, u)),
            Expr::Bond(_) => (),
            Expr::Label(_) => (),
            Expr::Branch(b) => ret.extend(get_atoms_mut(b)),
        }
    }
    ret
}

impl<'a> Smiles<'a> {
    #[allow(unused)]
    fn atoms(&self) -> Vec<&usize> {
        get_atoms(&self.exprs).into_iter().map(|p| p.1).collect()
    }

    fn atoms_mut<'b>(&'b mut self) -> Vec<&'b mut usize> {
        get_atoms_mut(&mut self.exprs)
            .into_iter()
            .map(|p| p.1)
            .collect()
    }
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
    let mut got = Smiles::try_from(smi.as_str()).unwrap();

    // check that the initial parse worked
    assert_eq!(got.to_string(), smi);

    // I have 49 atoms, but the atom indices go from 1 to 59. I want to BOTH
    // bring the values above 49 back into range AND make the numbers continuous

    let mut atoms = got.atoms_mut();

    // this gives a sequence of (idx, atom_idx) pair. I should sort by atom_idx,
    // then go back through by idx setting atom_idx to an incrementing counter.
    // cloning here because we don't want to modify atoms yet
    let mut pairs: Vec<(usize, usize)> =
        atoms.iter().map(|u| **u).enumerate().collect();
    pairs.sort_by_key(|p| p.1);

    let mut atom_idx = 1;
    for (idx, _) in pairs {
        *atoms[idx] = atom_idx;
        atom_idx += 1;
    }

    dbg!(atoms.len());
    dbg!(atoms);

    println!("output:");
    println!("{}", got.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let smi = read_to_string("test.smi").unwrap().trim().to_string();
        let got = Smiles::try_from(smi.as_str()).unwrap();
        assert_eq!(got.to_string(), smi);
    }
}
