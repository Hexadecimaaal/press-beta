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

#[inline]
#[must_use]
pub fn lam(e : Expr) -> Expr { Expr::Lam(Box::new(e)) }
#[inline]
#[must_use]
pub fn app(l : Expr, r : Expr) -> Expr { Expr::App(Box::new(l), Box::new(r)) }

use Expr::{App, Hole, Lam, Slot, Var};

lazy_static! {
  pub static ref ID: Expr = lam(Var(0));
  static ref ZERO: Expr = lam(lam(Var(0)));
  static ref CONST: Expr = lam(lam(Var(1)));
  static ref ONE: Expr = lam(lam(app(Var(1), Var(0))));
  static ref FORK: Expr = lam(lam(lam(app(app(Var(2), Var(0)), app(Var(1), Var(0)),))));
  pub static ref SUCC: Expr = lam(lam(lam(app(Var(1), app(app(Var(2), Var(1)), Var(0)),))));
  pub static ref PLUS: Expr = lam(lam(lam(lam(app(
    app(Var(3), Var(1)),
    app(app(Var(2), Var(1)), Var(0)),
  )))));
  pub static ref TIMES: Expr = lam(lam(lam(lam(
    app(app(Var(3), app(Var(2), Var(1))), Var(0),)
  ))));
  pub static ref POWER: Expr = lam(lam(lam(lam(
    app(app(app(Var(2), Var(3)), Var(1)), Var(0),)
  ))));
}

impl Expr {
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
      Lam(box Slot) | App(box Slot, _) | App(_, box Slot) => Some(self),
      Lam(e) => e.find_slot_parent(),
      App(l, r) => l.find_slot_parent().or_else(|| r.find_slot_parent()),
      _ => None,
    }
  }
  pub fn rightmost(&mut self) -> &mut Expr {
    match self {
      Var(_) | Hole | Slot => self,
      Lam(e) | App(_, e) => e.rightmost(),
    }
  }
  pub fn leftmost(&mut self) -> &mut Expr {
    match self {
      Var(_) | Hole | Slot => self,
      Lam(e) | App(e, _) => e.leftmost(),
    }
  }
  pub fn find_slot_leftsib(&mut self) -> Option<(&mut Expr, &mut Expr)> {
    match self {
      Var(_) | Hole | Slot => None,
      Lam(e) => e.find_slot_leftsib(),
      App(l, r) => {
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
      Lam(e) => e.find_slot_rightsib(),
      App(l, r) => {
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
        Lam(e) => e.find_redux(),
        App(l, r) => l.find_redux().or_else(|| r.find_redux()),
        _ => None,
      }
    }
  }
}

impl Expr {
  #[must_use]
  pub fn closed(&self, v : u8) -> bool {
    match self {
      Var(u) => *u <= v,
      Lam(e) => e.closed(v + 1),
      App(l, r) => l.closed(v) && r.closed(v),
      _ => true,
    }
  }

