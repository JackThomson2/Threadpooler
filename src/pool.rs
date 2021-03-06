use crate::job::Job;

use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use std::thread::spawn;
use std::{sync::Arc, thread::Builder};

type SendRef = Sender<Arc<Job>>;

#[derive(Debug)]
pub struct Pool {
    senders: Mutex<Vec<SendRef>>,
    num_threads: usize,
}

impl Default for Pool {
    fn default() -> Self {
        let num_threads = Pool::get_thread_count();
        let mut senders = Vec::with_capacity(num_threads);
        (0..num_threads).for_each(|i| {
            let (sender, recvr): (Sender<Arc<Job>>, Receiver<Arc<Job>>) = unbounded();
            let builder = Builder::new()
                .name(format!("Worker thread {}", i))
                .stack_size(10_000);
            let _ = builder.spawn(move || {
                for job in recvr.iter() {
                    job.execute()
                }
            });
            senders.push(sender);
        });
        let senders = senders;
        Self {
            senders: Mutex::new(senders),
            num_threads,
        }
    }
}

impl Pool {
    pub fn get_thread_count() -> usize {
        num_cpus::get() as usize
    }

    #[inline]
    pub fn dispatch_mut<F, A>(&self, elems: &mut [A], func: F)
    where
        F: Fn(&mut A) + Send + Sync,
    {
        // Job must wait to completion before this frame returns
        let job = unsafe { Job::new(elems, func, self.num_threads) };
        let job = Arc::new(job);
        self.notify_all(job.clone());
        job.wait();
    }

    #[inline]
    pub fn map<F, A, B>(&self, inputs: &[A], func: F) -> Vec<B>
    where
        B: Default + Clone,
        F: (Fn(&A) -> B) + Send + Sync,
    {
        let mut outs = Vec::new();
        outs.resize(inputs.len(), B::default());
        let mut elems: Vec<(&A, &mut B)> = inputs.iter().zip(outs.iter_mut()).collect();
        self.dispatch_mut(&mut elems, move |item: &mut (&A, &mut B)| {
            *item.1 = func(item.0);
        });
        outs
    }

    #[inline]
    fn notify_all(&self, job: Arc<Job>) {
        let senders = self.senders.lock();
        for s in senders.iter() {
            s.send(job.clone()).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_pool() {
        let pool = Pool::default();
        let mut array = [0usize; 100];
        pool.dispatch_mut(&mut array, |val: &mut usize| *val += 1);
        let expected = [1usize; 100];
        for i in 0..100 {
            assert_eq!(array[i], expected[i]);
        }
    }
    #[test]
    fn test_map() {
        let pool = Pool::default();
        let array = [0usize; 100];
        let output = pool.map(&array, |val: &usize| val + 1);
        let expected = [1usize; 100];
        for i in 0..100 {
            assert_eq!(expected[i], output[i]);
        }
    }
    #[test]
    fn test_static_reentrancy() {
        thread_local!(static RAYOFF_POOL: RefCell<Pool> = RefCell::new(Pool::default()));
        let array = [0usize; 100];
        let output = RAYOFF_POOL.with(|pool| {
            pool.borrow().map(&array, |val: &usize| {
                let array = [0usize; 100];
                let rv = RAYOFF_POOL.with(|pool| pool.borrow().map(&array, |val| val + 1));
                let total: usize = rv.into_iter().sum();
                val + total
            })
        });
        let expected = [100usize; 100];
        for i in 0..100 {
            assert_eq!(expected[i], output[i]);
        }
    }

    #[test]
    fn test_job_order_1() {
        let pool = Pool::default();
        let mut elems = [0usize; 100];
        // Job must wait to completion before this frame returns
        let job = unsafe {
            Job::new(
                &mut elems,
                |val: &mut usize| {
                    assert_eq!(*val, 0);
                    *val += 1
                },
                1,
            )
        };
        let job = Arc::new(job);
        job.execute();
        pool.notify_all(job.clone());
        job.wait();
    }

    #[test]
    fn test_job_order_2() {
        let pool = Pool::default();
        let mut elems = [0usize; 100];
        // Job must wait to completion before this frame returns
        let job = unsafe {
            Job::new(
                &mut elems,
                |val: &mut usize| {
                    assert_eq!(*val, 0);
                    *val += 1
                },
                1,
            )
        };
        let job = Arc::new(job);
        job.execute();
        job.wait();
        pool.notify_all(job.clone());
    }
}
