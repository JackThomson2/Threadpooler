use threadpooler::pool::Pool;

use criterion::black_box;

const to_test: usize = 100_000;

fn main() {
    let pool = Pool::default();
    let mut array = vec![0usize; to_test];
    for _i in 0..10_000 {
        pool.dispatch_mut(&mut array, |val: &mut usize| {
            for i in 0..1_000 {
                *val += black_box(1)
            }
        });
        println!("Done {}", _i);
    }

    println!("Done")
}
