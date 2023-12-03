pub use crate::arena::U;
use crate::arena::{Arena, Idx};

#[derive(Debug, Clone, Copy)]
pub struct Term(U, U);
#[derive(Debug, Default)]
pub enum TermRepr {
  #[default]
  Hole,
  Var(U),
  Lam(Idx<Term>),
  App(Idx<Term>, Idx<Term>),
}

impl From<TermRepr> for Term {
  fn from(value : TermRepr) -> Self {
    match value {
      TermRepr::Hole => Term(0, 0),
      TermRepr::Var(u) => Term(0, u + 1),
      TermRepr::Lam(e) => Term(e.raw, 0),
      TermRepr::App(l, r) => Term(l.raw, r.raw),
    }
  }
}

impl From<Term> for TermRepr {
  fn from(value : Term) -> Self {
    match value {
      Term(0, 0) => TermRepr::Hole,
      Term(0, u) => TermRepr::Var(u - 1),
      Term(l, 0) => TermRepr::Lam(l.into()),
      Term(l, r) => TermRepr::App(l.into(), r.into()),
    }
  }
}

impl Arena<Term> {
  pub fn duplicate(&mut self, at : Idx<Term>) -> Option<Idx<Term>> {
    match TermRepr::from(*self.get(at)) {
      TermRepr::Hole => {
        let new = self.alloc()?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::Hole.into());
        Some(new)
      }
      TermRepr::Var(u) => {
        let new = self.alloc()?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::Var(u).into());
        Some(new)
      }
      TermRepr::Lam(e) => {
        let new = self.alloc()?;
        let new_e = self.duplicate(e)?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::Lam(new_e).into());
        Some(new)
      }
      TermRepr::App(l, r) => {
        let new = self.alloc()?;
        let new_l = self.duplicate(l)?;
        let new_r = self.duplicate(r)?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::App(new_l, new_r).into());
        Some(new)
      }
    }
  }
  pub fn duplicate_from(&mut self, other : &Arena<Term>, at : Idx<Term>) -> Option<Idx<Term>> {
    match TermRepr::from(*other.get(at)) {
      TermRepr::Hole => {
        let new = self.alloc()?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::Hole.into());
        Some(new)
      }
      TermRepr::Var(u) => {
        let new = self.alloc()?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::Var(u).into());
        Some(new)
      }
      TermRepr::Lam(e) => {
        let new = self.alloc()?;
        let new_e = self.duplicate_from(other, e)?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::Lam(new_e).into());
        Some(new)
      }
      TermRepr::App(l, r) => {
        let new = self.alloc()?;
        let new_l = self.duplicate_from(other, l)?;
        let new_r = self.duplicate_from(other, r)?;
        let _ = core::mem::replace(self.get_mut(new), TermRepr::App(new_l, new_r).into());
        Some(new)
      }
    }
  }
  #[must_use]
  pub fn head(&self, at : Idx<Term>) -> Option<Idx<Term>> {
    match TermRepr::from(*self.get(at)) {
      TermRepr::Var(_) | TermRepr::Lam(_) | TermRepr::Hole => None,
      TermRepr::App(l, _) => match TermRepr::from(*self.get(l)) {
        TermRepr::Hole | TermRepr::Var(_) => None,
        TermRepr::Lam(_) => Some(at),
        TermRepr::App(..) => self.head(l),
      },
    }
  }

  pub fn shift(&mut self, at : Idx<Term>, level : U, amount : U) {
    match TermRepr::from(*self.get(at)) {
      TermRepr::Hole => {}
      TermRepr::Var(u) => {
        if u >= level {
          let _ = core::mem::replace(self.get_mut(at), TermRepr::Var(u + amount).into());
        }
      }
      TermRepr::Lam(e) => self.shift(e, level + 1, amount),
      TermRepr::App(l, r) => {
        self.shift(l, level, amount);
        self.shift(r, level, amount);
      }
    }
  }
}
