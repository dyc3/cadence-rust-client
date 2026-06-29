//! Poller rate limiting and auto-scaling.
//!
//! Two knobs that bound and adapt how aggressively a worker polls the Cadence
//! server:
//!
//! * [`RateLimiter`] — a token bucket that paces polls to a configured
//!   per-second rate (`WorkerActivitiesPerSecond` / `WorkerDecisionTasksPerSecond`).
//!   Rates at or above [`UNLIMITED_RPS`] disable pacing entirely.
//! * [`PollerAutoScaler`] — resizes the pool of concurrently-polling tasks
//!   between `min_pollers` and `max_pollers` based on observed poll latency
//!   versus a target (Go's `AutoScalerOptions`). When polls return quickly the
//!   worker has more work than pollers and scales up; when polls block on empty
//!   long-polls it scales down toward the minimum.
//!
//! Neither participates in workflow determinism — both live entirely on the
//! worker's polling side, never in replayed workflow code.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Notify};
use tokio::time::{interval, Instant};

/// Rates at or above this are treated as "unlimited" (no pacing).
pub const UNLIMITED_RPS: f64 = 100_000.0;

/// A token-bucket rate limiter. `acquire().await` returns immediately when a
/// token is available and otherwise sleeps just long enough for the bucket to
/// refill one token.
pub struct RateLimiter {
    rate: f64,
    burst: f64,
    state: Mutex<BucketState>,
}

struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Construct a limiter pacing to `rate_per_second`. Burst capacity equals one
    /// second of tokens (at least one), so a short idle period does not let the
    /// bucket grow unbounded.
    ///
    /// Non-finite or non-positive rates are invalid (they would make the refill
    /// path compute a non-finite/negative `Duration` and panic), so they are
    /// normalized to [`UNLIMITED_RPS`] — pacing disabled.
    pub fn new(rate_per_second: f64) -> Self {
        let rate = if rate_per_second.is_finite() && rate_per_second > 0.0 {
            rate_per_second
        } else {
            UNLIMITED_RPS
        };
        let burst = rate.max(1.0);
        Self {
            rate,
            burst,
            state: Mutex::new(BucketState {
                tokens: burst,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Whether pacing is disabled (rate at or above [`UNLIMITED_RPS`]).
    pub fn is_unlimited(&self) -> bool {
        self.rate >= UNLIMITED_RPS
    }

    /// Acquire one token, sleeping if necessary. A no-op for unlimited limiters.
    pub async fn acquire(&self) {
        if self.is_unlimited() {
            return;
        }
        loop {
            let wait = {
                let mut state = self.state.lock().await;
                let now = Instant::now();
                let elapsed = now.duration_since(state.last_refill).as_secs_f64();
                state.tokens = (state.tokens + elapsed * self.rate).min(self.burst);
                state.last_refill = now;
                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    None
                } else {
                    // Time until the bucket accrues the missing fraction of a token.
                    Some(Duration::from_secs_f64((1.0 - state.tokens) / self.rate))
                }
            };
            match wait {
                None => return,
                Some(d) => tokio::time::sleep(d).await,
            }
        }
    }
}

/// A resizable concurrency gate: at most `limit` permits may be held at once,
/// and the limit can be changed at runtime. Used to grow and shrink the active
/// poller pool without spawning or aborting tasks.
pub struct PollerGate {
    limit: AtomicUsize,
    active: AtomicUsize,
    notify: Notify,
}

impl PollerGate {
    pub fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit: AtomicUsize::new(limit.max(1)),
            active: AtomicUsize::new(0),
            notify: Notify::new(),
        })
    }

    pub fn limit(&self) -> usize {
        self.limit.load(Ordering::SeqCst)
    }

    pub fn active(&self) -> usize {
        self.active.load(Ordering::SeqCst)
    }

    /// Change the maximum number of concurrent permits, waking any waiters so a
    /// grow takes effect immediately.
    pub fn set_limit(&self, n: usize) {
        self.limit.store(n.max(1), Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Acquire a permit, waiting until the active count is below the limit.
    pub async fn acquire(self: &Arc<Self>) -> GatePermit {
        loop {
            // Register for notification *before* checking, so a concurrent
            // release/limit-change cannot be lost between the check and the wait.
            let notified = self.notify.notified();
            tokio::pin!(notified);

            let active = self.active.load(Ordering::SeqCst);
            if active < self.limit.load(Ordering::SeqCst) {
                if self
                    .active
                    .compare_exchange(active, active + 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    return GatePermit { gate: self.clone() };
                }
                // Lost the race; retry without sleeping.
                continue;
            }
            notified.await;
        }
    }
}

/// RAII permit; releasing it frees a slot in the [`PollerGate`].
pub struct GatePermit {
    gate: Arc<PollerGate>,
}

impl Drop for GatePermit {
    fn drop(&mut self) {
        self.gate.active.fetch_sub(1, Ordering::SeqCst);
        self.gate.notify.notify_one();
    }
}

/// Drives the size of a [`PollerGate`] between `min` and `max` based on observed
/// poll latency. Pollers call [`record_poll`](Self::record_poll) after each poll;
/// a control loop ([`run`](Self::run)) periodically recomputes the target size.
pub struct PollerAutoScaler {
    gate: Arc<PollerGate>,
    min: usize,
    max: usize,
    target: Duration,
    eval_interval: Duration,
    /// Accumulated poll samples for the current evaluation window. Count and
    /// latency are guarded together so `record_poll` cannot split a sample across
    /// a concurrent `recompute`.
    samples: std::sync::Mutex<SampleWindow>,
}

#[derive(Default)]
struct SampleWindow {
    count: u64,
    latency_sum_micros: u64,
}

impl PollerAutoScaler {
    pub fn new(
        min_pollers: usize,
        max_pollers: usize,
        target_poll_duration: Duration,
    ) -> Arc<Self> {
        let min = min_pollers.max(1);
        let max = max_pollers.max(min);
        Arc::new(Self {
            gate: PollerGate::new(min),
            min,
            max,
            target: target_poll_duration.max(Duration::from_millis(1)),
            // Re-evaluate at least every 100ms, and no faster than the target.
            eval_interval: target_poll_duration.max(Duration::from_millis(100)),
            samples: std::sync::Mutex::new(SampleWindow::default()),
        })
    }

    pub fn gate(&self) -> Arc<PollerGate> {
        self.gate.clone()
    }

    /// Record the latency of one completed poll.
    pub fn record_poll(&self, latency: Duration) {
        let mut samples = self.samples.lock().unwrap();
        samples.count += 1;
        samples.latency_sum_micros += latency.as_micros() as u64;
    }

    /// Fold the accumulated samples into a new gate size. Resets the window.
    pub fn recompute(&self) {
        let (count, sum) = {
            let mut samples = self.samples.lock().unwrap();
            let window = (samples.count, samples.latency_sum_micros);
            *samples = SampleWindow::default();
            window
        };
        if count == 0 {
            return;
        }
        let avg = Duration::from_micros(sum / count);
        let current = self.gate.limit();
        let next = next_poller_count(current, self.min, self.max, avg, self.target);
        if next != current {
            self.gate.set_limit(next);
        }
    }

    /// Periodic control loop; runs until the worker is stopped (task aborted).
    pub async fn run(self: Arc<Self>) {
        let mut tick = interval(self.eval_interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            self.recompute();
        }
    }
}

/// Pure scaling decision: scale up by one when average poll latency is
/// comfortably below target (work is plentiful), down by one when comfortably
/// above (pollers are idling on empty long-polls), and hold in the dead band.
/// Always clamped to `[min, max]`.
pub fn next_poller_count(
    current: usize,
    min: usize,
    max: usize,
    avg_poll_latency: Duration,
    target: Duration,
) -> usize {
    let low = target.mul_f64(0.8);
    let high = target.mul_f64(1.2);
    if avg_poll_latency < low {
        (current + 1).min(max)
    } else if avg_poll_latency > high {
        current.saturating_sub(1).max(min)
    } else {
        current.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_poller_count_scales_up_when_polls_fast() {
        // avg well below target → more work than pollers → scale up.
        let next = next_poller_count(
            2,
            1,
            8,
            Duration::from_millis(10),
            Duration::from_millis(100),
        );
        assert_eq!(next, 3);
    }

    #[test]
    fn test_next_poller_count_scales_down_when_polls_slow() {
        // avg well above target → pollers idling → scale down.
        let next = next_poller_count(
            5,
            1,
            8,
            Duration::from_millis(500),
            Duration::from_millis(100),
        );
        assert_eq!(next, 4);
    }

    #[test]
    fn test_next_poller_count_holds_in_dead_band() {
        let next = next_poller_count(
            3,
            1,
            8,
            Duration::from_millis(100),
            Duration::from_millis(100),
        );
        assert_eq!(next, 3);
    }

    #[test]
    fn test_next_poller_count_clamps_to_bounds() {
        // Cannot exceed max.
        assert_eq!(
            next_poller_count(
                8,
                1,
                8,
                Duration::from_millis(1),
                Duration::from_millis(100)
            ),
            8
        );
        // Cannot drop below min.
        assert_eq!(
            next_poller_count(1, 1, 8, Duration::from_secs(10), Duration::from_millis(100)),
            1
        );
    }

    #[test]
    fn test_recompute_grows_and_shrinks_gate() {
        let scaler = PollerAutoScaler::new(1, 4, Duration::from_millis(100));
        assert_eq!(scaler.gate().limit(), 1);

        // Fast polls → grow.
        scaler.record_poll(Duration::from_millis(5));
        scaler.record_poll(Duration::from_millis(5));
        scaler.recompute();
        assert_eq!(scaler.gate().limit(), 2);

        // Slow polls → shrink.
        scaler.record_poll(Duration::from_millis(900));
        scaler.recompute();
        assert_eq!(scaler.gate().limit(), 1);

        // No samples → unchanged.
        scaler.recompute();
        assert_eq!(scaler.gate().limit(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_invalid_rate_is_treated_as_unlimited() {
        // Zero, negative, and non-finite rates must not reach the refill path (which
        // would compute a non-finite/negative Duration and panic); they disable pacing.
        for rate in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let limiter = RateLimiter::new(rate);
            assert!(limiter.is_unlimited(), "rate {rate} should be unlimited");
            // Acquiring many times never blocks or panics.
            for _ in 0..1000 {
                limiter.acquire().await;
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_rate_limiter_paces_acquisitions() {
        // At 10/s with a 1s burst, the first ~10 tokens are immediate; the next
        // batch must wait ~0.1s each.
        let limiter = RateLimiter::new(10.0);
        let start = Instant::now();
        for _ in 0..20 {
            limiter.acquire().await;
        }
        // 20 acquisitions, 10 burst → ~10 paced at 0.1s ≈ 1s of virtual time.
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(900),
            "expected pacing to take ~1s, took {elapsed:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn test_unlimited_rate_limiter_never_waits() {
        let limiter = RateLimiter::new(UNLIMITED_RPS);
        assert!(limiter.is_unlimited());
        let start = Instant::now();
        for _ in 0..1000 {
            limiter.acquire().await;
        }
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test]
    async fn test_gate_limits_concurrency() {
        let gate = PollerGate::new(1);
        let _p1 = gate.acquire().await;
        assert_eq!(gate.active(), 1);

        // Second acquire must not complete while the first permit is held.
        let g2 = gate.clone();
        let pending =
            tokio::time::timeout(Duration::from_millis(50), async move { g2.acquire().await })
                .await;
        assert!(pending.is_err(), "gate allowed over-subscription");
    }

    #[tokio::test]
    async fn test_gate_grow_admits_waiter() {
        let gate = PollerGate::new(1);
        let _p1 = gate.acquire().await;

        let g2 = gate.clone();
        let handle = tokio::spawn(async move { g2.acquire().await });

        // Grow the gate; the waiter should now be admitted.
        gate.set_limit(2);
        let permit = tokio::time::timeout(Duration::from_millis(100), handle)
            .await
            .expect("waiter timed out")
            .expect("task panicked");
        assert_eq!(gate.active(), 2);
        drop(permit);
    }
}
