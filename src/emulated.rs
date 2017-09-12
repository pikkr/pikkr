//! emulate AVX extension for normal CPUs
//!
//! This is likely not as fast as using actual AVX, but will work without intrinsics.
//! It also makes for a neat benchmark.

#[allow(non_camel_case_types)]
pub type m256i = [u64; 4];

#[allow(unused_unsafe)]
pub mod avx {
    use super::m256i;

    pub fn mm256i(x: i8) -> m256i {
        [(x as u8) as u64 * 0x0101_0101_0101_0101_u64; 4]
    }

    fn slice_to_u64(s: &[u8]) -> u64 {
        let mut res = 0_u64;
        for (i, x) in s[0..8].iter().enumerate() {
            res |= (*x as u64) << ((7 - i) * 8);
        }
        res
    }


    pub unsafe fn u8_to_m256i(s: &[u8], i: usize) -> m256i {
        debug_assert!(i + 31 < s.len());
        [slice_to_u64(&s[i      .. i +  8]),
         slice_to_u64(&s[i +  8 .. i + 16]),
         slice_to_u64(&s[i + 16 .. i + 24]),
         slice_to_u64(&s[i + 24 .. i + 32])]
    }

    pub unsafe fn u8_to_m256i_rest(s: &[u8], i: usize) -> m256i {
        let mut result = [0_u64; 4];
        for x in i..s.len() {
            result[(x - i) / 8] |= (s[x] as u64) << ((7 - ((x - i) & 7)) * 8);
        }
        result
    }
}

pub fn mm256_cmpeq_epi8(x: m256i, y: m256i) -> m256i {
    fn bytewise_equal(x: u64, y: u64) -> u64 {
        let lo = ::std::u64::MAX / 0xFF;
        let hi = lo << 7;
        let x = x ^ y;
        !((((x & !hi) + !hi) | x) >> 7) & lo
    }
    [bytewise_equal(x[0], y[0]),
     bytewise_equal(x[1], y[1]),
     bytewise_equal(x[2], y[2]),
     bytewise_equal(x[3], y[3])]
}

pub fn mm256_movemask_epi8(x: m256i) -> u32 {
    let factor = 0x8040_2010_0804_0201_u64;
    ((x[0].wrapping_mul(factor) >> 56) & 0x0000_00FF_u64 |
     (x[1].wrapping_mul(factor) >> 48) & 0x0000_FF00_u64 |
     (x[2].wrapping_mul(factor) >> 40) & 0x00FF_0000_u64 |
     (x[3].wrapping_mul(factor) >> 32) & 0xFF00_0000_u64) as u32
}

