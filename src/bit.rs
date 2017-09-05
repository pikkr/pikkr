#[inline]
pub fn r(x: u64) -> u64 {
    x & x.wrapping_sub(1)
}

#[inline]
pub fn e(x: u64) -> u64 {
    x & x.wrapping_neg()
}

#[inline]
pub fn s(x: u64) -> u64 {
    x ^ x.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r() {
        struct TestCase {
            x: u64,
            want: u64,
        }

        let test_cases = vec![
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000001_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000001_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000001_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000001_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000001_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000011u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010u64,
            },
            TestCase {
                x:    0b10000000_00100000_00000000_00000000_00000000_00000000_00000100_00000000u64,
                want: 0b10000000_00100000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
             TestCase {
                x:    0b00000000_00010000_00110010_00000000_00000000_00001100_00000000_00000000u64,
                want: 0b00000000_00010000_00110010_00000000_00000000_00001000_00000000_00000000u64,
            },
             TestCase {
                x:    0b11100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
             TestCase {
                x:    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111u64,
                want: 0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111110u64,
            },
        ];

        for test_case in test_cases {
            assert_eq!(test_case.want, r(test_case.x));
        }
    }

    #[test]
    fn test_e() {
        struct TestCase {
            x: u64,
            want: u64,
        }

        let test_cases = vec![
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000001_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000001_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000001_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000001_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000001_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000001_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000001_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000001_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000001_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000001_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000011u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
            },
            TestCase {
                x:    0b10000000_00100000_00000000_00000000_00000000_00000000_00000100_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000100_00000000u64,
            },
            TestCase {
                x:    0b00000000_00010000_00110010_00000000_00000000_00001100_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000100_00000000_00000000u64,
            },
            TestCase {
                x:    0b11100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
            },
        ];

        for test_case in test_cases {
            assert_eq!(test_case.want, e(test_case.x));
        }
    }

    #[test]
    fn test_s() {
        struct TestCase {
            x: u64,
            want: u64,
        }

        let test_cases = vec![
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000010u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000011u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000001_11111111u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000001_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000001_11111111_11111111u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000001_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000001_11111111_11111111_11111111u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000001_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000001_11111111_11111111_11111111_11111111u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000001_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000000_00000001_11111111_11111111_11111111_11111111_11111111u64,
            },
            TestCase {
                x:    0b00000000_00000001_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000000_00000001_11111111_11111111_11111111_11111111_11111111_11111111u64,
            },
            TestCase {
                x:    0b00000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00000001_11111111_11111111_11111111_11111111_11111111_11111111_11111111u64,
            },
            TestCase {
                x:    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000011u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
            },
            TestCase {
                x:    0b10000000_00100000_00000000_00000000_00000000_00000000_00000100_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000111_11111111u64,
            },
            TestCase {
                x:    0b00000000_00010000_00110010_00000000_00000000_00001100_00000000_00000000u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000111_11111111_11111111u64,
            },
            TestCase {
                x:    0b11100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000u64,
                want: 0b00111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111u64,
            },
            TestCase {
                x:    0b11111111_11111111_11111111_11111111_11111111_11111111_11111111_11111111u64,
                want: 0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001u64,
            },
        ];

        for test_case in test_cases {
            assert_eq!(test_case.want, s(test_case.x));
        }
    }
}
