/// The length of a hash as returned by the hash functions implemented in this RFC (in
/// units of binary-coded, balanced trits).
pub const HASH_LEN: usize = 243;

/// The length internal state of the `CurlP` sponge construction (in units of binary-coded,
/// balanced trits).
pub const STATE_LEN: usize = HASH_LEN * 3;
pub const HALF_STATE_LEN: usize = STATE_LEN / 2;

pub const CURLP_81_ROUNDS: usize = 81;
pub const CURLP_27_ROUNDS: usize = 27;

pub(crate) const TRUTH_TABLE: [i8; 11] = [1, 0, -1, 2, 1, -1, 0, 2, -1, 1, 0];
