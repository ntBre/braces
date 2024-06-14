use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::io;

use nom::bytes::complete::{is_not, take_while1};
use nom::character::complete::{space0, space1};
use nom::multi::separated_list1;
use nom::sequence::tuple;
use nom::AsChar;
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
    context("element", is_not(":"))(s)
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

#[derive(Debug)]
pub struct Smiles<'a> {
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
    exprs: &'b mut [Expr<'a>],
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
    pub fn atoms(&self) -> Vec<&usize> {
        get_atoms(&self.exprs).into_iter().map(|p| p.1).collect()
    }

    fn atoms_mut(&mut self) -> Vec<&mut usize> {
        get_atoms_mut(&mut self.exprs)
            .into_iter()
            .map(|p| p.1)
            .collect()
    }
}

fn parse_line(
    s: &str,
) -> Result<(&str, Smiles, Vec<usize>), Box<dyn Error + '_>> {
    let (rest, got) = tuple((
        take_while1(AsChar::is_alphanum),
        space1,
        smiles,
        space1,
        delimited(
            char('('),
            separated_list1(tuple((tag(","), space0)), digit1),
            char(')'),
        ),
    ))(s)?;
    assert!(rest.is_empty(), "{}", rest);
    let (pid, _space, exprs, _space2, tors) = got;
    let tors: Vec<usize> =
        tors.into_iter().map(|s| s.parse().unwrap()).collect();
    Ok((pid, Smiles { exprs }, tors))
}

fn main() {
    loop {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        if !buf.is_empty() {
            let Ok((pid, mut smiles, tors)) = parse_line(buf.trim()) else {
                eprintln!("error");
                continue;
            };
            let mut atoms = smiles.atoms_mut();

            // this gives a sequence of (idx, atom_idx) pair. I should sort by
            // atom_idx, then go back through by idx setting atom_idx to an
            // incrementing counter. cloning here because we don't want to
            // modify atoms yet
            let mut pairs: Vec<(usize, usize)> =
                atoms.iter().map(|u| **u).enumerate().collect();
            pairs.sort_by_key(|p| p.1);

            // map between the old and new numbering scheme for fixing torsion
            // indices
            let mut atom_map = HashMap::new();

            let mut atom_idx = 1;
            for (idx, _) in pairs {
                atom_map.insert(*atoms[idx], atom_idx);
                *atoms[idx] = atom_idx;
                atom_idx += 1;
            }

            print!("{pid} {smiles} (");
            let tl = tors.len();
            for (i, t) in tors.into_iter().enumerate() {
                print!("{}", atom_map[&(t + 1)] - 1);
                if i < tl - 1 {
                    print!(", ");
                }
            }
            println!(")");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let smi = std::fs::read_to_string("test.smi")
            .unwrap()
            .trim()
            .to_string();
        let got = Smiles::try_from(smi.as_str()).unwrap();
        assert_eq!(got.to_string(), smi);
    }

    #[test]
    fn parse_line() {
        let line = "t146j [C:1]1([H:31])=[N:2][C:3]([C:4]([C:5]([C:6](/[N:7]=[S:8](\\[N:9]([C:10]([C:11]([C:12]([N:13]([c:14]2[n:15][c:16]([H:45])[c:17]([H:46])[c:18]([H:47])[c:19]2[H:48])[C:20]([c:21]2[c:22]([H:51])[c:23]([H:52])[c:24]([Br:25])[c:26]([H:53])[c:27]2[H:54])([H:49])[H:50])([H:43])[H:44])([H:41])[H:42])([H:39])[H:40])[H:38])[C:28]([H:55])([H:56])[H:57])([H:36])[H:37])([H:34])[H:35])([H:32])[H:33])=[C:29]([H:58])[N:30]1[H:59] (9, 8, 7, 27)";
        super::parse_line(line).unwrap();
    }
}
