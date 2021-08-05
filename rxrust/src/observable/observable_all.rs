use crate::prelude::*;

#[derive(Clone)]
pub struct ObserverAll<N, E, C, Item, Err> {
  next: N,
  error: E,
  complete: C,
  is_stopped: bool,
  marker: TypeHint<(*const Item, *const Err)>,
}

impl<Item, Err, N, E, C> ObserverAll<N, E, C, Item, Err> {
  #[inline(always)]
  pub fn new(next: N, error: E, complete: C) -> Self {
    ObserverAll {
      next,
      error,
      complete,
      is_stopped: false,
      marker: TypeHint::new(),
    }
  }
}

impl<Item, Err, N, E, C> Observer for ObserverAll<N, E, C, Item, Err>
where
  C: FnMut(),
  N: FnMut(Item),
  E: FnMut(Err),
{
  type Item = Item;
  type Err = Err;
  #[inline(always)]
  fn next(&mut self, value: Self::Item) { (self.next)(value); }

  fn error(&mut self, err: Self::Err) {
    (self.error)(err);
    self.is_stopped = true;
  }

  fn complete(&mut self) {
    (self.complete)();
    self.is_stopped = true;
  }

  #[inline]
  fn is_stopped(&self) -> bool { self.is_stopped }
}

pub trait SubscribeAll<'a, N, E, C> {
  /// A type implementing [`SubscriptionLike`]
  type Unsub: SubscriptionLike;

  /// Invokes an execution of an Observable and registers Observer handlers for
  /// notifications it will emit.
  ///
  /// * `error`: A handler for a terminal event resulting from an error.
  /// * `complete`: A handler for a terminal event resulting from successful
  /// completion.
  fn subscribe_all(
    self,
    next: N,
    error: E,
    complete: C,
  ) -> SubscriptionWrapper<Self::Unsub>;
}

impl<'a, S, N, E, C> SubscribeAll<'a, N, E, C> for S
where
  S: LocalObservable<'a>,
  N: FnMut(S::Item) + 'a,
  E: FnMut(S::Err) + 'a,
  S::Err: 'a,
  S::Item: 'a,
  C: FnMut() + 'a,
{
  type Unsub = S::Unsub;
  fn subscribe_all(
    self,
    next: N,
    error: E,
    complete: C,
  ) -> SubscriptionWrapper<Self::Unsub>
  where
    Self: Sized,
  {
    let subscriber = Subscriber::local(ObserverAll {
      next,
      error,
      complete,
      is_stopped: false,
      marker: TypeHint::new(),
    });
    SubscriptionWrapper(self.actual_subscribe(subscriber))
  }
}

impl<'a, S, N, E, C> SubscribeAll<'a, N, E, C> for Shared<S>
where
  S: SharedObservable,
  N: FnMut(S::Item) + Send + Sync + 'static,
  E: FnMut(S::Err) + Send + Sync + 'static,
  C: FnMut() + Send + Sync + 'static,
  S::Err: 'static,
  S::Item: 'static,
{
  type Unsub = S::Unsub;
  fn subscribe_all(
    self,
    next: N,
    error: E,
    complete: C,
  ) -> SubscriptionWrapper<Self::Unsub>
  where
    Self: Sized,
  {
    let subscriber = Subscriber::shared(ObserverAll {
      next,
      error,
      complete,
      is_stopped: false,
      marker: TypeHint::new(),
    });
    SubscriptionWrapper(self.0.actual_subscribe(subscriber))
  }
}

#[test]
fn raii() {
  let mut times = 0;
  {
    let mut subject = LocalSubject::new();
    {
      let _ = subject
        .clone()
        .subscribe_all(|_| times += 1, |_| {}, || {})
        .unsubscribe_when_dropped();
    } // <-- guard is dropped here!
    subject.next(());
    subject.error(());
  }
  assert_eq!(times, 0);
}
