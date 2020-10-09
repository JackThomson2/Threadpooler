use crate::lockedBatch::LockedBatch;

use parking_lot::{Condvar, Mutex};
use std::cmp;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use smallvec::SmallVec;

pub struct Job {
    func: Progress,
    ctx: *mut c_void,
    elems: *mut c_void,
    num: usize,
    done_index: AtomicUsize,
    waker_lock: Arc<(Mutex<bool>, Condvar)>,

    locked_batch: SmallVec<[LockedBatch; 64]>,
}

/// Safe because ctx/elems are only touched until job is complete.
/// Job::wait must be called to complete the job
unsafe impl Send for Job {}

/// Safe because data access either atomic or read only
/// elems atomic access is guarded by the work_index
/// Job::wait must be called to complete the job
unsafe impl Sync for Job {}

type Progress = extern "C" fn(*mut c_void, *mut c_void, u64);

impl Job {
    /// This function is unsafe because the Job object must
    /// be complete.  Users must call Job::wait befor returning.
    #[inline]
    pub unsafe fn new<F, A>(elems: &mut [A], func: F, cores: usize) -> Self
    where
        F: Fn(&mut A) + Send + Sync,
    {
        let mut closure = |ptr: *mut c_void, index: u64| {
            let elem: &mut A = std::mem::transmute((ptr as *mut A).add(index as usize));
            (func)(elem)
        };

        let cores = cmp::max(cores, 1) * 8;

        let estim_per_batch = elems.len() / cores;
        let batch_overflow = elems.len() % cores;

        let locked_batch = (0..cores)
            .map(|i| {
                let (num, offset) = if i == 0 {
                    (estim_per_batch + batch_overflow, 0)
                } else {
                    (estim_per_batch, (i * estim_per_batch) + batch_overflow)
                };

                LockedBatch::new(num, offset)
            })
            .collect();

        let (ctx, func) = Job::unpack_closure(&mut closure);
        Self {
            elems: elems.as_mut_ptr() as *mut c_void,
            num: elems.len(),
            done_index: AtomicUsize::new(0),
            func,
            ctx,
            locked_batch,
            waker_lock: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    /// This function unpacks the closure into a context and a trampoline
    /// Source: https://s3.amazonaws.com/temp.michaelfbryan.com/callbacks/index.html
    #[inline]
    unsafe fn unpack_closure<F>(closure: &mut F) -> (*mut c_void, Progress)
    where
        F: FnMut(*mut c_void, u64),
    {
        extern "C" fn trampoline<F>(data: *mut c_void, elems: *mut c_void, n: u64)
        where
            F: FnMut(*mut c_void, u64),
        {
            let closure: &mut F = unsafe { &mut *(data as *mut F) };
            (*closure)(elems, n);
        }

        (closure as *mut F as *mut c_void, trampoline::<F>)
    }

    #[inline]
    fn get_free_slot(&self) -> Option<&LockedBatch> {
        // First try to find a free slot
        for slot in self.locked_batch.iter() {
            if slot.is_free() {
                if !slot.own() {
                    continue;
                }
                return Some(slot);
            }
        }

        let found = self
            .locked_batch
            .iter()
            .filter(|slot| !slot.is_complete())
            .max_by(|slot, slot2| slot.get_weight().cmp(&slot2.get_weight()));

        if let Some(slot) = found {
            slot.add_worker();
            return Some(slot);
        }
        None
    }

    #[inline]
    pub fn execute(&self) {
        let mut completed = 0;

        loop {
            let free_slot = self.get_free_slot();
            if free_slot.is_none() {
                break;
            }

            let locked_batch = free_slot.unwrap();
            loop {
                let index = locked_batch.get_next_index();

                if index >= locked_batch.count {
                    if !locked_batch.is_sharing() {
                        locked_batch.force_inc_counter(index);
                    }
                    break;
                }

                (self.func)(self.ctx, self.elems, (index + locked_batch.offset) as u64);
                completed += 1;
            }

            // println!("Finished work with pool: {}", locked_batch.offset);
        }
        // println!("Thread is closing... we did {}", completed);
        let done = self.done_index.fetch_add(completed, Ordering::Release);

        if done + completed >= self.num {
            let &(ref lock, ref cvar) = &*(self.waker_lock.clone());
            let mut started = lock.lock();
            *started = true;
            cvar.notify_all();
        }
    }

    #[inline]
    pub fn wait(&self) {
        let &(ref lock, ref cvar) = &*(self.waker_lock.clone());

        let mut started = lock.lock();
        if !*started {
            cvar.wait(&mut started);
        }
    }
}
