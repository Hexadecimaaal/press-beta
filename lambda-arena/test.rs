pub static mut TEST_SLICE_1 : [(u16, u16); 2000] = [(0, 0); 2000];

#[test]
fn test_arena() {
  use crate::arena::*;
  let mut arena;
  unsafe {
    arena = Arena::new(&mut TEST_SLICE_1);
  }
  assert_eq!(*arena.get(0.into()), (0u16, 0u16));
  let new = arena.alloc().unwrap();
  assert_eq!(new.raw, 1);
  let _ = core::mem::replace(arena.get_mut(new), (1u16, 1u16));
  assert_eq!(*arena.get(new), (1u16, 1u16));
}

#[test]
fn test_lambda_arena() {}
