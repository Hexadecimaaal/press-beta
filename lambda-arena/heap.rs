use core::{marker::PhantomData, mem::MaybeUninit};

pub type U = u16;

pub struct Idx<T> {
  pub raw : U,
  _phantom : PhantomData<fn() -> T>,
}

impl<T> core::fmt::Debug for Idx<T> {
  fn fmt(&self, f : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_tuple("Idx").field(&self.raw).finish()
  }
}

impl<T> Copy for Idx<T> {}
impl<T> Clone for Idx<T> {
  fn clone(&self) -> Self { *self }
}

impl<T> PartialEq for Idx<T> {
  fn eq(&self, other : &Self) -> bool { self.raw == other.raw }
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

pub struct Heap<T, const PAGESIZE: usize, const G1PAGES: usize> {
  g2_ptr : usize,
  page_ptr : usize,
  g1_pages : [Option<Box<[T; PAGESIZE]>>; G1PAGES],
  g2_page : [T; PAGESIZE],
}

impl<T, const PAGESIZE: usize, const G1PAGES: usize> Heap<T, PAGESIZE, G1PAGES>
where
  T : Default,
{
  #[must_use]
  pub fn new() -> Heap<T, PAGESIZE, G1PAGES> {
    let mut g1_pages : [MaybeUninit<Option<Box<[T; PAGESIZE]>>>; G1PAGES] =
      MaybeUninit::uninit_array();
    for p in g1_pages.iter_mut() {
      p.write(None);
    }
    let g1_pages = unsafe { MaybeUninit::array_assume_init(g1_pages) };
    let mut g2_page : [MaybeUninit<T>; PAGESIZE] = MaybeUninit::uninit_array();
    for p in g2_page.iter_mut() {
      p.write(Default::default());
    }
    let g2_page = unsafe { MaybeUninit::array_assume_init(g2_page) };
    Heap {
      g2_ptr : 1,
      page_ptr : 0,
      g1_pages,
      g2_page,
    }
  }
  fn address(idx : Idx<T>) -> (usize, usize) {
    assert_ne!(idx, 0.into());
    (idx.raw as usize / PAGESIZE, idx.raw as usize % PAGESIZE)
  }
  fn unaddress_g1(page : usize, offset : usize) -> Idx<T> {
    assert_ne!(offset, 0);
    Idx::from((page * PAGESIZE + offset) as U)
  }
  fn unaddress_g2(offset : usize) -> Idx<T> {
    assert_ne!(offset, 0);
    Idx::from((G1PAGES * PAGESIZE + offset) as U)
  }
  #[must_use]
  pub fn get(&self, idx : Idx<T>) -> &T {
    let (page, offset) = Self::address(idx);
    if page == G1PAGES {
      &self.g2_page[offset]
    } else if page < G1PAGES {
      &self.g1_pages[page].as_ref().unwrap()[offset]
    } else {
      panic!("segmentation fault lol")
    }
  }
  pub fn get_mut(&mut self, idx : Idx<T>) -> &mut T {
    let (page, offset) = Self::address(idx);
    if page == G1PAGES {
      &mut self.g2_page[offset]
    } else if page < G1PAGES {
      &mut self.g1_pages[page].as_mut().unwrap()[offset]
    } else {
      panic!("segmentation fault lol")
    }
  }
  /// allocate in g2 unconditionally. only call this after checking for
  /// high water mark and collect g2 if needed.
  pub fn alloc_g2(&mut self) -> Idx<T> {
    assert!(self.g2_ptr < PAGESIZE);
    let newaddr = self.g2_ptr;
    self.g2_ptr += 1;
    Self::unaddress_g2(newaddr)
  }

  pub fn init(&mut self, init : T) -> Option<Idx<T>> {
    let new = self.alloc_g2();
    *self.get_mut(new) = init;
    Some(new)
  }

  pub fn init_with<F>(&mut self, init : F) -> Option<Idx<T>>
  where
    F : FnOnce() -> T,
  {
    let new = self.alloc_g2();
    *self.get_mut(new) = init();
    Some(new)
  }
}

impl<T : Default, const PAGESIZE: usize, const G1PAGES: usize> Default
  for Heap<T, PAGESIZE, G1PAGES>
{
  fn default() -> Self { Self::new() }
}

pub trait Object
where
  Self : Sized,
{
  fn get_refs(&self) -> Box<dyn Iterator<Item = Idx<Self>>>;
}
