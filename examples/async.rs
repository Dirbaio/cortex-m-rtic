//! examples/spawn2.rs

#![no_main]
#![no_std]
#![feature(const_fn)]
#![feature(type_alias_impl_trait)]

use core::cell::Cell;
use core::cell::UnsafeCell;
use core::future::Future;
use core::mem;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use cortex_m_semihosting::{debug, hprintln};
use panic_semihosting as _;

#[rtic::app(device = lm3s6965)]
mod app {
    #[init]
    fn init(_c: init::Context) -> init::LateResources {
        foo::spawn(true).unwrap();
        bar::spawn(true).unwrap();

        init::LateResources {}
    }

    #[task]
    fn foo(_c: foo::Context, spawn: bool) {
        // BEGIN BOILERPLATE
        type F = impl Future + 'static;
        fn create() -> F {
            task()
        }

        static mut TASK: Task<F> = Task::new();

        unsafe {
            if spawn {
                TASK.spawn(create);
            }
            TASK.poll(|| {
                // it's OK if the task fails to spawn because it's already queued.
                // if it's awakened 2 times we just need to run it once.
                let _ = foo::spawn(false);
            });
        }
        // END BOILERPLATE

        async fn task() {
            loop {
                hprintln!("foo");
                please_yield().await;
            }
        }
    }

    #[task]
    fn bar(_c: bar::Context, spawn: bool) {
        // BEGIN BOILERPLATE
        type F = impl Future + 'static;
        fn create() -> F {
            task()
        }

        static mut TASK: Task<F> = Task::new();

        unsafe {
            if spawn {
                TASK.spawn(create);
            }
            TASK.poll(|| {
                // it's OK if the task fails to spawn because it's already queued.
                // if it's awakened 2 times we just need to run it once.
                let _ = bar::spawn(false);
            });
        }
        // END BOILERPLATE

        async fn task() {
            loop {
                hprintln!("bar");
                please_yield().await;
            }
        }
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn SSI0();
    }
}

//=============
// Waker

static WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake, waker_drop);

unsafe fn waker_clone(p: *const ()) -> RawWaker {
    RawWaker::new(p, &WAKER_VTABLE)
}

unsafe fn waker_wake(p: *const ()) {
    let f: fn() = mem::transmute(p);
    f();
}

unsafe fn waker_drop(_: *const ()) {
    // nop
}

//============
// Task

enum Task<F: Future + 'static> {
    Idle,
    Running(F),
    Done(F::Output),
}

impl<F: Future + 'static> Task<F> {
    const fn new() -> Self {
        Self::Idle
    }

    fn spawn(&mut self, future: impl FnOnce() -> F) {
        *self = Task::Running(future());
    }

    unsafe fn poll(&mut self, wake: fn()) {
        match self {
            Task::Idle => {}
            Task::Running(future) => {
                let future = Pin::new_unchecked(future);
                let waker_data: *const () = mem::transmute(wake);
                let waker = Waker::from_raw(RawWaker::new(waker_data, &WAKER_VTABLE));
                let mut cx = Context::from_waker(&waker);

                match future.poll(&mut cx) {
                    Poll::Ready(r) => *self = Task::Done(r),
                    Poll::Pending => {}
                };
            }
            Task::Done(_) => {}
        }
    }
}

//=============
// Yield

struct Yield {
    done: bool
}

impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            self.done = true;
            Poll::Pending
        }
    }
}

fn please_yield() -> Yield {
    Yield { done: false}
}