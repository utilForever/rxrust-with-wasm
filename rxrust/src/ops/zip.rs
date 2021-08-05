use crate::prelude::*;
use crate::{complete_proxy_impl, error_proxy_impl, is_stopped_proxy_impl};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// An Observable that combines from two other two Observables.
///
/// This struct is created by the zip method on [Observable](Observable::zip).
/// See its documentation for more.
#[derive(Clone)]
pub struct ZipOp<A, B> {
  pub(crate) a: A,
  pub(crate) b: B,
}

impl<A, B> Observable for ZipOp<A, B>
where
  A: Observable,
  B: Observable<Err = A::Err>,
{
  type Item = (A::Item, B::Item);
  type Err = A::Err;
}

impl<'a, A, B> LocalObservable<'a> for ZipOp<A, B>
where
  A: LocalObservable<'a>,
  B: LocalObservable<'a, Err = A::Err>,
  A::Item: 'a,
  B::Item: 'a,
{
  type Unsub = LocalSubscription;
  fn actual_subscribe<O: Observer<Item = Self::Item, Err = Self::Err> + 'a>(
    self,
    subscriber: Subscriber<O, LocalSubscription>,
  ) -> Self::Unsub {
    let sub = subscriber.subscription;
    let o_zip = ZipObserver::new(subscriber.observer, sub.clone());
    let o_zip = Rc::new(RefCell::new(o_zip));
    sub.add(self.a.actual_subscribe(Subscriber {
      observer: AObserver(o_zip.clone(), TypeHint::new()),
      subscription: LocalSubscription::default(),
    }));

    sub.add(self.b.actual_subscribe(Subscriber {
      observer: BObserver(o_zip, TypeHint::new()),
      subscription: LocalSubscription::default(),
    }));
    sub
  }
}

impl<A, B> SharedObservable for ZipOp<A, B>
where
  A: SharedObservable,
  B: SharedObservable<Err = A::Err>,
  A::Item: Send + Sync + 'static,
  B::Item: Send + Sync + 'static,
  A::Unsub: Send + Sync,
  B::Unsub: Send + Sync,
{
  type Unsub = SharedSubscription;
  fn actual_subscribe<
    O: Observer<Item = Self::Item, Err = Self::Err> + Sync + Send + 'static,
  >(
    self,
    subscriber: Subscriber<O, SharedSubscription>,
  ) -> Self::Unsub {
    let sub = subscriber.subscription;
    let o_zip = ZipObserver::new(subscriber.observer, sub.clone());
    let o_zip = Arc::new(Mutex::new(o_zip));
    sub.add(self.a.actual_subscribe(Subscriber {
      observer: AObserver(o_zip.clone(), TypeHint::new()),
      subscription: SharedSubscription::default(),
    }));

    sub.add(self.b.actual_subscribe(Subscriber {
      observer: BObserver(o_zip, TypeHint::new()),
      subscription: SharedSubscription::default(),
    }));
    sub
  }
}

enum ZipItem<A, B> {
  ItemA(A),
  ItemB(B),
}

struct ZipObserver<O, U, A, B> {
  observer: O,
  subscription: U,
  a: VecDeque<A>,
  b: VecDeque<B>,
  completed_one: bool,
}

impl<O, U, A, B> ZipObserver<O, U, A, B> {
  fn new(o: O, u: U) -> Self {
    ZipObserver {
      observer: o,
      subscription: u,
      a: VecDeque::default(),
      b: VecDeque::default(),
      completed_one: false,
    }
  }
}

impl<O, U, A, B, Err> Observer for ZipObserver<O, U, A, B>
where
  O: Observer<Item = (A, B), Err = Err>,
  U: SubscriptionLike,
{
  type Item = ZipItem<A, B>;
  type Err = Err;
  fn next(&mut self, value: ZipItem<A, B>) {
    match value {
      ZipItem::ItemA(v) => {
        if !self.b.is_empty() {
          self.observer.next((v, self.b.pop_front().unwrap()))
        } else {
          self.a.push_back(v);
        }
      }
      ZipItem::ItemB(v) => {
        if !self.a.is_empty() {
          self.observer.next((self.a.pop_front().unwrap(), v))
        } else {
          self.b.push_back(v)
        }
      }
    }
  }

  fn error(&mut self, err: Err) {
    self.observer.error(err);
    self.subscription.unsubscribe();
  }

  fn complete(&mut self) {
    if self.completed_one {
      self.subscription.unsubscribe();
      self.observer.complete();
    } else {
      self.completed_one = true;
    }
  }

  is_stopped_proxy_impl!(observer);
}

struct AObserver<O, B>(O, TypeHint<B>);

impl<O, A, B, Err> Observer for AObserver<O, B>
where
  O: Observer<Item = ZipItem<A, B>, Err = Err>,
{
  type Item = A;
  type Err = Err;
  fn next(&mut self, value: A) { self.0.next(ZipItem::ItemA(value)); }

  error_proxy_impl!(Err, 0);
  complete_proxy_impl!(0);
  is_stopped_proxy_impl!(0);
}

struct BObserver<O, A>(O, TypeHint<A>);

impl<O, A, B, Err> Observer for BObserver<O, A>
where
  O: Observer<Item = ZipItem<A, B>, Err = Err>,
{
  type Item = B;
  type Err = Err;
  fn next(&mut self, value: B) { self.0.next(ZipItem::ItemB(value)); }

  error_proxy_impl!(Err, 0);
  complete_proxy_impl!(0);
  is_stopped_proxy_impl!(0);
}

#[cfg(test)]
mod test {
  use crate::prelude::*;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Arc;

  #[test]
  fn smoke() {
    let zip = observable::from_iter(0..10).zip(observable::from_iter(0..10));
    let zipped_count = Arc::new(AtomicUsize::new(0));
    let zcc = zipped_count.clone();
    zip
      .clone()
      .count()
      .subscribe(|v| zipped_count.store(v, Ordering::Relaxed));
    let mut zipped_sum = 0;
    assert_eq!(zcc.load(Ordering::Relaxed), 10);
    zip.map(|(a, b)| a + b).sum().subscribe(|v| zipped_sum = v);
    assert_eq!(zipped_sum, 90);
  }

  #[test]
  fn complete() {
    let mut complete = false;
    {
      let mut s1 = LocalSubject::new();
      s1.clone()
        .zip(LocalSubject::new())
        .subscribe_complete(|((), ())| {}, || complete = true);

      s1.complete();
    }
    assert!(!complete);

    {
      let mut s1 = LocalSubject::new();
      let mut s2 = LocalSubject::new();
      s1.clone()
        .zip(s2.clone())
        .subscribe_complete(|((), ())| {}, || complete = true);

      s1.complete();
      s2.complete();
    }
    assert!(complete);
  }

  #[test]
  fn bench() { do_bench(); }

  benchmark_group!(do_bench, bench_zip);

  fn bench_zip(b: &mut bencher::Bencher) { b.iter(smoke); }
}
