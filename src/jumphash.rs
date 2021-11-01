use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
// Package jump implements Google's Jump Consistent Hash
/*
From the paper "A Fast, Minimal Memory, Consistent Hash Algorithm" by John Lamping, Eric Veach (2014).
http://arxiv.org/abs/1406.2294
*/
/// hashes an &str to a u32 which is in the range of 0..buckets
/// expects a number of buckets greater than zero
fn jump_hash_from_str(key: &str, buckets: u32) -> u32 {
    assert!(buckets >= 1);
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let key = hasher.finish();
    jump_hash_from_u64(key, buckets)
}

// A constant from a 64 bit linear congruential generator found in the original paper
// but explained here:
// https://nuclear.llnl.gov/CNP/rng/rngman/node4.html
// Jump hash has a 64 bit pseudo-random generator 'embedded' in it
const SIXTY_FOUR_BIT_LINEAR_CONGRUENTIAL_GENERATOR_A_CONSTANT: i64 = 2862933555777941757;
/// hashes a u64 to a u32 which is in the range of 0..buckets
/// expects a number of buckets greater than zero
fn jump_hash_from_u64(key: u64, buckets: u32) -> u32 {
    assert!(buckets >= 1);
    let mut b: i64 = -1;
    let mut j: i64 = 0;
    let mut key = key;

    while j < buckets as i64 {
        b = j;
        key = key.wrapping_mul( 2862933555777941757).wrapping_add(1);
        j = (((b.wrapping_add(1) as f64) * ((1i64 << 31) as f64)) / ((key >> 33).wrapping_add(1) as f64)) as i64;
    }
    b as u32
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    #[should_panic]
    fn bucket_length_of_zero_forbidden_when_hashing_from_u64() {
        jump_hash_from_u64(1, 0);
    }

    #[test]
    #[should_panic]
    fn bucket_length_of_zero_forbidden_when_hashing_from_str() {
        jump_hash_from_str("foobar", 0);
    }

    #[test]
    fn bucket_chosen_always_within_range() {
        for num_buckets in 1..500 {
            for i in 0..100_000 {
                let idx = jump_hash_from_str(&i.to_string(), num_buckets);
                assert!(idx < num_buckets)
            }
        }
    }
    #[test]
    fn new_shard_causes_minimal_reshuffling() {
        let num_keys = 10_000;
        for num_buckets in 5..=1000 {
            let mut key_moved = 0;
            for i in 1..num_keys {
                let current_shard = jump_hash_from_str(&i.to_string(), num_buckets);
                let next_shard = jump_hash_from_str(&i.to_string(), num_buckets + 1);

                if current_shard != next_shard {
                    if next_shard != num_buckets  {
                        panic!("if keys move, we'd expect them to move to new shard, not an existing shard")
                    }
                    key_moved += 1;
                }
            }
            //Either we expect the amount of keys moved to the new shard to be consistent with the amount of keys we'd expect to be divided up equally into each shard (with considerable wiggle-room of 15%)
            let keys_moved_as_acceptable_proportion = (key_moved as f64) < ((num_keys as f64) / (num_buckets as f64 ) * 1.15 );
            //or we expect the relative number of keys as compared to the total number of keys to be small
            let keys_moved_as_acceptable_percent_of_total = (key_moved as f64 / num_keys as f64) < 0.02;
            assert!(keys_moved_as_acceptable_proportion || keys_moved_as_acceptable_percent_of_total)
        }
    }

    struct TestCase{
         key: u64,
         bucket: Vec<u32>
    }
    #[test]
    fn matches_reference_code() {
        // test case borrowed from https://github.com/dgryski/go-jump/blob/master/jump_test.go
        let cases = vec!(
            TestCase{key: 1, bucket: vec!(0, 0, 0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 17, 17)},
            TestCase{key: 0xdeadbeef, bucket: vec!(0, 1, 2, 3, 3, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 16, 16, 16)},
            TestCase{key: 0x0ddc0ffeebadf00d, bucket: vec!(0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 15, 15, 15, 15)},
        );
        for case in cases {
            for (i, expected_value)  in case.bucket.iter().enumerate() {
                let result = jump_hash_from_u64(case.key, (i + 1) as u32);
                assert_eq!(&result, expected_value, "test case didn't match")

            }

        }
    }
}