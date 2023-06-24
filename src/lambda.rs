extern crate alloc;
use alloc::{boxed::Box, fmt::Display};
use core::prelude::rust_2024::*;
use core::{char, fmt, matches, mem, u32, usize, write};
use once_cell::sync::Lazy;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Expr {
  Var(u32),
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

pub static ID : Lazy<Expr> = Lazy::new(|| lam(Var(0)));
// static ZERO: Expr = lam(lam(Var(0)));
pub static CONST : Lazy<Expr> = Lazy::new(|| lam(lam(Var(1))));
// static  ONE: Expr = lam(lam(app(Var(1), Var(0))));
pub static FORK : Lazy<Expr> =
  Lazy::new(|| lam(lam(lam(app(app(Var(2), Var(0)), app(Var(1), Var(0)))))));
pub static SUCC : Lazy<Expr> =
  Lazy::new(|| lam(lam(lam(app(Var(1), app(app(Var(2), Var(1)), Var(0)))))));
pub static PLUS : Lazy<Expr> = Lazy::new(|| {
  lam(lam(lam(lam(app(
    app(Var(3), Var(1)),
    app(app(Var(2), Var(1)), Var(0)),
  )))))
});
pub static TIMES : Lazy<Expr> =
  Lazy::new(|| lam(lam(lam(lam(app(app(Var(3), app(Var(2), Var(1))), Var(0)))))));
pub static POWER : Lazy<Expr> =
  Lazy::new(|| lam(lam(lam(lam(app(app(app(Var(2), Var(3)), Var(1)), Var(0)))))));

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
  pub fn closed(&self, v : u32) -> bool {
    match self {
      Var(u) => *u <= v,
      Lam(e) => e.closed(v + 1),
      App(l, r) => l.closed(v) && r.closed(v),
      _ => true,
    }
  }

  pub fn replace(&mut self, to : &Expr) {
    fn replace_(expr : &mut Expr, v : u32, to : &Expr, shift : u32) {
      fn shift_(expr : &mut Expr, v : u32, amount : u32) {
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
    fn unshift(expr : &mut Expr, v : u32) {
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

impl Expr {
  #[must_use]
  pub fn from_nat(n : u32) -> Expr {
    let mut ret = Var(0);
    for _ in 0..n {
      ret = app(Var(1), ret);
    }
    lam(lam(ret))
  }
  #[must_use]
  pub fn to_nat(&self) -> Option<u32> {
    let mut ret = 0u32;
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
const VAR_NUMERALS : [char; 11] = ['🄌', '➊', '➋', '➌', '➍', '➎', '➏', '➐', '➑', '➒', '➓'];
const VAR_LEAF : [char; 11] = ['🄋', '➀', '➁', '➂', '➃', '➄', '➅', '➆', '➇', '➈', '➉'];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LeafMode {
  No,
  Leaf,
  InputDot,
}

#[derive(Debug)]
pub struct DisplayStruct<'a> {
  pub expr : &'a Expr,
  pub cursor : &'a Expr,
  pub leaf_mode : LeafMode,
}

impl DisplayStruct<'_> {
  pub const CURSOR_START : char = '\u{e000}';
  pub const CURSOR_END : char = '\u{e001}';
}

impl Display for DisplayStruct<'_> {
  fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
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
            write!(
              f,
              "{}",
              if f.sign_aware_zero_pad() {
                VAR_LEAF[*u as usize]
              } else {
                VAR_NUMERALS[*u as usize]
              }
            )
          } else {
            write!(f, "[{u}]")
          }
        }
        Lam(e) => {
          if f.alternate() {
            write!(f, "(λ{:+})", DisplayStruct { expr : e, ..*self })
          } else {
            write!(f, "λ{:+}", DisplayStruct { expr : e, ..*self })
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
          if let Var(u) = self.cursor && self.leaf_mode == LeafMode::Leaf {
            write!(f, "{}", VAR_LEAF[*u as usize])
          } else if *self.cursor == Hole && self.leaf_mode == LeafMode::InputDot {
            write!(f, "⬤")
          } else {
            write!(f, "{}", Self::CURSOR_START)?;
            self.cursor.fmt(f)?;
            write!(f, "{}", Self::CURSOR_END)
          }
        }
      }
    }
  }
}

impl Display for Expr {
  #[inline]
  fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
    DisplayStruct {
      expr : self,
      cursor : &Hole,
      leaf_mode : LeafMode::No,
    }
    .fmt(f)
  }
}
