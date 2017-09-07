use super::bit;
use super::error::{Error, ErrorKind};
use super::query::Query;
use super::result::Result;
use super::utf8::{COMMA, CR, HT, LF, RIGHT_BRACE, SPACE};
use fnv::{FnvHashMap, FnvHashSet};

#[inline]
pub fn basic_parse<'a>(rec: &'a [u8], index: &Vec<Vec<u64>>, queries: &mut FnvHashMap<&[u8], Query>, start: usize, end: usize, level: usize, query_num: usize, stats: &mut Vec<FnvHashSet<usize>>, set_stats: bool, results: &mut Vec<Option<&'a [u8]>>, b_quote: &Vec<u64>) -> Result<()> {
    let mut found_num = 0;
    let cp = generate_colon_positions(index, start, end, level);
    let mut vei = end;
    for i in (0..cp.len()).rev() {
        let (fsi, fei) = match search_pre_field_indices(b_quote, if i > 0 { cp[i - 1] } else { start }, cp[i]) {
            Ok(fsei) => fsei,
            Err(e) => {
                return Err(e);
            }
        };
        let field = &rec[fsi + 1..fei];
        if let Some(query) = queries.get_mut(field) {
            let (vsi, vei) = search_post_value_indices(rec, cp[i] + 1, vei, i == cp.len() - 1);
            found_num += 1;
            if set_stats && !stats[query.i].contains(&i) {
                stats[query.i].insert(i);
            }
            if let Some(ref mut children) = query.children {
                if let Err(e) = basic_parse(
                    rec,
                    index,
                    children,
                    vsi,
                    vei,
                    level + 1,
                    query.children_len,
                    stats,
                    set_stats,
                    results,
                    b_quote,
                ) {
                    return Err(e);
                };
            }
            if query.target {
                results[query.ri] = Some(&rec[vsi..vei + 1]);
            }
            if found_num == query_num {
                return Ok(());
            }
        }
        vei = fsi - 1;
    }
    Ok(())
}

