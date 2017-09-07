use x86intrin::{m256i, mm256_setr_epi8};

#[inline]
pub fn mm256i(i: i8) -> m256i {
    mm256_setr_epi8(
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
        i,
    )
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mm256i() {
        let test_cases = vec![0, 1, 2, 3];
        for i in test_cases {
            let want = mm256_setr_epi8(
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
            );
            let got = mm256i(i);
            assert_eq!(want.as_u8x32().as_array(), got.as_u8x32().as_array());
        }
    }
}
