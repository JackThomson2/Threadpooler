extern crate threadpooler;

use threadpooler::pool::Pool;

const to_test: usize = 32;

fn main() {
    let pool = Pool::default();
    let mut array = [0usize; to_test];
    for _i in 0..1 {
        pool.dispatch_mut(&mut array, |val: &mut usize| *val += 1);
    }

    println!("{:?}", array);
}
