use crate::next_proxy_impl;
use crate::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MergeOp<S1, S2> {
  pub(crate) source1: S1,
  pub(crate) source2: S2,
}

pub struct SharedMergeOp<S1, S2>(MergeOp<S1, S2>);

impl<S1, S2> Observable for MergeOp<S1, S2>
where
  S1: Observable,
  S2: Observable<Item = S1::Item, Err = S1::Err>,
{
  type Item = S1::Item;
  type Err = S1::Err;
}

impl<'a, S1, S2> LocalObservable<'a> for MergeOp<S1, S2>
where
  S1: LocalObservable<'a>,
  S2: LocalObservable<'a, Item = S1::Item, Err = S1::Err>,
{
  type Unsub = LocalSubscription;

  fn actual_subscribe<O: Observer<Item = Self::Item, Err = Self::Err> + 'a>(
    self,
    subscriber: Subscriber<O, LocalSubscription>,
  ) -> Self::Unsub {
    let subscription = subscriber.subscription;
    let merge_observer = Rc::new(RefCell::new(MergeObserver {
      observer: subscriber.observer,
      subscription: subscription.clone(),
      completed_one: false,
    }));
    subscription.add(self.source1.actual_subscribe(Subscriber {
      observer: merge_observer.clone(),
      subscription: LocalSubscription::default(),
    }));
    subscription.add(self.source2.actual_subscribe(Subscriber {
      observer: merge_observer,
      subscription: LocalSubscription::default(),
    }));
    subscription
  }
}

impl<S1, S2> SharedObservable for MergeOp<S1, S2>
where
  S1: SharedObservable,
  S2: SharedObservable<Item = S1::Item, Err = S1::Err, Unsub = S1::Unsub>,
  S1::Unsub: Send + Sync,
{
  type Unsub = SharedSubscription;
  fn actual_subscribe<
    O: Observer<Item = Self::Item, Err = Self::Err> + Sync + Send + 'static,
  >(
    self,
    subscriber: Subscriber<O, SharedSubscription>,
  ) -> Self::Unsub {
    let subscription = subscriber.subscription;
    let merge_observer = Arc::new(Mutex::new(MergeObserver {
      observer: subscriber.observer,
      subscription: subscription.clone(),
      completed_one: false,
    }));
    subscription.add(self.source1.actual_subscribe(Subscriber {
      observer: merge_observer.clone(),
      subscription: SharedSubscription::default(),
    }));
    subscription.add(self.source2.actual_subscribe(Subscriber {
      observer: merge_observer,
      subscription: SharedSubscription::default(),
    }));
    subscription
  }
}

#[derive(Clone)]
pub struct MergeObserver<O, Unsub> {
  observer: O,
  subscription: Unsub,
  completed_one: bool,
}

impl<Item, Err, O, Unsub> Observer for MergeObserver<O, Unsub>
where
  O: Observer<Item = Item, Err = Err>,
  Unsub: SubscriptionLike,
{
  type Item = Item;
  type Err = Err;
  next_proxy_impl!(Item, observer);
  fn error(&mut self, err: Err) {
    self.observer.error(err);
    self.subscription.unsubscribe();
  }

  fn complete(&mut self) {
    if self.completed_one {
      self.observer.complete();
    } else {
      self.completed_one = true;
    }
  }

  #[inline]
  fn is_stopped(&self) -> bool {
    self.observer.is_stopped()
  }
}

#[cfg(test)]
mod test {
  use crate::prelude::*;
  use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  };

  #[test]
  fn odd_even_merge() {
    // three collection to store streams emissions
    let mut odd_store = vec![];
    let mut even_store = vec![];
    let mut numbers_store = vec![];

    {
      let mut numbers = LocalSubject::new();
      // enabling multiple observers for even stream;
      let even = numbers.clone().filter(|v| *v % 2 == 0);
      // enabling multiple observers for odd stream;
      let odd = numbers.clone().filter(|v| *v % 2 != 0);

      // merge odd and even stream again
      let merged = even.clone().merge(odd.clone());

      //  attach observers
      merged.subscribe(|v| numbers_store.push(v));
      odd.subscribe(|v| odd_store.push(v));
      even.subscribe(|v| even_store.push(v));

      (0..10).for_each(|v| {
        numbers.next(v);
      });
    }
    assert_eq!(even_store, vec![0, 2, 4, 6, 8]);
    assert_eq!(odd_store, vec![1, 3, 5, 7, 9]);
    assert_eq!(numbers_store, (0..10).collect::<Vec<_>>());
  }

  #[test]
  fn merge_unsubscribe_work() {
    let mut numbers = LocalSubject::new();
    // enabling multiple observers for even stream;
    let even = numbers.clone().filter(|v| *v % 2 == 0);
    // enabling multiple observers for odd stream;
    let odd = numbers.clone().filter(|v| *v % 2 != 0);

    even
      .merge(odd)
      .subscribe(|_| unreachable!("oh, unsubscribe not work."))
      .unsubscribe();

    numbers.next(&1);
  }

  #[test]
  fn completed_test() {
    let completed = Arc::new(AtomicBool::new(false));
    let c_clone = completed.clone();
    let mut even = LocalSubject::new();
    let mut odd = LocalSubject::new();

    even.clone().merge(odd.clone()).subscribe_complete(
      |_: &()| {},
      move || completed.store(true, Ordering::Relaxed),
    );

    even.complete();
    assert!(!c_clone.load(Ordering::Relaxed));
    odd.complete();
    assert!(c_clone.load(Ordering::Relaxed));
    c_clone.store(false, Ordering::Relaxed);
    even.complete();
    assert!(!c_clone.load(Ordering::Relaxed));
  }

  #[test]
  fn error_test() {
    let completed = Arc::new(Mutex::new(0));
    let cc = completed.clone();
    let error = Arc::new(Mutex::new(0));
    let ec = error.clone();
    let mut even = LocalSubject::new();
    let mut odd = LocalSubject::new();

    even.clone().merge(odd.clone()).subscribe_all(
      |_: ()| {},
      move |_| *error.lock().unwrap() += 1,
      move || *completed.lock().unwrap() += 1,
    );

    odd.error("");
    even.clone().error("");
    even.complete();

    // if error occur,  stream terminated.
    assert_eq!(*cc.lock().unwrap(), 0);
    // error should be hit just once
    assert_eq!(*ec.lock().unwrap(), 1);
  }

  #[test]
  fn merge_fork() {
    let o = observable::create(|mut s| {
      s.next(1);
      s.next(2);
      s.error(());
    });

    let m = o.clone().merge(o.clone());
    m.clone().merge(m.clone()).subscribe(|_| {});
  }

  #[test]
  fn merge_local_and_shared() {
    let mut res = vec![];
    let shared = observable::of(1);
    let local = observable::of(2);

    shared.merge(local).into_shared().subscribe(move |v| {
      res.push(v);
    });
  }

  #[test]
  fn bench() {
    do_bench();
  }

  benchmark_group!(do_bench, bench_merge);

  fn bench_merge(b: &mut bencher::Bencher) {
    b.iter(odd_even_merge);
  }
}
