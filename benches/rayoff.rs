extern crate criterion_bencher_compat;
extern crate rayoff;
extern crate rayon;
extern crate threadpooler;

use rayoff::pool::Pool;
use rayon::prelude::*;
use threadpooler::pool::Pool as Pooler;

use criterion_bencher_compat as bencher;

use bencher::{benchmark_group, benchmark_main, Bencher};

const TO_TEST: usize = 100000;

fn bench_pool(bencher: &mut Bencher) {
    let pool = Pool::default();
    bencher.iter(|| {
        let mut array = [0usize; TO_TEST];
        pool.dispatch_mut(&mut array, |val: &mut usize| *val += 1);
        let expected = [1usize; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

fn bench_pooler(bencher: &mut Bencher) {
    let pool = Pooler::default();
    bencher.iter(|| {
        let mut array = [0usize; TO_TEST];
        let _ = pool
            .dispatch_mut(&mut array, |val: &mut usize| *val += 1)
            .is_ok();
        let expected = [1usize; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

fn bench_baseline(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = [0usize; TO_TEST];
        for i in array.iter_mut() {
            *i += 1;
        }
        let expected = [1usize; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

fn bench_rayon(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut array = [0usize; TO_TEST];
        array.par_iter_mut().for_each(|p| *p += 1);
        let expected = [1usize; TO_TEST];
        for i in 0..TO_TEST {
            assert_eq!(array[i], expected[i]);
        }
    })
}

benchmark_group!(
    benches,
    bench_rayon,
    bench_pool,
    bench_pooler,
    bench_baseline
);

benchmark_main!(benches);
