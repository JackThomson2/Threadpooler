extern crate threadpooler;

use threadpooler::pool::Pool;

const to_test: usize = 100000;

fn main() {
    let pool = Pool::default();
    let mut array = [0usize; to_test];
    for _i in 0..10000 {
        pool.dispatch_mut(&mut array, |val: &mut usize| *val += 1);
    }
}