#[inline]
pub fn speculative_parse<'a>(rec: &'a [u8], index: &Vec<Vec<u64>>, queries: &FnvHashMap<&[u8], Query>, start: usize, end: usize, level: usize, stats: &Vec<FnvHashSet<usize>>, results: &mut Vec<Option<&'a [u8]>>, b_quote: &Vec<u64>) -> Result<bool> {
    let cp = generate_colon_positions(index, start, end, level);
    for (s, q) in queries.iter() {
        let mut found = false;
        for i in &stats[q.i] {
            if *i >= cp.len() {
                continue;
            }
            let (fsi, fei) = match search_pre_field_indices(b_quote, if *i > 0 { cp[*i - 1] } else { start }, cp[*i]) {
                Ok(fsei) => fsei,
                Err(e) => {
                    return Err(e);
                }
            };
            let field = &rec[fsi + 1..fei];
            if s == &field {
                let vei = if *i < cp.len() - 1 {
                    let nfsi = match search_pre_field_indices(b_quote, cp[*i], cp[*i + 1]) {
                        Ok((nfsi, _)) => nfsi,
                        Err(e) => {
                            return Err(e);
                        }
                    };
                    nfsi - 1
                } else {
                    end
                };
                let (vsi, vei) = search_post_value_indices(rec, cp[*i] + 1, vei, *i == cp.len() - 1);
                if let Some(ref children) = q.children {
                    found = match speculative_parse(
                        rec,
                        index,
                        children,
                        vsi,
                        vei,
                        level + 1,
                        stats,
                        results,
                        b_quote,
                    ) {
                        Ok(found) => found,
                        Err(e) => return Err(e),
                    };
                } else {
                    found = true;
                }
                if q.target {
                    results[q.ri] = Some(&rec[vsi..vei + 1]);
                }
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    Ok(true)
}

#[inline]
fn generate_colon_positions(index: &Vec<Vec<u64>>, start: usize, end: usize, level: usize) -> Vec<usize> {
    let mut c = Vec::new();
    for i in start / 64..(end + 63) / 64 {
        let mut m_colon = index[level][i];
        while m_colon != 0 {
            let m_bit = bit::e(m_colon);
            let offset = i * 64 + (m_bit.trailing_zeros() as usize);
            if start <= offset && offset <= end {
                c.push(offset);
            }
            m_colon = bit::r(m_colon);
        }
    }
    return c;
}

#[inline]
fn search_pre_field_indices(b_quote: &Vec<u64>, start: usize, end: usize) -> Result<(usize, usize)> {
    let mut si = 0;
    let mut ei = 0;
    let mut ei_set = false;
    let mut n_quote = 0;
    for i in (start / 64..(end + 63) / 64).rev() {
        let mut m_quote = b_quote[i];
        while m_quote != 0 {
            let m_bit = bit::e(m_quote);
            let offset = i * 64 + (m_bit.trailing_zeros() as usize);
            if end <= offset {
                break;
            }
            if start < offset {
                if ei_set {
                    si = offset;
                } else {
                    si = ei;
                    ei = offset;
                }
                n_quote += 1;
            }
            m_quote = bit::r(m_quote);
        }
        if n_quote >= 2 {
            break;
        }
        if n_quote == 1 && !ei_set {
            ei_set = true;
        }
    }
    if n_quote >= 2 {
        Ok((si, ei))
    } else {
        Err(Error::from(ErrorKind::InvalidRecord))
    }
}

#[inline]
fn search_post_value_indices(rec: &[u8], si: usize, ei: usize, last_cp: bool) -> (usize, usize) {
    let mut si = si;
    let mut ei = ei;
    while rec[si] == SPACE || rec[si] == HT || rec[si] == LF || rec[si] == CR {
        si += 1;
    }
    while rec[ei] == SPACE || rec[ei] == HT || rec[ei] == LF || rec[ei] == CR || rec[ei] == COMMA {
        ei -= 1;
    }
    if last_cp && rec[ei] == RIGHT_BRACE {
        ei -= 1;
        while rec[ei] == SPACE || rec[ei] == HT || rec[ei] == LF || rec[ei] == CR {
            ei -= 1;
        }
    }
    (si, ei)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::avx;
    use super::super::index_builder;
    use super::super::utf8::{BACKSLASH, COLON, LEFT_BRACE, QUOTE, RIGHT_BRACE};

    #[test]
    fn test_basic_parse() {
        let json_rec_str = r#"{ "aaa" : "AAA", "bbb" : 111, "ccc": ["C1", "C2"], "ddd" : { "d1" : "D1", "d2" : "D2", "d3": 333 }, "eee": { "e1": "EEE" } } "#;
        let json_rec = json_rec_str.as_bytes();
        let mut b_backslash = Vec::with_capacity((json_rec.len() + 63) / 64);
        let mut b_quote = Vec::with_capacity((json_rec.len() + 63) / 64);
        let mut b_colon = Vec::with_capacity((json_rec.len() + 63) / 64);
        let mut b_left = Vec::with_capacity((json_rec.len() + 63) / 64);
        let mut b_right = Vec::with_capacity((json_rec.len() + 63) / 64);
        index_builder::build_structural_character_bitmap(
            &json_rec,
            &mut b_backslash,
            &mut b_quote,
            &mut b_colon,
            &mut b_left,
            &mut b_right,
            &avx::mm256i(BACKSLASH as i8),
            &avx::mm256i(QUOTE as i8),
            &avx::mm256i(COLON as i8),
            &avx::mm256i(LEFT_BRACE as i8),
            &avx::mm256i(RIGHT_BRACE as i8),
        );
        index_builder::build_structural_quote_bitmap(&b_backslash, &mut b_quote);
        let mut b_string_mask = Vec::with_capacity(b_quote.len());
        index_builder::build_string_mask_bitmap(&b_quote, &mut b_string_mask);
        for i in 0..b_string_mask.len() {
            let b = b_string_mask[i];
            b_colon[i] &= b;
            b_left[i] &= b;
            b_right[i] &= b;
        }
        let l = 10;
        let mut index = Vec::with_capacity(l);
        let r = index_builder::build_leveled_colon_bitmap(&b_colon, &b_left, &b_right, l, &mut index);
        assert_eq!(Ok(()), r);
        let mut children = FnvHashMap::default();
        children.insert(
            "d1".as_bytes(),
            Query {
                i: 0,
                ri: 0,
                target: true,
                children: None,
                children_len: 0,
            },
        );
        children.insert(
            "d3".as_bytes(),
            Query {
                i: 1,
                ri: 1,
                target: true,
                children: None,
                children_len: 0,
            },
        );
        let children_len = children.len();
        let mut queries = FnvHashMap::default();
        queries.insert(
            "aaa".as_bytes(),
            Query {
                i: 2,
                ri: 2,
                target: true,
                children: None,
                children_len: 0,
            },
        );
        queries.insert(
            "bbb".as_bytes(),
            Query {
                i: 3,
                ri: 3,
                target: true,
                children: None,
                children_len: 0,
            },
        );
        queries.insert(
            "ccc".as_bytes(),
            Query {
                i: 4,
                ri: 4,
                target: true,
                children: None,
                children_len: 0,
            },
        );
        queries.insert(
            "ddd".as_bytes(),
            Query {
                i: 5,
                ri: 0,
                target: false,
                children: Some(children),
                children_len: children_len,
            },
        );
        queries.insert(
            "eee".as_bytes(),
            Query {
                i: 6,
                ri: 5,
                target: true,
                children: None,
                children_len: 0,
            },
        );

        let queries_len = queries.len();

        let mut stats = vec![
            FnvHashSet::default(),
            FnvHashSet::default(),
            FnvHashSet::default(),
            FnvHashSet::default(),
            FnvHashSet::default(),
            FnvHashSet::default(),
            FnvHashSet::default(),
        ];

        let mut results = vec![None, None, None, None, None, None];

        let result = basic_parse(
            json_rec,
            &index,
            &mut queries,
            0,
            json_rec.len() - 1,
            0,
            queries_len,
            &mut stats,
            true,
            &mut results,
            &b_quote,
        );

        assert_eq!(Ok(()), result);
        assert_eq!(Some(r#""D1""#.as_bytes()), results[0]);
        assert_eq!(Some(r#"333"#.as_bytes()), results[1]);
        assert_eq!(Some(r#""AAA""#.as_bytes()), results[2]);
        assert_eq!(Some(r#"111"#.as_bytes()), results[3]);
        assert_eq!(Some(r#"["C1", "C2"]"#.as_bytes()), results[4]);
        assert_eq!(Some(r#"{ "e1": "EEE" }"#.as_bytes()), results[5]);
    }

    #[test]
    fn test_generate_colon_positions() {
        struct TestCase {
            index: Vec<Vec<u64>>,
            start: usize,
            end: usize,
            level: usize,
            want: Vec<usize>,
        }
        let test_cases = vec![
            TestCase {
                index: vec![vec![]],
                start: 0,
                end: 0,
                level: 0,
                want: vec![],
            },
            TestCase {
                index: vec![
                    vec![
                        0b00000001_11000000_00000000_00000000_00001000_00000000_00000011_10000000,
                    ],
                ],
                start: 8,
                end: 55,
                level: 0,
                want: vec![8, 9, 27, 54, 55],
            },
            TestCase {
                index: vec![
                    vec![
                        0b10000000_00000000_00000000_00000000_00001000_00000000_00000011_10000000,
                        0b00000001_11000001_00000000_00000011_00000000_00000000_00000000_00000001,
                    ],
                ],
                start: 8,
                end: 119,
                level: 0,
                want: vec![8, 9, 27, 63, 64, 96, 97, 112, 118, 119],
            },
        ];
        for t in test_cases {
            let cp = generate_colon_positions(&t.index, t.start, t.end, t.level);
            assert_eq!(t.want, cp);
        }
    }
}
