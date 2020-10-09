use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

const FREE: u8 = 0b0000_0000;
const OWNED: u8 = 0b0000_0001;

const COMPLETE: u8 = 0b000_0010;

const PENDINGSHARE: u8 = 0b0000_0100;
const SHARING: u8 = 0b0000_1000;

#[derive(Debug)]
pub struct LockedBatch {
    state: AtomicU8,
    cntr: AtomicUsize,
    workers: AtomicU8,
    fast_cntr: UnsafeCell<usize>,

    waker_lock: Arc<(Mutex<bool>, Condvar)>,

    pub offset: usize,
    pub count: usize,
}

/// Safe because ctx/elems are only touched until job is complete.
/// Job::wait must be called to complete the job
unsafe impl Send for LockedBatch {}

/// Safe because data access either atomic or read only
/// elems atomic access is guarded by the work_index
/// Job::wait must be called to complete the job
unsafe impl Sync for LockedBatch {}

impl LockedBatch {
    pub fn new(count: usize, offset: usize) -> LockedBatch {
        LockedBatch {
            state: AtomicU8::new(FREE),
            cntr: AtomicUsize::new(0),
            workers: AtomicU8::new(0),
            fast_cntr: UnsafeCell::new(0),
            waker_lock: Arc::new((Mutex::new(false), Condvar::new())),

            offset,
            count,
        }
    }

    #[inline]
    pub fn is_wanting_share(&self) -> bool {
        (self.state.load(Ordering::Acquire) & PENDINGSHARE) != 0
    }

    #[inline]
    pub fn spin_wait(&self) {
        // let now = Instant::now();
        // println!("Waiting for pool {}", self.offset);

        let &(ref lock, ref cvar) = &*(self.waker_lock.clone());

        let mut started = lock.lock();
        if !*started {
            cvar.wait(&mut started);
        }

        // println!("Spin lock took {:#?} for pool {}", now.elapsed(), self.offset);
    }

    #[inline]
    pub fn request_share(&self) {
        self.state.fetch_or(PENDINGSHARE, Ordering::Release);
    }

    #[inline]
    pub fn is_sharing(&self) -> bool {
        (self.state.load(Ordering::Acquire) & SHARING) != 0
    }

    #[inline]
    pub fn is_free(&self) -> bool {
        self.state.load(Ordering::Acquire) == 0
    }

    #[inline]
    pub fn own(&self) -> bool {
        let ok = self
            .state
            .compare_exchange(FREE, OWNED, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();

        if ok {
            // println!("Pool {} is now owned", self.offset);
            self.workers.fetch_add(1, Ordering::Relaxed);
        }

        ok
    }

    #[inline]
    pub fn force_inc_counter(&self, amount: usize) {
        self.cntr.fetch_add(amount, Ordering::Relaxed);

        //Tidy up
        let &(ref lock, ref cvar) = &*(self.waker_lock.clone());
        let mut started = lock.lock();
        *started = true;
        cvar.notify_all();
    }

    #[inline]
    pub fn get_next_index(&self) -> usize {
        let is_sharing = self.is_sharing();
        if !is_sharing && self.is_wanting_share() {
            // println!("Pool offset {} is now sharing", self.offset);
            self.cntr
                .fetch_add(unsafe { *(self.fast_cntr.get()) }, Ordering::Release);
            self.state.fetch_or(SHARING, Ordering::Release);

            let &(ref lock, ref cvar) = &*(self.waker_lock.clone());
            let mut started = lock.lock();
            *started = true;
            cvar.notify_all();

            return self.cntr.fetch_add(1, Ordering::Relaxed);
        }

        if !is_sharing {
            unsafe {
                let index = *self.fast_cntr.get();
                *self.fast_cntr.get() = index + 1;

                index
            }
        } else {
            self.cntr.fetch_add(1, Ordering::Acquire)
        }
    }

    #[inline]
    pub fn add_worker(&self) {
        if !self.is_sharing() {
            if !self.is_wanting_share() {
                self.request_share();
            }
            self.workers.fetch_add(1, Ordering::Relaxed);
            self.spin_wait();
            return;
        }
        self.workers.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_weight(&self) -> usize {
        let workers = self.workers.load(Ordering::Relaxed) as usize;

        if workers > 1 {
            if self.is_sharing() {
                (self.count.saturating_sub(self.get_current_index())) / workers
            } else {
                (unsafe { (self.count.saturating_sub(*self.fast_cntr.get())) / 5 }) / workers
            }
        } else if workers == 1 {
            unsafe { (self.count.saturating_sub(*self.fast_cntr.get())) / 10 }
        } else {
            self.count
        }
    }

    #[inline]
    pub fn get_current_index(&self) -> usize {
        self.cntr.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn is_complete(&self) -> bool {
        self.cntr.load(Ordering::Relaxed) >= self.count
    }
}
