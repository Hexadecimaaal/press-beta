use core::marker::PhantomData;

pub type U = u16;

#[derive(Debug)]
pub struct Idx<T> {
  pub raw : U,
  _phantom : PhantomData<fn() -> T>,
}

impl<T> Copy for Idx<T> {}
impl<T> Clone for Idx<T> {
  fn clone(&self) -> Self { *self }
}

impl<T> From<U> for Idx<T> {
  #[must_use]
  fn from(value : U) -> Idx<T> {
    Idx {
      raw : value,
      _phantom : PhantomData,
    }
  }
}

pub struct Arena<T : 'static>(U, &'static mut [T]);

impl<T> Arena<T> {
  pub fn new(slice : &'static mut [T]) -> Arena<T> { Arena(1, slice) }
  #[must_use]
  pub fn get(&self, idx : Idx<T>) -> &T {
    assert!(idx.raw < self.0);
    &self.1[idx.raw as usize]
  }
  pub fn get_mut(&mut self, idx : Idx<T>) -> &mut T {
    assert!(idx.raw < self.0);
    &mut self.1[idx.raw as usize]
  }
  pub fn alloc(&mut self) -> Option<Idx<T>> {
    if self.0 as usize >= self.1.len() {
      None
    } else {
      let ret = Some(self.0.into());
      self.0 += 1;
      ret
    }
  }
}
