use lazy_static::lazy_static;
use std::{fmt::Display, mem};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Expr {
  Var(u8),
  Lam(Box<Expr>),
  App(Box<Expr>, Box<Expr>),
  /// input logic: use this as default state and overwrite whenever possible
  #[default]
  Hole,
  Slot,
}

use Expr::*;

lazy_static! {
  pub static ref ID: Expr = Lam(box Var(0));
  static ref ZERO: Expr = Lam(box Lam(box Var(0)));
  static ref CONST: Expr = Lam(box Lam(box (Var(1))));
  static ref ONE: Expr = Lam(box Lam(box App(box Var(1), box Var(0))));
  static ref FORK: Expr = Lam(box Lam(box Lam(box App(
    box App(box Var(2), box Var(0)),
    box App(box Var(1), box Var(0)),
  ))));
  pub static ref SUCC: Expr = Lam(box Lam(box Lam(box App(
    box Var(1),
    box App(box App(box Var(2), box Var(1)), box Var(0)),
  ))));
  pub static ref PLUS: Expr = Lam(box Lam(box Lam(box Lam(box App(
    box App(box Var(3), box Var(1)),
    box App(box App(box Var(2), box Var(1)), box Var(0)),
  )))));
  pub static ref TIMES: Expr = Lam(box Lam(box Lam(box Lam(box App(
    box App(box Var(3), box App(box Var(2), box Var(1))),
    box Var(0),
  )))));
  pub static ref POWER: Expr = Lam(box Lam(box Lam(box Lam(box App(
    box App(box App(box Var(2), box Var(3)), box Var(1)),
    box Var(0),
  )))));
}

impl Expr {
  pub fn shift(&self, v : u8) -> Expr {
    match self {
      Var(u) => {
        if *u >= v {
          Var(u + 1)
        } else {
          Var(*u)
        }
      }
      Lam(e) => Lam(box e.shift(v + 1)),
      App(l, r) => App(box l.shift(v), box r.shift(v)),
      _ => self.clone(),
    }
  }
  pub fn closed(&self, v : u8) -> bool {
    match self {
      Var(u) => *u <= v,
      Lam(e) => e.closed(v + 1),
      App(l, r) => l.closed(v) && r.closed(v),
      _ => true,
    }
  }
  pub fn unshift(&mut self, v : u8) {
    match self {
      Var(u) => {
        if *u >= v {
          *u = *u - 1
        }
      }
      Lam(e) => e.unshift(v + 1),
      App(l, r) => {
        l.unshift(v);
        r.unshift(v)
      }
      _ => {}
    }
  }
  pub fn replace(&mut self, v : u8, to : &Expr) {
    match self {
      Var(u) => {
        if *u == v {
          *self = to.clone()
        }
      }
      Lam(e) => {
        if to.closed(0) {
          e.replace(v + 1, to)
        } else {
          e.replace(v + 1, &to.shift(0))
        }
      }
      App(l, r) => {
        l.replace(v, to);
        r.replace(v, to)
      }
      Hole | Slot => {}
    }
  }

  /// returns Some if replace FAILED
  pub fn replace_slot(&mut self, to : Expr) -> Option<Expr> {
    match self {
      Var(_) | Hole => Some(to),
      Slot => {
        *self = to;
        None
      }
      Lam(e) => e.replace_slot(to),
      App(l, r) => l.replace_slot(to).and_then(|to| r.replace_slot(to)),
    }
  }
  pub fn find_slot_parent(&mut self) -> Option<&mut Expr> {
    match self {
      Lam(box Slot) => Some(self),
      Lam(box e) => e.find_slot_parent(),
      App(box Slot, _) | App(_, box Slot) => Some(self),
      App(box l, box r) => l.find_slot_parent().or_else(|| r.find_slot_parent()),
      _ => None,
    }
  }
  pub fn rightmost(&mut self) -> &mut Expr {
    match self {
      Var(_) | Hole | Slot => self,
      Lam(box e) | App(_, box e) => e.rightmost(),
    }
  }
  pub fn leftmost(&mut self) -> &mut Expr {
    match self {
      Var(_) | Hole | Slot => self,
      Lam(box e) | App(box e, _) => e.leftmost(),
    }
  }
  pub fn find_slot_leftsib(&mut self) -> Option<(&mut Expr, &mut Expr)> {
    match self {
      Var(_) | Hole | Slot => None,
      Lam(box e) => e.find_slot_leftsib(),
      App(box l, box r) => {
        if r.leftmost() == &Slot {
          Some((r.leftmost(), l.rightmost()))
        } else {
          l.find_slot_leftsib().or_else(|| r.find_slot_leftsib())
        }
      }
    }
  }
  pub fn find_slot_rightsib(&mut self) -> Option<(&mut Expr, &mut Expr)> {
    match self {
      Var(_) | Hole | Slot => None,
      Lam(box e) => e.find_slot_rightsib(),
      App(box l, box r) => {
        if l.rightmost() == &Slot {
          Some((l.rightmost(), r.leftmost()))
        } else {
          l.find_slot_rightsib().or_else(|| r.find_slot_rightsib())
        }
      }
    }
  }
  pub fn head(&mut self) -> Option<&mut Expr> {
    if matches!(self, App(box Lam(_), _)) {
      Some(self)
    } else if let App(l, _) = self {
      l.head()
    } else {
      None
    }
  }
  pub fn find_redux(&mut self) -> Option<&mut Expr> {
    if matches!(self, App(box Lam(_), _)) {
      Some(self)
    } else {
      match self {
        Lam(box e) => e.find_redux(),
        App(box l, box r) => l.find_redux().or_else(|| r.find_redux()),
        _ => None,
      }
    }
  }
  pub fn beta(&mut self) -> bool {
    if let App(box Lam(e), r) = self {
      e.unshift(1);
      e.replace(0, &r);
      *self = mem::take(e);
      true
    } else {
      false
    }
  }
  pub fn hnf(&mut self) {
    while let Some(head) = self.head() {
      head.beta();
    }
  }

