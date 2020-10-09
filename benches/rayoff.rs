extern crate criterion_bencher_compat;
extern crate rayoff;
extern crate rayon;
extern crate threadpooler;

use rayoff::pool::Pool;
use rayon::prelude::*;
use threadpooler::pool::Pool as Pooler;

use criterion::black_box;
use criterion_bencher_compat as bencher;

use bencher::{benchmark_group, benchmark_main, Bencher};

const TO_TEST: usize = 10_000;
const UP_TO: usize = 1000;

fn rayoff_bench_pool(bencher: &mut Bencher) {
    let pool = Pool::default();
    bencher.iter(|| {
        let mut array = vec![0usize; TO_TEST];
        pool.dispatch_mut(&mut array, |val: &mut usize| {
            for i in 0..UP_TO {
                *val += black_box(1);
            }
        });
        let expected = vec![UP_TO; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

fn rayoff_bench_pooler(bencher: &mut Bencher) {
    let pool = Pooler::default();
    bencher.iter(|| {
        let mut array = vec![0usize; TO_TEST];
        let _ = pool.dispatch_mut(&mut array, |val: &mut usize| {
            for i in 0..UP_TO {
                *val += black_box(1);
            }
        });
        let expected = vec![UP_TO; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

fn rayoff_bench_baseline(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = vec![0usize; TO_TEST];
        for i in array.iter_mut() {
            for x in 0..UP_TO {
                *i += black_box(1);
            }
        }
        let expected = vec![UP_TO; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

fn rayoff_bench_rayon(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = vec![0usize; TO_TEST];
        array.par_iter_mut().for_each(|p| {
            for i in 0..UP_TO {
                *p += black_box(1);
            }
        });
        let expected = vec![UP_TO; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

benchmark_group!(
    rayoff,
    rayoff_bench_pool,
    rayoff_bench_pooler,
    rayoff_bench_rayon,
    rayoff_bench_baseline
);

benchmark_main!(rayoff);