  pub fn replace(&mut self, to : &Expr) {
    fn replace_(expr : &mut Expr, v : u8, to : &Expr, shift : u8) {
      fn shift_(expr : &mut Expr, v : u8, amount : u8) {
        match expr {
          Var(u) => {
            if *u >= v {
              *u += amount;
            }
          }
          Lam(e) => shift_(e, v + 1, amount),
          App(l, r) => {
            shift_(l, v, amount);
            shift_(r, v, amount);
          }
          _ => {}
        }
      }
      match expr {
        Var(u) => {
          if *u == v {
            let mut new = to.clone();
            shift_(&mut new, 0, shift);
            *expr = new;
          }
        }
        Lam(e) => {
          if to.closed(0) {
            replace_(e, v + 1, to, shift);
          } else {
            replace_(e, v + 1, to, shift + 1);
          }
        }
        App(l, r) => {
          replace_(l, v, to, shift);
          replace_(r, v, to, shift);
        }
        Hole | Slot => {}
      }
    }
    replace_(self, 0, to, 0);
  }
  pub fn beta(&mut self) -> bool {
    fn unshift(expr : &mut Expr, v : u8) {
      match expr {
        Var(u) => {
          if *u >= v {
            *u -= 1;
          }
        }
        Lam(e) => unshift(e, v + 1),
        App(l, r) => {
          unshift(l, v);
          unshift(r, v);
        }
        _ => {}
      }
    }
    if let App(box Lam(e), box r) = self {
      unshift(e, 1);
      e.replace(r);
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
}

#[test]
fn test_beta() {
  let mut idid = app(ID.clone(), ID.clone());
  idid.beta();
  assert_eq!(idid, *ID);
  let mut free_beta = app(
    lam(lam(app(app(Var(3), Var(1)), lam(app(Var(0), Var(2)))))),
    lam(app(Var(4), Var(0))),
  );
  free_beta.beta();
  assert_eq!(
    free_beta,
    lam(app(
      app(Var(2), lam(app(Var(5), Var(0)))),
      lam(app(Var(0), lam(app(Var(6), Var(0)))))
    ))
  );
}

impl Expr {
  #[must_use]
  pub fn from_nat(n : u8) -> Expr {
    let mut ret = Var(0);
    for _ in 0..n {
      ret = app(Var(1), ret);
    }
    lam(lam(ret))
  }
  #[must_use]
  pub fn to_nat(&self) -> Option<u8> {
    let mut ret = 0u8;
    if let Lam(box Lam(box e)) = self {
      let mut e = e;
      while let App(box Var(1), box eprime) = e {
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
}

#[test]
fn test_to_nat() {
  assert_eq!(ZERO.to_nat(), Some(0u8));
  assert_eq!(ONE.to_nat(), Some(1u8));
}

const VAR_NUMERALS : [char; 11] = ['🄌', '➊', '➋', '➌', '➍', '➎', '➏', '➐', '➑', '➒', '➓'];

#[derive(Debug)]
pub struct DisplayStruct<'a> {
  pub expr : &'a Expr,
  pub cursor : &'a Expr,
  pub leaf_mode : bool,
}

impl Display for DisplayStruct<'_> {
  fn fmt(&self, f : &mut std::fmt::Formatter) -> std::fmt::Result {
    if let Some(n) = self.expr.to_nat() {
      write!(f, "{n}")
    } else {
      match self.expr {
        _ if *ID == *self.expr => write!(f, "I"),
        _ if *CONST == *self.expr => write!(f, "K"),
        _ if *FORK == *self.expr => write!(f, "S"),
        _ if *SUCC == *self.expr => write!(f, "SUCC"),
        _ if *PLUS == *self.expr => write!(f, "+"),
        _ if *TIMES == *self.expr => write!(f, "*"),
        _ if *POWER == *self.expr => write!(f, "^"),
        Var(u) => {
          if *u <= 10 {
            write!(f, "{}", VAR_NUMERALS[*u as usize])
          } else {
            write!(f, "[{u}]")
          }
        }
        Lam(e) => {
          if f.alternate() {
            write!(f, "(𝛌{:+})", DisplayStruct { expr : e, ..*self })
          } else {
            write!(f, "𝛌{:+}", DisplayStruct { expr : e, ..*self })
          }
        }
        App(l, r) => {
          if f.sign_plus() {
            write!(f, " ")?;
          }
          if f.alternate() {
            write!(f, "(")?;
          }
          match (&**l, self.cursor) {
            (Lam(_), _) | (Slot, Lam(_)) => write!(
              f,
              "{:#} {:#}",
              DisplayStruct { expr : l, ..*self },
              DisplayStruct { expr : r, ..*self }
            ),
            _ => write!(
              f,
              "{} {:#}",
              DisplayStruct { expr : l, ..*self },
              DisplayStruct { expr : r, ..*self }
            ),
          }?;
          if f.alternate() {
            write!(f, ")")
          } else {
            Ok(())
          }
        }
        Hole => write!(f, "▪"),
        Slot => {
          if self.leaf_mode {
            write!(f, "\x1b[4m")?;
            self.cursor.fmt(f)?;
            write!(f, "\x1b[24m")
          } else {
            write!(f, "\x1b[7m")?;
            self.cursor.fmt(f)?;
            write!(f, "\x1b[27m")
          }
        }
      }
    }
  }
}

impl Display for Expr {
  fn fmt(&self, f : &mut std::fmt::Formatter) -> std::fmt::Result {
    DisplayStruct {
      expr : self,
      cursor : &Hole,
      leaf_mode : false,
    }
    .fmt(f)
  }
}

#[test]
fn test_with_formatting() {
  assert_eq!(ID.to_string(), "I");
  assert_eq!(app(ID.clone(), ID.clone()).to_string(), "I I");
  assert_eq!(ZERO.to_string(), "0");
  assert_eq!(ONE.to_string(), "1");
  assert_eq!(Expr::from_nat(10).to_string(), "10");
  assert_eq!(lam(Expr::from_nat(10)).to_string(), "𝛌10");

  assert_eq!(PLUS.to_string(), "+");
  let mut p1 = app(PLUS.clone(), ONE.clone());
  assert_eq!(p1.to_string(), "+ 1");
  p1.hnf();
  assert_eq!(p1.to_string(), "𝛌𝛌𝛌 1 ➊ (➋ ➊ 🄌)");
  p1.nf();
  assert_eq!(p1.to_string(), "SUCC");
  let mut p11 = app(p1, ONE.clone());
  p11.nf();
  assert_eq!(p11.to_string(), "2");

  let mut pow24 = app(app(POWER.clone(), p11), Expr::from_nat(4));
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