  pub fn nf(&mut self) {
    while let Some(redox) = self.find_redux() {
      redox.beta();
    }
  }

  pub fn to_nat(&self) -> Option<u8> {
    let mut ret = 0u8;
    if let Lam(box Lam(box e)) = self {
      let mut e = e;
      while let App(box Var(1), eprime) = e {
        ret += 1;
        e = eprime;
      }
      if Var(0) == *e {
        Some(ret)
      } else {
        None
      }
    } else {
      None
    }
  }

  pub fn from_nat(n : u8) -> Expr {
    let mut ret = Var(0);
    for _ in 0..n {
      ret = App(box Var(1), box ret);
    }
    Lam(box Lam(box ret))
  }
}

#[test]
fn test_to_nat() {
  assert_eq!(ZERO.to_nat(), Some(0u8));
  assert_eq!(ONE.to_nat(), Some(1u8));
}

#[test]
fn test_shift() {
  assert_eq!(ID.clone().shift(0), *ID);
  assert_eq!(ZERO.clone().shift(0), *ZERO);
  assert_eq!(Var(0).shift(0), Var(1));
  assert_eq!(ONE.clone().shift(0), *ONE);
}

#[test]
fn test_beta() {
  let mut idid = App(box ID.clone(), box ID.clone());
  idid.beta();
  assert_eq!(idid, *ID);
  let mut free_beta = App(
    box Lam(box Lam(box App(
      box App(box Var(3), box Var(1)),
      box Lam(box App(box Var(0), box Var(2))),
    ))),
    box Lam(box App(box Var(4), box Var(0))),
  );
  free_beta.beta();
  assert_eq!(
    free_beta,
    Lam(box App(
      box App(box Var(2), box Lam(box App(box Var(5), box Var(0)))),
      box Lam(box App(
        box Var(0),
        box Lam(box App(box Var(6), box Var(0)))
      ))
    ))
  )
}

const VAR_NUMERALS : [char; 11] = ['🄌', '➊', '➋', '➌', '➍', '➎', '➏', '➐', '➑', '➒', '➓'];

impl Display for Expr {
  fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(n) = self.to_nat() {
      write!(f, "{}", n)
    } else {
      match self {
        _ if *ID == *self => write!(f, "I"),
        _ if *CONST == *self => write!(f, "K"),
        _ if *FORK == *self => write!(f, "S"),
        _ if *SUCC == *self => write!(f, "SUCC"),
        _ if *PLUS == *self => write!(f, "+"),
        _ if *TIMES == *self => write!(f, "*"),
        _ if *POWER == *self => write!(f, "^"),
        Var(u) => {
          if *u <= 10 {
            write!(f, "{}", VAR_NUMERALS[*u as usize])
          } else {
            write!(f, "[{}]", *u)
          }
        }
        Lam(e) => {
          if f.alternate() {
            write!(f, "(𝛌{:+})", e)
          } else {
            write!(f, "𝛌{:+}", e)
          }
        }
        App(box l, box r) => {
          if f.sign_plus() {
            write!(f, " ")?
          }
          if f.alternate() {
            write!(f, "(")?
          }
          match (l, r) {
            (Lam(_), _) => write!(f, "{:#} {:#}", l, r),
            (..) => write!(f, "{} {:#}", l, r),
          }?;
          if f.alternate() {
            write!(f, ")")
          } else {
            Ok(())
          }
        }
        Hole => write!(f, "▪"),
        Slot => {
          write!(f, "_")?;
          if f.sign_plus() {
            write!(f, "+")?;
          }
          if f.alternate() {
            write!(f, "#")?;
          }
          write!(f, "_")
        }
      }
    }
  }
}

#[test]
fn test_with_formatting() {
  assert_eq!(ID.to_string(), "I");
  assert_eq!(App(box ID.clone(), box ID.clone()).to_string(), "I I");
  assert_eq!(ZERO.to_string(), "0");
  assert_eq!(ONE.to_string(), "1");
  assert_eq!(Expr::from_nat(10).to_string(), "10");
  assert_eq!(Lam(box Expr::from_nat(10)).to_string(), "𝛌10");

  assert_eq!(PLUS.to_string(), "+");
  let mut p1 = App(box PLUS.clone(), box ONE.clone());
  assert_eq!(p1.to_string(), "+ 1");
  p1.hnf();
  assert_eq!(p1.to_string(), "𝛌𝛌𝛌 1 ➊ (➋ ➊ 🄌)");
  p1.nf();
  assert_eq!(p1.to_string(), "SUCC");
  let mut p11 = App(box p1, box ONE.clone());
  p11.nf();
  assert_eq!(p11.to_string(), "2");

  let mut pow24 = App(box App(box POWER.clone(), box p11), box Expr::from_nat(4));
  assert_eq!(pow24.to_string(), "^ 2 4");
  pow24.head().unwrap().beta();
  assert_eq!(pow24.to_string(), "(𝛌𝛌𝛌 ➋ 2 ➊ 🄌) 4");
  pow24.beta();
  assert_eq!(pow24.to_string(), "𝛌𝛌 4 2 ➊ 🄌");
  pow24.find_redux().unwrap().beta();
  assert_eq!(pow24.to_string(), "𝛌𝛌 (𝛌 2 (2 (2 (2 🄌)))) ➊ 🄌");
  pow24.nf();
  assert_eq!(pow24.to_string(), "16");
}
