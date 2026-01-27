pub mod cache;
pub mod zoned;

use rand::distributions::{Alphanumeric, DistString};

pub fn nonce(size: usize) -> String {
    let mut rng = rand::thread_rng();
    Alphanumeric.sample_string(&mut rng, size)
}
