pub use crate::heap::U;
use crate::heap::{Heap, Idx, Object};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
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

impl Object for Term {
  fn get_refs(&self) -> Box<dyn Iterator<Item = Idx<Self>>> {
    match TermRepr::from(*self) {
      TermRepr::Hole | TermRepr::Var(_) => Box::new(core::iter::empty()),
      TermRepr::Lam(e) => Box::new(core::iter::once(e)),
      TermRepr::App(l, r) => Box::new(vec![l, r].into_iter()),
    }
  }
}

impl core::fmt::Debug for Term {
  fn fmt(&self, f : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{:?}", TermRepr::from(*self))
  }
}

impl<const PAGESIZE: usize, const G1PAGES: usize> Heap<Term, PAGESIZE, G1PAGES> {
  pub fn duplicate(&mut self, at : Idx<Term>) -> Option<Idx<Term>> {
    match TermRepr::from(*self.get(at)) {
      TermRepr::Hole => self.init(TermRepr::Hole.into()),
      TermRepr::Var(u) => self.init(TermRepr::Var(u).into()),
      TermRepr::Lam(e) => {
        let new_e = self.duplicate(e)?;
        self.init(TermRepr::Lam(new_e).into())
      }
      TermRepr::App(l, r) => {
        let new_l = self.duplicate(l)?;
        let new_r = self.duplicate(r)?;
        self.init(TermRepr::App(new_l, new_r).into())
      }
    }
  }
  pub fn duplicate_from<const OPS: usize, const OG1S: usize>(
    &mut self,
    other : &Heap<Term, OPS, OG1S>,
    at : Idx<Term>,
  ) -> Option<Idx<Term>> {
    match TermRepr::from(*other.get(at)) {
      TermRepr::Hole => self.init(TermRepr::Hole.into()),
      TermRepr::Var(u) => self.init(TermRepr::Var(u).into()),
      TermRepr::Lam(e) => {
        let new_e = self.duplicate_from(other, e)?;
        self.init(TermRepr::Lam(new_e).into())
      }
      TermRepr::App(l, r) => {
        let new_l = self.duplicate_from(other, l)?;
        let new_r = self.duplicate_from(other, r)?;
        self.init(TermRepr::App(new_l, new_r).into())
      }
    }
  }

  #[must_use]
  pub fn is_redux(&self, at : Idx<Term>) -> bool {
    if let TermRepr::App(l, _) = TermRepr::from(*self.get(at)) {
      if let TermRepr::Lam(_) = TermRepr::from(*self.get(l)) {
        return true;
      }
    }
    false
  }

  #[must_use]
  pub fn head(&self, at : Idx<Term>) -> Option<Idx<Term>> {
    if self.is_redux(at) {
      Some(at)
    } else {
      match TermRepr::from(*self.get(at)) {
        TermRepr::Var(_) | TermRepr::Hole => None,
        TermRepr::Lam(e) => self.head(e),
        TermRepr::App(l, _) => self.head(l),
      }
    }
  }

  #[must_use]
  pub fn redux(&self, at : Idx<Term>) -> Option<Idx<Term>> {
    if self.is_redux(at) {
      Some(at)
    } else {
      match TermRepr::from(*self.get(at)) {
        TermRepr::Var(_) | TermRepr::Hole => None,
        TermRepr::Lam(e) => self.redux(e),
        TermRepr::App(l, r) => self.redux(l).or_else(|| self.redux(r)),
      }
    }
  }

  pub fn shift(&mut self, at : Idx<Term>, level : U, amount : U) {
    match TermRepr::from(*self.get(at)) {
      TermRepr::Hole => {}
      TermRepr::Var(u) => {
        if u >= level {
          *self.get_mut(at) = TermRepr::Var(u + amount).into();
        }
      }
      TermRepr::Lam(e) => self.shift(e, level + 1, amount),
      TermRepr::App(l, r) => {
        self.shift(l, level, amount);
        self.shift(r, level, amount);
      }
    }
  }

  pub fn replace_closed(&mut self, at : Idx<Term>, var : U, with : Idx<Term>) -> Option<Idx<Term>> {
    match TermRepr::from(*self.get(at)) {
      TermRepr::Hole => Some(at),
      TermRepr::Var(v) => {
        if v == var {
          Some(with)
        } else {
          Some(at)
        }
      }
      TermRepr::Lam(e) => {
        let new_e = self.replace_closed(e, var + 1, with)?;
        if new_e == e {
          Some(at)
        } else {
          self.init(TermRepr::Lam(new_e).into())
        }
      }
      TermRepr::App(l, r) => {
        let new_l = self.replace_closed(l, var, with)?;
        let new_r = self.replace_closed(r, var, with)?;
        if new_l == l && new_r == r {
          Some(at)
        } else {
          self.init(TermRepr::App(new_l, new_r).into())
        }
      }
    }
  }

  pub fn replace(
    &mut self,
    at : Idx<Term>,
    var : U,
    with : Idx<Term>,
    level : U,
  ) -> Option<Idx<Term>> {
    todo!()
  }

  pub fn beta(&mut self, at : Idx<Term>) {}
}
