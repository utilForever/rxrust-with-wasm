use crate::prelude::*;
use crate::{complete_proxy_impl, error_proxy_impl, is_stopped_proxy_impl};

#[derive(Clone)]
pub struct MapToOp<S, B> {
  pub(crate) source: S,
  pub(crate) value: B,
}

#[doc(hidden)]
macro_rules! observable_impl {
    ($subscription:ty, $($marker:ident +)* $lf: lifetime) => {
    fn actual_subscribe<O > (
      self,
      subscriber: Subscriber<O, $subscription>,
    ) -> Self::Unsub
    where O: Observer<Item=Self::Item,Err= Self::Err> + $($marker +)* $lf {
      let value = self.value;
      self.source.actual_subscribe(Subscriber {
        observer: MapToObserver {
          observer: subscriber.observer,
          value,
          marker: TypeHint::new(),
        },
        subscription: subscriber.subscription,
      })
    }
  }
}

impl<S, B> Observable for MapToOp<S, B>
where
  S: Observable,
{
  type Item = B;
  type Err = S::Err;
}

impl<'a, B, S> LocalObservable<'a> for MapToOp<S, B>
where
  S: LocalObservable<'a>,
  B: Clone + 'a,
  S::Item: 'a,
{
  type Unsub = S::Unsub;
  observable_impl!(LocalSubscription,'a);
}

impl<B, S> SharedObservable for MapToOp<S, B>
where
  S: SharedObservable,
  B: Clone + Send + Sync + 'static,
  S::Item: 'static,
{
  type Unsub = S::Unsub;
  observable_impl!(SharedSubscription, Send + Sync + 'static);
}

#[derive(Clone)]
pub struct MapToObserver<O, B, Item> {
  observer: O,
  value: B,
  marker: TypeHint<*const Item>,
}

impl<Item, Err, O, B> Observer for MapToObserver<O, B, Item>
where
  O: Observer<Item = B, Err = Err>,
  B: Clone,
{
  type Item = Item;
  type Err = Err;
  fn next(&mut self, _: Item) {
    self.observer.next(self.value.clone())
  }
  error_proxy_impl!(Err, observer);
  complete_proxy_impl!(observer);
  is_stopped_proxy_impl!(observer);
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn primitive_type() {
    let mut i = 0;
    observable::from_iter(100..101)
      .map_to(5)
      .subscribe(|v| i += v);
    assert_eq!(i, 5);
  }

  #[test]
  fn reference_lifetime_should_work() {
    let mut i = 0;

    observable::of(100).map_to(5).subscribe(|v| i += v);
    assert_eq!(i, 5);
  }

  #[test]
  fn fork_and_shared() {
    // type to type can fork
    let m = observable::from_iter(0..100).map_to(5);
    m.map_to(6).into_shared().subscribe(|_| {});

    // type mapped to other type can fork
    let m = observable::from_iter(vec!['a', 'b', 'c']).map_to(1);
    m.map_to(2.0).into_shared().subscribe(|_| {});

    // ref to ref can fork
    let m = observable::of(&1).map_to(3);
    m.map_to(4).into_shared().subscribe(|_| {});
  }

  #[test]
  fn map_types_mixed() {
    let mut i = 0;
    observable::from_iter(vec!['a', 'b', 'c'])
      .map_to(1)
      .subscribe(|v| i += v);
    assert_eq!(i, 3);
  }

  #[test]
  fn bench() {
    do_bench();
  }

  benchmark_group!(do_bench, bench_map_to);

  fn bench_map_to(b: &mut bencher::Bencher) {
    b.iter(primitive_type);
  }
}
