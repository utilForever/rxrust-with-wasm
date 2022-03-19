use crate::{impl_local_shared_both, prelude::*};

#[derive(Clone)]
pub struct MapOp<S, M> {
  pub(crate) source: S,
  pub(crate) func: M,
}

impl<Item, S, M> Observable for MapOp<S, M>
where
  S: Observable,
  M: FnMut(S::Item) -> Item,
{
  type Item = Item;
  type Err = S::Err;
}

impl_local_shared_both! {
  impl<Item, S, M>  MapOp<S, M>;
  type Unsub = S::Unsub;
  macro method($self: ident, $observer: ident, $ctx: ident) {
    let map = $self.func;
    $self.source.actual_subscribe(MapObserver {
      observer: $observer,
      map,
      _marker: TypeHint::new(),
    })
  }
  where
    S: @ctx::Observable,
    S::Item: @ctx::local_only('o) @ctx::shared_only('static),
    M: FnMut(S::Item) -> Item
      + @ctx::local_only('o) @ctx::shared_only( Send + Sync + 'static)
}

#[derive(Clone)]
pub struct MapObserver<O, M, Item> {
  observer: O,
  map: M,
  _marker: TypeHint<*const Item>,
}

impl<Item, Err, O, M, B> Observer for MapObserver<O, M, Item>
where
  O: Observer<Item = B, Err = Err>,
  M: FnMut(Item) -> B,
{
  type Item = Item;
  type Err = Err;
  fn next(&mut self, value: Item) { self.observer.next((self.map)(value)) }

  fn error(&mut self, err: Self::Err) { self.observer.error(err) }

  fn complete(&mut self) { self.observer.complete() }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn primitive_type() {
    let mut i = 0;
    observable::from_iter(100..101)
      .map(|v| v * 2)
      .subscribe(|v| i += v);
    assert_eq!(i, 200);
  }

  #[test]
  fn reference_lifetime_should_work() {
    let mut i = 0;

    observable::of(100).map(|v| v).subscribe(|v| i += v);
    assert_eq!(i, 100);
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn fork_and_shared() {
    // type to type can fork
    let m = observable::from_iter(0..100).map(|v| v);
    m.map(|v| v).into_shared().subscribe(|_| {});

    // type mapped to other type can fork
    let m = observable::from_iter(vec!['a', 'b', 'c']).map(|_v| 1);
    m.map(|v| v as f32).into_shared().subscribe(|_| {});

    // ref to ref can fork
    let m = observable::of(&1).map(|v| v);
    m.map(|v| v).into_shared().subscribe(|_| {});
  }

  #[test]
  fn map_types_mixed() {
    let mut i = 0;
    observable::from_iter(vec!['a', 'b', 'c'])
      .map(|_v| 1)
      .subscribe(|v| i += v);
    assert_eq!(i, 3);
  }

  #[test]
  fn benchmark() { do_bench(); }

  benchmark_group!(do_bench, bench);

  fn bench(b: &mut bencher::Bencher) { b.iter(primitive_type); }
}
