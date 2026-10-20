use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Cached<T> {
    value: Mutex<Option<(Instant, T)>>,
    refreshing: AtomicBool,
    ttl: Duration,
}

impl<T: Clone + Send + 'static> Cached<T> {
    pub const fn new(ttl: Duration) -> Self {
        Cached { value: Mutex::new(None), refreshing: AtomicBool::new(false), ttl }
    }

    pub fn get(self: &'static Self, compute: fn() -> T) -> T {
        let current = self.value.lock().unwrap_or_else(|e| e.into_inner()).clone();
        match current {
            Some((at, value)) => {
                if at.elapsed() > self.ttl && !self.refreshing.swap(true, Ordering::SeqCst) {
                    std::thread::spawn(move || {
                        let fresh = compute();
                        *self.value.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), fresh));
                        self.refreshing.store(false, Ordering::SeqCst);
                    });
                }
                value
            }
            None => {
                let fresh = compute();
                *self.value.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), fresh.clone()));
                fresh
            }
        }
    }

    pub fn clear(&self) {
        *self.value.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static CACHE: Cached<usize> = Cached::new(Duration::from_millis(30));

    fn compute() -> usize {
        CALLS.fetch_add(1, Ordering::SeqCst) + 1
    }

    #[test]
    fn serves_cached_values_and_refreshes_stale_ones_in_the_background() {
        assert_eq!(CACHE.get(compute), 1);
        assert_eq!(CACHE.get(compute), 1);
        std::thread::sleep(Duration::from_millis(40));
        assert_eq!(CACHE.get(compute), 1);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(CACHE.get(compute), 2);
        CACHE.clear();
        assert_eq!(CACHE.get(compute), 3);
    }
}
