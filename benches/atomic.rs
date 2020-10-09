use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use criterion::black_box;
use criterion_bencher_compat as bencher;

use bencher::{benchmark_group, benchmark_main, Bencher};

const COUNT: usize = 1_000_000;

fn atomic_load(bencher: &mut Bencher) {
    let cntr = AtomicUsize::new(0);

    bencher.iter(|| {
        for i in 0..COUNT {
            let _a = black_box(cntr.load(Ordering::Relaxed));
        }
    })
}

fn atomic_add(bencher: &mut Bencher) {
    let cntr = AtomicUsize::new(0);
    bencher.iter(|| {
        for i in 0..COUNT {
            cntr.fetch_add(black_box(1), Ordering::Relaxed);
        }
    })
}

fn non_atomic_add(bencher: &mut Bencher) {
    let mut cntr: usize = 0;
    bencher.iter(|| {
        for i in 0..COUNT {
            cntr += black_box(1usize);
        }
    })
}

fn unsafe_add(bencher: &mut Bencher) {
    let cntr = UnsafeCell::new(0usize);
    bencher.iter(|| {
        for i in 0..COUNT {
            unsafe {
                *(cntr.get()) += black_box(1);
            }
        }
    })
}

fn atomic_toggle(bencher: &mut Bencher) {
    let cntr = AtomicBool::new(false);
    let mut cnt = 0;
    bencher.iter(|| {
        for i in 0..COUNT {
            let b = cntr.fetch_xor(black_box(true), Ordering::Relaxed);

            if !b {
                cnt += 1;
            }
        }
    })
}

fn atomic_bool(bencher: &mut Bencher) {
    let cntr = AtomicBool::new(false);
    let mut cnt = 0;
    bencher.iter(|| {
        for i in 0..COUNT {
            let b = black_box(cntr.load(Ordering::Relaxed));

            if !b {
                cnt += 1;
            }
        }
    })
}

fn atomic_bool_seq(bencher: &mut Bencher) {
    let cntr = AtomicBool::new(false);
    let mut cnt = 0;
    bencher.iter(|| {
        for i in 0..COUNT {
            let b = black_box(cntr.load(Ordering::SeqCst));

            if !b {
                cnt += 1;
            }
        }
    })
}

benchmark_group!(
    benches,
    atomic_add,
    unsafe_add,
    non_atomic_add,
    atomic_load,
    atomic_bool,
    atomic_bool_seq,
    atomic_toggle
);

benchmark_main!(benches);
