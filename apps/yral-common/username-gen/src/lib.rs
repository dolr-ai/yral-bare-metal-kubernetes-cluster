use rand::{
    SeedableRng,
    distr::uniform::{UniformChar, UniformSampler},
    seq::IndexedRandom,
};
use rand_xoshiro::Xoshiro256StarStar;
use sha2::{Digest, Sha256};

use crate::data::{adjectives::ADJECTIVES, nouns::NOUNS};

mod data;

const RAND_DIGIT_COUNT: usize = 3;

/// Generate a deterministic random username from a user identifier string.
/// The user_id (JWT sub or UUID) is hashed with SHA-256 to produce a
/// 32-byte seed for the RNG. This replaces the old IC Principal-based seeding.
pub fn random_username_from_principal(user_id: &str, max_len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    let seed = hasher.finalize();
    let mut rng = Xoshiro256StarStar::from_seed(seed.into());

    let noun = *NOUNS.choose(&mut rng).unwrap();
    let adjective = *ADJECTIVES.choose(&mut rng).unwrap();

    let mut base = String::new();
    base.push_str(noun);
    base.push_str(adjective);
    base.truncate(max_len - RAND_DIGIT_COUNT);

    let digit_dist = UniformChar::new_inclusive('0', '9').unwrap();

    for _ in 0..RAND_DIGIT_COUNT {
        base.push(digit_dist.sample(&mut rng));
    }
    base.shrink_to_fit();

    base
}

#[cfg(test)]
mod test {
    use super::random_username_from_principal;
    use candid::Principal;

    #[test]
    fn test_rng_len() {
        let princ = Principal::anonymous();
        let res = random_username_from_principal(princ, 15);
        println!("{res}");
        assert!(res.len() <= 15);
    }

    #[test]
    fn test_rng_reproducible() {
        let princ = Principal::anonymous();
        let res1 = random_username_from_principal(princ, 15);
        let res2 = random_username_from_principal(princ, 15);
        assert_eq!(res1, res2);
    }
}
