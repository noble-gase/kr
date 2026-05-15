pub mod redkit;
pub mod zoned;

use rand::distr::{Alphanumeric, SampleString};

pub fn nonce(size: usize) -> String {
    let mut rng = rand::rng();
    Alphanumeric.sample_string(&mut rng, size)
}
