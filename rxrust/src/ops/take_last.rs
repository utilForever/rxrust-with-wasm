use crate::{impl_local_shared_both, prelude::*};
use std::collections::VecDeque;

#[derive(Clone)]
pub struct TakeLastOp<S> {
  pub(crate) source: S,
  pub(crate) count: usize,
}

impl<S: Observable> Observable for TakeLastOp<S> {
  type Item = S::Item;
  type Err = S::Err;
}

impl_local_shared_both! {
  impl<S> TakeLastOp<S>;
  type Unsub = S::Unsub;
  macro method($self: ident, $observer: ident, $ctx: ident) {
    $self.source.actual_subscribe(TakeLastObserver {
      observer: $observer,
      count: $self.count,
      queue: VecDeque::new(),
    })
  }
  where
    @ctx::shared_only(S::Item: Send + Sync + 'static ,)
    S: @ctx::Observable @ctx::local_only( + 'o)
}

pub struct TakeLastObserver<O, Item> {
  observer: O,
  count: usize,
  queue: VecDeque<Item>, // TODO: replace VecDeque with RingBuf
}

impl<Item, Err, O> Observer for TakeLastObserver<O, Item>
where
  O: Observer<Item = Item, Err = Err>,
{
  type Item = Item;
  type Err = Err;
  fn next(&mut self, value: Item) {
    self.queue.push_back(value);
    while self.queue.len() > self.count {
      self.queue.pop_front();
    }
  }

  fn error(&mut self, err: Self::Err) { self.observer.error(err) }

  fn complete(&mut self) {
    for value in self.queue.drain(..) {
      self.observer.next(value);
    }
    self.observer.complete();
  }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn base_function() {
    let mut completed = false;
    let mut ticks = vec![];

    observable::from_iter(0..100)
      .take_last(5)
      .subscribe_complete(|v| ticks.push(v), || completed = true);

    assert_eq!(ticks, vec![95, 96, 97, 98, 99]);
    assert!(completed);
  }

  #[test]
  fn take_last_support_fork() {
    let mut nc1 = 0;
    let mut nc2 = 0;
    {
      let take_last5 = observable::from_iter(0..100).take_last(5);
      let f1 = take_last5.clone();
      let f2 = take_last5;

      f1.take_last(5).subscribe(|_| nc1 += 1);
      f2.take_last(5).subscribe(|_| nc2 += 1);
    }
    assert_eq!(nc1, 5);
    assert_eq!(nc2, 5);
  }

  #[cfg(not(target_arch = "wasm32"))]
  #[test]
  fn ininto_shared() {
    observable::from_iter(0..100)
      .take_last(5)
      .take_last(5)
      .into_shared()
      .subscribe(|_| {});
  }

  #[test]
  fn bench() { do_bench(); }

  benchmark_group!(do_bench, bench_take_last);

  fn bench_take_last(b: &mut bencher::Bencher) { b.iter(base_function); }
}
