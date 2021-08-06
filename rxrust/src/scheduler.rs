use crate::prelude::*;
use async_std::prelude::FutureExt as AsyncFutureExt;
use futures::future::{lazy, AbortHandle, FutureExt};
use std::future::Future;

use futures::StreamExt;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub fn task_future<T>(
    task: impl FnOnce(T) + 'static,
    state: T,
    delay: Option<Duration>,
) -> (impl Future<Output = ()>, SpawnHandle) {
    let fut = lazy(|_| task(state)).delay(delay.unwrap_or_default());
    let (fut, handle) = futures::future::abortable(fut);
    (fut.map(|_| ()), SpawnHandle::new(handle))
}

pub trait LocalScheduler {
    fn spawn<Fut>(&self, future: Fut)
    where
        Fut: Future<Output = ()> + 'static;

    fn schedule<T: 'static>(
        &self,
        task: impl FnOnce(T) + 'static,
        delay: Option<Duration>,
        state: T,
    ) -> SpawnHandle {
        let (f, handle) = task_future(task, state, delay);
        self.spawn(f);
        handle
    }

    fn schedule_repeating(
        &self,
        task: impl FnMut(usize) + 'static,
        time_between: Duration,
        at: Option<Instant>,
    ) -> SpawnHandle {
        let (f, handle) = repeating_future(task, time_between, at);
        self.spawn(f.map(|_| ()));
        handle
    }
}

#[derive(Clone)]
pub struct SpawnHandle {
    pub handle: AbortHandle,
    is_closed: Arc<RwLock<bool>>,
}

impl SpawnHandle {
    #[inline]
    pub fn new(handle: AbortHandle) -> Self {
        SpawnHandle {
            handle,
            is_closed: Arc::new(RwLock::new(false)),
        }
    }
}

impl SubscriptionLike for SpawnHandle {
    fn unsubscribe(&mut self) {
        let was_closed = *self.is_closed.read().unwrap();
        if !was_closed {
            *self.is_closed.write().unwrap() = true;
            self.handle.abort();
        }
    }

    #[inline]
    fn is_closed(&self) -> bool {
        *self.is_closed.read().unwrap()
    }
}

#[cfg(feature = "futures-scheduler")]
mod futures_scheduler {
    use crate::scheduler::LocalScheduler;
    use futures::{
        executor::{LocalSpawner, ThreadPool},
        task::{LocalSpawnExt, SpawnExt},
        Future, FutureExt,
    };

    impl LocalScheduler for LocalSpawner {
        fn spawn<Fut>(&self, future: Fut)
        where
            Fut: Future<Output = ()> + 'static,
        {
            self.spawn_local(future.map(|_| ())).unwrap();
        }
    }
}

fn repeating_future(
    task: impl FnMut(usize) + 'static,
    time_between: Duration,
    at: Option<Instant>,
) -> (impl Future<Output = ()>, SpawnHandle) {
    let now = Instant::now();
    let delay = at.map(|inst| {
        if inst > now {
            inst - now
        } else {
            Duration::from_micros(0)
        }
    });
    let future = to_interval(task, time_between, delay.unwrap_or(time_between));
    let (fut, handle) = futures::future::abortable(future);
    (fut.map(|_| ()), SpawnHandle::new(handle))
}

fn to_interval(
    mut task: impl FnMut(usize) + 'static,
    interval_duration: Duration,
    delay: Duration,
) -> impl Future<Output = ()> {
    let mut number = 0;

    futures::future::ready(())
        .then(move |_| {
            task(number);
            async_std::stream::interval(interval_duration).for_each(move |_| {
                number += 1;
                task(number);
                futures::future::ready(())
            })
        })
        .delay(delay)
}

#[cfg(feature = "wasm-scheduler")]
mod wasm_scheduler {
    use crate::scheduler::LocalScheduler;
    use futures::{
        executor::LocalSpawner,
        Future, FutureExt,
    };

    impl LocalScheduler for LocalSpawner {
        fn spawn<Fut>(&self, future: Fut)
        where
            Fut: Future<Output = ()> + 'static,
        {
            wasm_bindgen_futures::spawn_local(future.map(|_| ()));
        }
    }
}
