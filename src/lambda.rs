use core::panic;
use lazy_static::lazy_static;
use std::{fmt::Display, mem};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
enum Expr {
  Var(u8),
  Lam(Box<Expr>),
  App(Box<Expr>, Box<Expr>),
  #[default] Hole
}

lazy_static! {
  static ref ID: Expr = Expr::Lam(box Expr::Var(0));
  static ref ZERO: Expr = Expr::Lam(box Expr::Lam(box Expr::Var(0)));
  static ref ONE: Expr = Expr::Lam(box Expr::Lam(box Expr::App(
    box Expr::Var(1),
    box Expr::Var(0),
  )));
  static ref PLUS: Expr = Expr::Lam(box Expr::Lam(box Expr::Lam(box Expr::Lam(box Expr::App(
    box Expr::App(box Expr::Var(3), box Expr::Var(1)),
    box Expr::App(
      box Expr::App(box Expr::Var(2), box Expr::Var(1)),
      box Expr::Var(0),
    ),
  )))));
}

impl Expr {
  fn replace(&mut self, v : u8, to : &Expr) {
    match self {
      Expr::Var(u) => {
        if *u == v {
          *self = to.clone()
        }
      }
      Expr::Lam(e) => e.replace(v + 1, to),
      Expr::App(l, r) => {
        l.replace(v, to);
        r.replace(v, to)
      }
      Expr::Hole => {}
    }
  }
  fn head(&mut self) -> Option<&mut Expr> {
    if matches!(self, Expr::App(box Expr::Lam(_), _)) {
      Some(self)
    } else if let Expr::App(l, _) = self {
      l.head()
    } else {
      None
    }
  }
  pub fn beta(&mut self) {
    if let Expr::App(box Expr::Lam(e), r) = self {
      e.replace(0, &r);
      *self = mem::take(e);
    } else {
      panic!("{:?} does not beta", self)
    }
  }
  pub fn hnf(&mut self) {
    while let Some(head) = self.head() {
      head.beta();
    }
  }

  fn to_nat(&self) -> Option<u8> {
    let mut ret = 0u8;
    if let Expr::Lam(box Expr::Lam(box e)) = self {
      let mut e = e;
      while let Expr::App(box Expr::Var(1), eprime) = e {
        ret += 1;
        e = eprime;
      }
      if Expr::Var(0) == *e {
        Some(ret)
      } else {
        None
      }
    } else {
      None
    }
  }

  fn from_nat(n: u8) -> Expr {
    let mut ret = Expr::Var(0);
    for _ in 0..n {
      ret = Expr::App(box Expr::Var(1), box ret);
    }
    Expr::Lam(box Expr::Lam(box ret))
  }
}

#[test]
fn test_to_nat() {
  assert_eq!(ZERO.to_nat(), Some(0u8));
  assert_eq!(ONE.to_nat(), Some(1u8));
}

#[test]
fn test_beta() {
  let mut idid = Expr::App(box ID.clone(), box ID.clone());
  idid.beta();
  assert_eq!(idid, *ID);
}

const VAR_NUMERALS : [char; 11] = ['🄌', '➊', '➋', '➌', '➍', '➎', '➏', '➐', '➑', '➒', '➓'];

impl Display for Expr {
  fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if *ID == *self {
      write!(f, "[id]")
    } else {
      match self {
        Expr::Var(u) => {
          if *u < 11 {
            write!(f, "{}", VAR_NUMERALS[*u as usize])
          } else {
            write!(f, "[{}]", *u)
          }
        }
        Expr::Lam(e) => write!(f, "𝛌{}", e),
        Expr::App(l, r) => write!(f, "({} {})", l, r),
        Expr::Hole => write!(f, "▪")
      }
    }
  }
}

#[test]
fn test_formatting() {
  assert_eq!(ID.to_string(), "[id]");
  assert_eq!(
    Expr::App(box ID.clone(), box ID.clone()).to_string(),
    "([id] [id])"
  );
}
