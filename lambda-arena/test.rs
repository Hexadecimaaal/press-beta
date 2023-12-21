// pub static mut TEST_SLICE_1 : [(u16, u16); 2000] = [(0, 0); 2000];
// const PAGESIZE : usize = 32 * 1024 / (2 * core::mem::size_of::<U>());
// const TOTAL_PAGES : usize = 7;
// const G1PAGES : usize = TOTAL_PAGES - 1;

#[test]
fn test_heap() {
  use crate::heap::*;
  let mut heap : Heap<(u16, u16), 10, 0> = Heap::new();
  assert_eq!(*heap.get(1.into()), (0u16, 0u16));
  let new = heap.alloc_g2();
  assert_eq!(new.raw, 1);
  *heap.get_mut(new) = (1u16, 1u16);
  assert_eq!(*heap.get(new), (1u16, 1u16));
}

#[test]
fn test_lambda_arena_dup_from() {
  use crate::heap::*;
  use crate::lambda::*;
  let mut heap: Heap<Term, 20, 0> = Heap::new();
  let dzero = heap.init_with(|| TermRepr::Var(0).into()).unwrap();
  let id = heap.init_with(|| TermRepr::Lam(dzero).into()).unwrap();
  let id2 = heap.duplicate(id).unwrap();
  let idid = heap.init_with(|| TermRepr::App(id, id2).into()).unwrap();
}

// #[test]
// fn test_lambda_arena_replace_closed() {
//   use crate::arena::*;
//   use crate::lambda::*;
//   let mut a = [Term::default(); 20];
//   let mut arena = Heap::new(&mut a);
//   let dzero = arena.init_with(|| TermRepr::Var(0).into()).unwrap();
//   let id = arena.init_with(|| TermRepr::Lam(dzero).into()).unwrap();
//   let done = arena.init_with(|| TermRepr::Var(1).into()).unwrap();
//   let free_occur = arena.init_with(|| TermRepr::Lam(done).into()).unwrap();
//   let zero = arena.replace_closed(free_occur, 0, id).unwrap();
//   dbg!(a);
// }
