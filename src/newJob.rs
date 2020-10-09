use crossbeam_deque::{Steal, Worker};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Job<F, T>
where
    F: Fn(*mut T),
{
    complete: *const AtomicU64,
    method: *const F,
    data: *mut T,
}

/// Safe because ctx/elems are only touched until job is complete.
/// Job::wait must be called to complete the job
unsafe impl<F, T> Send for Job<F, T> where F: Fn(*mut T) {}

/// Safe because data access either atomic or read only
/// elems atomic access is guarded by the work_index
/// Job::wait must be called to complete the job
unsafe impl<F, T> Sync for Job<F, T> where F: Fn(*mut T) {}

impl<F, T> Job<F, T>
where
    F: Fn(*mut T) + Send + Sync,
{
    pub fn create_job(data: &mut [T], method: &F, worker: &Worker<Self>, complete: &AtomicU64) {
        for job in data {
            worker.push(Self {
                complete: complete as *const AtomicU64,
                method,
                data: job,
            })
        }
    }

    #[inline]
    pub fn run(&mut self) {
        unsafe {
            (*self.method)(self.data);
            (*self.complete).fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod job_tests {
    use std::thread::spawn;

    use super::*;

    #[test]
    fn test_pool() {
        let mut array = [0usize; 100];
        let method = |u: *mut usize| unsafe { *u += 1 };

        {
            let complete = AtomicU64::new(0);
            let pool = Worker::new_fifo();
            Job::create_job(&mut array, &method, &pool, &complete);

            loop {
                let done = complete.load(Ordering::Relaxed);

                let new_job = pool.pop();

                if let Some(mut job) = new_job {
                    job.run();
                }

                println!("Completed {}", done);

                if done == 99 {
                    break;
                }
            }
        }

        let expected = [1usize; 100];
        for i in 0..100 {
            assert_eq!(expected[i], expected[i]);
        }
    }

    #[test]
    fn test_threaded_pool() {
        let mut array = [0usize; 100];
        let method = |u: *mut usize| unsafe { *u += 1 };

        {
            let mut threads = vec![];
            let complete = AtomicU64::new(0);
            let pool = Worker::new_fifo();
            Job::create_job(&mut array, &method, &pool, &complete);

            for i in 0..4 {
                let worker = pool.stealer().clone();
                let a = spawn(move || {
                    println!("Thread running...");
                    loop {
                        if let Steal::Success(mut job) = worker.steal() {
                            job.run();
                        }
                    }
                });

                threads.push(a);
            }

            loop {
                let done = complete.load(Ordering::Relaxed);
                if done == 99 {
                    break;
                }
            }
        }

        let expected = [1usize; 100];
        for i in 0..100 {
            assert_eq!(expected[i], expected[i]);
        }
    }
}
