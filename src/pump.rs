//! Event-driven main-thread pump.
//!
//! The app's per-frame housekeeping (draining the mouse-hook / tray /
//! hotkey channels, floating-window callbacks, OCR-result polling, …)
//! used to run off an always-on 8 ms `slint::Timer` — 125 wake-ups a
//! second even when Snipdeck sits idle in the tray, which keeps the CPU
//! out of its deep sleep states.
//!
//! This module runs that same work only while there is something to do,
//! then **stops the timer** so the Slint/winit event loop blocks and the
//! process truly sleeps until woken. Wake-ups come from the sources that
//! can produce work while we're idle — the global mouse/keyboard hook,
//! the tray thread, global hotkeys, floating-window callbacks and async
//! OCR/translate completions — each of which calls [`wake`] after
//! enqueuing its event.
//!
//! Everything the pump touches lives on the main thread, so [`wake`]
//! (callable from any thread) just flips an atomic and, if we were
//! asleep, posts a resume onto the event loop via
//! `invoke_from_event_loop`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// `true` only while the timer is stopped (loop asleep). Starts awake so
/// early `wake()` calls (before `install`) are cheap no-ops.
static SLEEPING: AtomicBool = AtomicBool::new(false);

thread_local! {
    static PUMP: RefCell<Option<Pump>> = const { RefCell::new(None) };
}

struct Pump {
    timer: slint::Timer,
    interval: Duration,
    /// One iteration of housekeeping. Returns `true` while the app still
    /// wants to run (it either wants to stay awake, or it did work this
    /// iteration and shouldn't be put to sleep mid-burst).
    step: Rc<dyn Fn() -> bool>,
    /// Consecutive idle iterations. We only sleep after a short grace so
    /// brief lulls — and the lag before a global hotkey reaches its queue
    /// after the keyboard hook woke us — don't thrash the timer.
    idle_ticks: Cell<u32>,
}

/// Continuous-idle iterations before we stop the timer. At the 8 ms
/// interval this is ~250 ms.
const GRACE_TICKS: u32 = 32;

/// Install and start the pump. `step` runs one iteration and returns
/// `true` while there is still anything to do. Call once, on the main
/// thread, from the event loop.
pub fn install(interval: Duration, step: impl Fn() -> bool + 'static) {
    let step: Rc<dyn Fn() -> bool> = Rc::new(step);
    SLEEPING.store(false, Ordering::SeqCst);
    PUMP.with(|p| {
        *p.borrow_mut() = Some(Pump {
            timer: slint::Timer::default(),
            interval,
            step,
            idle_ticks: Cell::new(0),
        });
    });
    arm_timer();
}

/// (Re)start the repeating timer with the stored step, resetting the idle
/// counter. Only ever borrows `PUMP` shared, and Slint's `Timer` methods
/// take `&self`, so this is safe to call from inside a timer callback.
fn arm_timer() {
    PUMP.with(|p| {
        if let Some(pump) = &*p.borrow() {
            pump.idle_ticks.set(0);
            pump.timer
                .start(slint::TimerMode::Repeated, pump.interval, on_tick);
        }
    });
}

/// Timer callback: run one iteration; after a grace period of idle, stop
/// the timer so the loop sleeps.
fn on_tick() {
    let active = run_step();
    PUMP.with(|p| {
        let borrow = p.borrow();
        let Some(pump) = &*borrow else { return };
        if active {
            pump.idle_ticks.set(0);
            return;
        }
        let n = pump.idle_ticks.get() + 1;
        pump.idle_ticks.set(n);
        if n < GRACE_TICKS {
            return;
        }
        // Sleep handshake. Mark ourselves sleeping, then run ONE more
        // iteration as a re-check. This closes the wake/sleep race: a
        // `wake()` that fires around now either
        //   (a) enqueued its event before this re-check, which then drains
        //       it and reports active (so we stay awake), or
        //   (b) observes `SLEEPING == true` and posts a resume.
        // Either way no wake-up can be lost.
        SLEEPING.store(true, Ordering::SeqCst);
        if run_step() {
            SLEEPING.store(false, Ordering::SeqCst);
            pump.idle_ticks.set(0);
        } else {
            pump.timer.stop();
            log::debug!("pump: idle, timer stopped (event loop sleeping)");
            // Idempotent guard against a re-entrant wake. The re-check
            // `run_step()` above can pump posted messages (main.rs: the share
            // sheet's WinRT `.get()`), so a `wake()` can run nested INSIDE it —
            // flipping SLEEPING back to false and arming the timer, but with the
            // resume it queued already consumed there. We then stop the very
            // timer it armed, stranding the loop asleep with SLEEPING == false,
            // where every future `wake()` no-ops. If a nested wake un-slept us,
            // re-arm so the loop keeps ticking.
            if !SLEEPING.load(Ordering::SeqCst) {
                arm_timer();
            }
        }
    });
}

/// Run the stored step, holding no `PUMP` borrow while it executes (the
/// step touches `AppState`, never the pump).
fn run_step() -> bool {
    let step = PUMP.with(|p| p.borrow().as_ref().map(|pump| pump.step.clone()));
    step.map(|s| s()).unwrap_or(false)
}

/// Wake the pump from any thread. Callers enqueue their event **first**,
/// then call this. Coalesced: a cheap no-op while already awake.
pub fn wake() {
    // If we were already awake, another iteration will drain the event.
    if !SLEEPING.swap(false, Ordering::SeqCst) {
        return;
    }
    if slint::invoke_from_event_loop(|| {
        log::debug!("pump: wake");
        arm_timer();
        // Handle the just-enqueued event now instead of waiting a full
        // interval for the first tick.
        let _ = run_step();
    })
    .is_err()
    {
        // Posting failed (realistically only before the event loop starts or
        // during shutdown). We already swapped SLEEPING to false, but nothing
        // will re-arm the timer, so a later wake would no-op with the loop
        // asleep. Roll the swap back so the next wake can retry.
        SLEEPING.store(true, Ordering::SeqCst);
    }
}
