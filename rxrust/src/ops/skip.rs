use crate::prelude::*;
use crate::{complete_proxy_impl, error_proxy_impl};

#[derive(Clone)]
pub struct SkipOp<S> {
  pub(crate) source: S,
  pub(crate) count: u32,
}

#[doc(hidden)]
macro_rules! observable_impl {
    ($subscription:ty, $($marker:ident +)* $lf: lifetime) => {
  fn actual_subscribe<O>(
    self,
    subscriber: Subscriber<O, $subscription>,
  ) -> Self::Unsub
  where O: Observer<Item=Self::Item,Err= Self::Err> + $($marker +)* $lf {
    let subscriber = Subscriber {
      observer: SkipObserver {
        observer: subscriber.observer,
        subscription: subscriber.subscription.clone(),
        count: self.count,
        hits: 0,
      },
      subscription: subscriber.subscription,
    };
    self.source.actual_subscribe(subscriber)
  }
}
}

observable_proxy_impl!(SkipOp, S);

impl<'a, S> LocalObservable<'a> for SkipOp<S>
where
  S: LocalObservable<'a>,
{
  type Unsub = S::Unsub;
  observable_impl!(LocalSubscription, 'a);
}

impl<S> SharedObservable for SkipOp<S>
where
  S: SharedObservable,
{
  type Unsub = S::Unsub;
  observable_impl!(SharedSubscription, Send + Sync + 'static);
}

pub struct SkipObserver<O, S> {
  observer: O,
  subscription: S,
  count: u32,
  hits: u32,
}

impl<Item, Err, O, U> Observer for SkipObserver<O, U>
where
  O: Observer<Item = Item, Err = Err>,
  U: SubscriptionLike,
{
  type Item = Item;
  type Err = Err;
  fn next(&mut self, value: Item) {
    self.hits += 1;
    if self.hits > self.count {
      self.observer.next(value);
      if self.hits == self.count {
        self.complete();
        self.subscription.unsubscribe();
      }
    }
  }

  error_proxy_impl!(Err, observer);
  complete_proxy_impl!(observer);

  #[inline]
  fn is_stopped(&self) -> bool {
    self.observer.is_stopped()
  }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;

  #[test]
  fn base_function() {
    let mut completed = false;
    let mut next_count = 0;

    observable::from_iter(0..100)
      .skip(5)
      .subscribe_complete(|_| next_count += 1, || completed = true);

    assert_eq!(next_count, 95);
    assert!(completed);
  }

  #[test]
  fn base_empty_function() {
    let mut completed = false;
    let mut next_count = 0;

    observable::from_iter(0..100)
      .skip(101)
      .subscribe_complete(|_| next_count += 1, || completed = true);

    assert_eq!(next_count, 0);
    assert!(completed);
  }

  #[test]
  fn skip_support_fork() {
    let mut nc1 = 0;
    let mut nc2 = 0;
    {
      let skip5 = observable::from_iter(0..100).skip(5);
      let f1 = skip5.clone();
      let f2 = skip5;

      f1.skip(5).subscribe(|_| nc1 += 1);
      f2.skip(5).subscribe(|_| nc2 += 1);
    }
    assert_eq!(nc1, 90);
    assert_eq!(nc2, 90);
  }

  #[test]
  fn ininto_shared() {
    observable::from_iter(0..100)
      .skip(5)
      .skip(5)
      .into_shared()
      .subscribe(|_| {});
  }

  #[test]
  fn benchmark() {
    do_bench();
  }

  benchmark_group!(do_bench, bench);

  fn bench(b: &mut bencher::Bencher) {
    b.iter(base_function);
  }
}
