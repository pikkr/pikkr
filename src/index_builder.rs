use super::bit;
use x86intrin::{m256i, mm256_cmpeq_epi8, mm256_movemask_epi8};

#[inline]
pub fn build_structural_character_bitmap(s: &Vec<m256i>, d: &mut Vec<u64>, m: m256i) {
    let n = s.len();
    let mut i = 0;
    while i + 1 < n {
        let i1 = mm256_movemask_epi8(mm256_cmpeq_epi8(s[i], m));
        let i2 = mm256_movemask_epi8(mm256_cmpeq_epi8(s[i+1], m));
        d.push((i1 as u32 as u64) | ((i2 as u32 as u64) << 32));
        i += 2;
    }
    if n & 1 == 1 {
        d.push(mm256_movemask_epi8(mm256_cmpeq_epi8(s[i], m)) as u32 as u64);
    }
}

#[inline]
pub fn build_structural_quote_bitmap(b_backslash: &Vec<u64>, b_quote: &mut Vec<u64>) {
    let mut b_unstructural_quote= Vec::with_capacity(b_quote.len());
    let mut b_backslash_quote = b_quote.clone();
    bit::shift_right_by_one(&mut b_backslash_quote);
    bit::and(&b_backslash, &mut b_backslash_quote);
    for i in 0..b_backslash_quote.len() {
        let mut unstructural_quote = 0u64;
        let mut backslash_quote = b_backslash_quote[i];
        while backslash_quote != 0 {
            let backslash_quote_mask = bit::s(backslash_quote);
            let backslash_quote_mask_ones_num = backslash_quote_mask.count_ones();
            let mut consecutive_backslash_num = 0;
            for j in (0..i+1).rev() {
                let backslash_b = b_backslash[j];
                if j == i {
                    let backslash_b_mask = (backslash_b & backslash_quote_mask) << (64 - backslash_quote_mask_ones_num);
                    let leading_ones_num = (!backslash_b_mask).leading_zeros();
                    consecutive_backslash_num += leading_ones_num;
                    if leading_ones_num != backslash_quote_mask_ones_num {
                        break;
                    }
                } else {
                    let backslash_b_mask = backslash_b & 0xffffffffffffffffu64;
                    let leading_ones_num = (!backslash_b_mask).leading_zeros();
                    consecutive_backslash_num += leading_ones_num;
                    if leading_ones_num != 64 {
                        break;
                    }
                }
            }
            if consecutive_backslash_num & 1 == 1 {
                unstructural_quote |= bit::e(backslash_quote);
            }
            backslash_quote = bit::r(backslash_quote);
        }
        b_unstructural_quote.push(!unstructural_quote);
    }
    bit::shift_left_by_one(&mut b_unstructural_quote);
    bit::and(&b_unstructural_quote, b_quote);
}

#[inline]
pub fn build_string_mask_bitmap(b_quote: &mut Vec<u64>) {
    let mut n = 0;
    for i in 0..b_quote.len() {
        let mut m_quote = b_quote[i];
        let mut m_string = 0u64;
        while m_quote != 0 {
            let m = bit::s(m_quote);
            m_string ^= m;
            m_quote = bit::r(m_quote);
            n += 1;
        }
        if n & 1 == 0 {
            m_string = !m_string;
        }
        b_quote[i] = m_string;
    }
}

#[inline]
pub fn build_leveled_colon_bitmap(b_colon: &Vec<u64>, b_left: &Vec<u64>, b_right: &Vec<u64>, l: usize, b: &mut Vec<Vec<u64>>) {
    for _ in 0..l {
        b.push(b_colon.clone());
    }
    let mut s = Vec::new();
    for i in 0..b_right.len() {
        let mut m_left = b_left[i];
        let mut m_right = b_right[i];
        loop {
            let m_rightbit = bit::e(m_right);
            let mut m_leftbit = bit::e(m_left);
            while m_leftbit != 0 && (m_rightbit == 0 || m_leftbit < m_rightbit) {
                s.push((i, m_leftbit));
                m_left = bit::r(m_left);
                m_leftbit = bit::e(m_left);
            }
            if m_rightbit != 0 {
                let (j, mlb) = s.pop().unwrap();
                m_leftbit = mlb;
                if s.len() > 0 {
                    let upper_l = s.len() - 1;
                    if upper_l < l {
                        if i == j {
                            b[upper_l][i] &= !(m_rightbit.wrapping_sub(m_leftbit));
                        } else {
                            b[upper_l][j] &= m_leftbit.wrapping_sub(1);
                            b[upper_l][i] &= !(m_rightbit.wrapping_sub(1));
                            for k in j+1..i {
                                b[upper_l][k] = 0
                            }
                        }
                    }
                }

            }
            m_right = bit::r(m_right);
            if m_rightbit == 0 {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::avx;
    use super::super::utf8::QUOTE;
    use x86intrin::mm256_setr_epi8;

    #[test]
    fn test_build_structural_character_bitmap() {
        let char = QUOTE as i8;
        let m = avx::mm256i(char);
        struct TestCase {
            s: Vec<m256i>,
            d: Vec<u64>
        }
        #[allow(overflowing_literals)]
        let test_cases = vec![
            TestCase {
                s: vec![],
                d: vec![],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, char, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_01000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, char)],
                d: vec![0b00000000_00000000_00000000_00000000_10000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char)],
                d: vec![0b00000000_00000000_00000000_00000000_11111111_11111111_11111111_11111111],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000001_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000001_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000001_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000001_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000001_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, char)],
                d: vec![0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_11111111_11111111_11111111_11111111],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char)],
                d: vec![0b11111111_11111111_11111111_11111111_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char),
                        mm256_setr_epi8(char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char,
                                        char, char, char, char, char, char, char, char)],
                d: vec![0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000010_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, char),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_10000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000001_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, char, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00100000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, char),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, char, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00000000_00000010_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, char, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_00000000_00001000_00000000_00000000],
            },
            TestCase {
                s: vec![mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff),
                        mm256_setr_epi8(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, char)],
                d: vec![0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                        0b00000000_00000000_00000000_00000000_10000000_00000000_00000000_00000000],
            },
        ];
        for t in test_cases {
            let mut d = Vec::with_capacity((t.s.len() + 1) / 2);
            build_structural_character_bitmap(&t.s, &mut d, m);
            assert_eq!(t.d, d);
        }
    }

    #[test]
    fn test_build_structural_quote_bitmap() {
        struct TestCase {
            b_backslash: Vec<u64>,
            b_quote: Vec<u64>,
            want: Vec<u64>,
        }
        let test_cases = vec![
            TestCase {
                b_backslash: vec![],
                b_quote: vec![],
                want: vec![],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000100_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000110_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000111_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000111_10000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000111_11000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000111_11100000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00001000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000110_00000000_00000000_00000000_00100000_00000000_00000000,
                    0b00000000_00000000_00011110_00000000_00000000_00000000_00001110_00000000
                ],
                b_quote: vec![
                    0b00000000_00001000_00000000_00000000_00100000_01000000_00000000_00000010,
                    0b00000000_00000000_00100000_00000000_00000000_00000000_00010000_00001000
                ],
                want: vec![
                    0b00000000_00001000_00000000_00000000_00100000_00000000_00000000_00000010,
                    0b00000000_00000000_00100000_00000000_00000000_00000000_00000000_00001000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b11100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b11110000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b11100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b10001111_10000001_11111110_00000000_00000000_00000000_00000000_00000000,
                    0b10000000_00000000_00000000_00000111_00011110_00000000_00000000_00000001,
                    0b00000000_00000000_00000000_00000000_00000000_01100000_00100000_00000000
                ],
                b_quote: vec![
                    0b00010000_00000010_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00001000_00100000_00000000_00000000_00000010,
                    0b00000000_00000000_00000000_00000000_00000000_10000000_01000000_00000001
                ],
                want: vec![
                    0b00000000_00000010_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00100000_00000000_00000000_00000010,
                    0b00000000_00000000_00000000_00000000_00000000_10000000_00000000_00000000
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
            },
            TestCase {
                b_backslash: vec![
                    0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            }
        ];
        for t in test_cases {
            let mut b_quote = t.b_quote.clone();
            build_structural_quote_bitmap(&t.b_backslash, &mut b_quote);
            assert_eq!(t.want, b_quote);
        }
    }

    #[test]
    fn test_build_string_mask_bitmap() {
        struct TestCase {
            b_quote: Vec<u64>,
            want: Vec<u64>,
        }
        let test_cases = vec![
            TestCase {
                b_quote: vec![],
                want: vec![],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000100
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_01000000_00000100
                ],
                want: vec![
                    0b11111111_11111111_11111111_11111111_11111111_11111111_10000000_00000111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000010_00000000_00000000_01000000_00000100
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000011_11111111_11111111_10000000_00000111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_01000000_00000000_00000010_00000000_00000000_01000000_00000100
                ],
                want: vec![
                    0b11111111_10000000_00000000_00000011_11111111_11111111_10000000_00000111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111,
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000100,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00010000_00000000_00000000_00000000_00000000_00000000_00010000_00100000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b00011111_11111111_11111111_11111111_11111111_11111111_11100000_00111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00010000_00000000_00000000_00000000_00000000_00000000_00010000_00100000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00001000
                ],
                want: vec![
                    0b00011111_11111111_11111111_11111111_11111111_11111111_11100000_00111111,
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11110000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00010000_00000000_00000000_00000000_00000000_00000000_00010000_00100000,
                    0b00000000_00000000_00000000_00000000_10000000_00000000_00000000_00001000
                ],
                want: vec![
                    0b00011111_11111111_11111111_11111111_11111111_11111111_11100000_00111111,
                    0b00000000_00000000_00000000_00000000_11111111_11111111_11111111_11110000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111,
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111,
                    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_11111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00100000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_11111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b11111111_11111111_11111111_11111111_11111111_11000000_00000000_00000000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00010000_00000000_00000000_00100000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_11111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00000000_00000000_00011111_11111111_11111111_11000000_00000000_00000000
                ],
            },
            TestCase {
                b_quote: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b00001000_00000000_00010000_00000000_00000000_00100000_00000000_00000000
                ],
                want: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_11111111,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                    0b11110000_00000000_00011111_11111111_11111111_11000000_00000000_00000000
                ],
            }
        ];
        for t in test_cases {
            let mut b_quote = t.b_quote.clone();
            build_string_mask_bitmap(&mut b_quote);
            assert_eq!(t.want, b_quote);
        }
    }

    #[test]
    fn test_build_leveled_colon_bitmap() {
        struct TestCase {
            b_colon: Vec<u64>,
            b_left: Vec<u64>,
            b_right: Vec<u64>,
            l: usize,
            want: Vec<Vec<u64>>,
        }
        let test_cases = vec![
            TestCase {
                b_colon: vec![],
                b_left: vec![],
                b_right: vec![],
                l: 1,
                want: vec![vec![]],
            },
            TestCase {
                b_colon: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                b_right: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                l: 1,
                want: vec![vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ]],
            },
            TestCase {
                b_colon: vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_right: vec![
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                l: 1,
                want: vec![vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ]],
            },
            TestCase {
                b_colon: vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001
                ],
                b_right: vec![
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                l: 2,
                want: vec![vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ], vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ]],
            },
            TestCase {
                b_colon: vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00001000_00000000_00000001
                ],
                b_right: vec![
                    0b10000000_00000000_01000000_00000000_00000000_00000000_00000000_00000000
                ],
                l: 1,
                want: vec![vec![
                    0b00000000_00010000_00000000_00000000_00000000_00000000_00000100_00000000
                ]],
            },
            TestCase {
                b_colon: vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00001000_00000000_00000001
                ],
                b_right: vec![
                    0b10000000_00000000_01000000_00000000_00000000_00000000_00000000_00000000
                ],
                l: 2,
                want: vec![vec![
                    0b00000000_00010000_00000000_00000000_00000000_00000000_00000100_00000000
                ], vec![
                    0b00000000_00010000_00000000_00000000_00010000_00000000_00000100_00000000
                ]],
            },
            TestCase {
                b_colon: vec![
                    0b00001000_00001000_00001000_00000000_00000000_00010000_00010000_00010000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000001_00000001_00000001_00000001u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ],
                b_right: vec![
                    0b10000000_10000000_10000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ],
                l: 3,
                want: vec![vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00010000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ], vec![
                    0b00001000_00000000_00000000_00000000_00000000_00000000_00010000_00010000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ], vec![
                    0b00001000_00001000_00000000_00000000_00000000_00010000_00010000_00010000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ]],
            },
            TestCase {
                b_colon: vec![
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ],
                b_left: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ],
                b_right: vec![
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ],
                l: 3,
                want: vec![vec![
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ], vec![
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ], vec![
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                    0b00000000_10000000_00000000_00000000_00000000_00000000_00000000_00000000u64
                ]],
            }
        ];
        for t in test_cases {
            let mut b = Vec::with_capacity(t.l);
            build_leveled_colon_bitmap(&t.b_colon, &t.b_left, &t.b_right, t.l, &mut b);
            assert_eq!(t.want, b);
        }
    }
}
