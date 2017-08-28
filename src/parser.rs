use super::bit;
use super::stat::Stat;
use super::query::Query;
use super::utf8::{BACKSLASH, COMMA, CR, HT, LF, QUOTE, SPACE, RIGHT_BRACE};
use fnv::FnvHashMap;

#[inline]
pub fn basic_parse(rec: &[u8], index: &Vec<Vec<u64>>, queries: &mut FnvHashMap<&[u8], Query>, start: usize, end: usize, level: usize, query_num: usize, found_num: usize) -> usize {
    let mut found_num = found_num;
    let cp = generate_colon_positions(index, start, end, level);
    let mut vei = end;
    for i in (0..cp.len()).rev() {
        let (fsi, fei) = search_pre_field_indices(rec, cp[i]);
        let field = rec.get(fsi+1..fei).unwrap();
        if let Some(query) = queries.get_mut(field) {
            let (vsi, vei) = search_post_value_indices(rec, cp[i]+1, vei, i == cp.len()-1);
            query.result = Some((vsi, vei+1, i));
            found_num += 1;
            if let Some(ref mut children) = query.children {
                found_num = basic_parse(rec, index, children, vsi, vei, level+1, query_num, found_num);
            }
            if found_num == query_num {
                return found_num;
            }
        }
        vei = fsi - 1;
    }
    return found_num;
}

#[inline]
pub fn speculative_parse(rec: &[u8], index: &Vec<Vec<u64>>, queries: &mut FnvHashMap<&[u8], Query>, start: usize, end: usize, level: usize, stats: &FnvHashMap<&[u8], Stat>) -> bool {
    let cp = generate_colon_positions(index, start, end, level);
    for (s, q) in queries.iter_mut() {
        let mut found = false;
        if let Some(ref st) = stats.get(s) {
            for i in &st.locs {
                if *i >= cp.len() {
                    continue;
                }
                let (fsi, fei) = search_pre_field_indices(rec, cp[*i]);
                let field = rec.get(fsi+1..fei).unwrap();
                if s == &field {
                    let vei = if *i < cp.len()-1 {
                        let (nfsi, _) = search_pre_field_indices(rec, cp[*i+1]);
                        nfsi - 1
                    } else {
                        end
                    };
                    let (vsi, vei) = search_post_value_indices(rec, cp[*i]+1, vei, *i == cp.len()-1);
                    q.result = Some((vsi, vei+1, *i));
                    if let Some(ref mut children) = q.children {
                        if let Some(ref st_children) = st.children {
                            found = speculative_parse(rec, index, children, vsi, vei, level+1, st_children);
                        }
                    } else {
                        found = true;
                    }
                    break;
                }
            }
        }
        if !found {
            return false;
        }
    }
    true
}

#[inline]
fn generate_colon_positions(index: &Vec<Vec<u64>>, start: usize, end: usize, level: usize) -> Vec<usize> {
    let mut c = Vec::new();
    for i in start/64..(end+63)/64 {
        let mut m_colon = index[level][i];
        while m_colon != 0 {
            let m_bit = bit::e(m_colon);
            let offset = i * 64 + (m_bit.wrapping_sub(1).count_ones() as usize);
            if start <= offset && offset <= end {
                c.push(offset);
            }
            m_colon = bit::r(m_colon);
        }
    }
    return c;
}

#[inline]
fn search_pre_field_indices(rec: &[u8], cp: usize) -> (usize, usize) {
    let mut eq = false;
    let mut ei = 0;
    let mut sq = false;
    let mut si = 0;
    let mut sqbn = 0;
    for i in (0..cp).rev() {
        if sq {
            if rec[i] == BACKSLASH {
                sqbn += 1;
                continue;
            }
            if sqbn & 1 == 0 {
                break;
            }
            sq = false;
            si = 0;
            sqbn = 0;
        }
        if rec[i] == QUOTE {
            if !eq {
                eq = true;
                ei = i;
                continue;
            }
            sq = true;
            si = i;
            sqbn = 0;
        }
    }
    (si, ei)
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
    use super::super::bit;
    use super::super::index_builder;
    use super::super::utf8::{BACKSLASH, COLON, LEFT_BRACE, QUOTE, RIGHT_BRACE};

    #[test]
    fn test_basic_parse() {
        let json_rec_str = r#"{ "aaa" : "AAA", "bbb" : 111, "ccc": ["C1", "C2"], "ddd" : { "d1" : "D1", "d2" : "D2", "d3": 333 }, "eee": { "e1": "EEE" } } "#;
        let json_rec = json_rec_str.as_bytes();
        let mut s = Vec::with_capacity((json_rec.len() + 31) / 32);
        avx::u8_to_m256i(&json_rec,&mut s);
        let mut b_backslash = Vec::with_capacity((s.len() + 1) / 2);
        let mut b_quote = Vec::with_capacity((s.len() + 1) / 2);
        let mut b_colon = Vec::with_capacity((s.len() + 1) / 2);
        let mut b_left = Vec::with_capacity((s.len() + 1) / 2);
        let mut b_right = Vec::with_capacity((s.len() + 1) / 2);
        index_builder::build_structural_character_bitmap(&s, &mut b_backslash, avx::mm256i(BACKSLASH as i8));
        index_builder::build_structural_character_bitmap(&s, &mut b_quote, avx::mm256i(QUOTE as i8));
        index_builder::build_structural_character_bitmap(&s, &mut b_colon, avx::mm256i(COLON as i8));
        index_builder::build_structural_character_bitmap(&s, &mut b_left, avx::mm256i(LEFT_BRACE as i8));
        index_builder::build_structural_character_bitmap(&s, &mut b_right, avx::mm256i(RIGHT_BRACE as i8));
        index_builder::build_structural_quote_bitmap(&b_backslash, &mut b_quote);
        index_builder::build_string_mask_bitmap(&mut b_quote);
        let b_string_mask= b_quote;
        bit::and(&b_string_mask, &mut b_colon);
        bit::and(&b_string_mask, &mut b_left);
        bit::and(&b_string_mask, &mut b_right);
        let l = 10;
        let mut index = Vec::with_capacity(l);
        index_builder::build_leveled_colon_bitmap(&b_colon, &b_left, &b_right, l, &mut index);
        let mut children = FnvHashMap::default();
        children.insert("d1".as_bytes(), Query {
            result: None,
            children: None,
        });
        children.insert("d3".as_bytes(), Query {
            result: None,
            children: None,
        });
        let mut queries = FnvHashMap::default();
        queries.insert("aaa".as_bytes(), Query {
            result: None,
            children: None,
        });
        queries.insert("bbb".as_bytes(), Query {
            result: None,
            children: None,
        });
        queries.insert("ccc".as_bytes(), Query {
            result: None,
            children: None,
        });
        queries.insert("ddd".as_bytes(), Query {
            result: None,
            children: Some(children),
        });
        queries.insert("eee".as_bytes(), Query {
            result: None,
            children: None,
        });

        basic_parse(json_rec, &index, &mut queries, 0, json_rec.len()-1, 0, 7, 0);

        assert_eq!(Some((10, 15, 0)), queries.get("aaa".as_bytes()).unwrap().result);
        assert_eq!(Some((25, 28, 1)), queries.get("bbb".as_bytes()).unwrap().result);
        assert_eq!(Some((37, 49, 2)), queries.get("ccc".as_bytes()).unwrap().result);
        assert_eq!(Some((59, 98, 3)), queries.get("ddd".as_bytes()).unwrap().result);
        match queries.get("ddd".as_bytes()) {
            Some(ref query) => {
                match query.children {
                    Some(ref children) => {
                        match children.get("d1".as_bytes()) {
                            Some(ref child_query) => {
                                assert_eq!(Some((68, 72, 0)), child_query.result);
                            },
                            _ => (),
                        }
                        match children.get("d3".as_bytes()) {
                            Some(ref child_query) => {
                                assert_eq!(Some((93, 96, 2)), child_query.result);
                            },
                            _ => (),
                        }

                    },
                    _ => (),
                }
            },
            _ => (),
        }
        assert_eq!(Some((107, 122, 4)), queries.get("eee".as_bytes()).unwrap().result);
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
                index: vec![vec![
                    0b00000001_11000000_00000000_00000000_00001000_00000000_00000011_10000000
                ]],
                start: 8,
                end: 55,
                level: 0,
                want: vec![8, 9, 27, 54, 55],
            },
            TestCase {
                index: vec![vec![
                    0b10000000_00000000_00000000_00000000_00001000_00000000_00000011_10000000,
                    0b00000001_11000001_00000000_00000011_00000000_00000000_00000000_00000001
                ]],
                start: 8,
                end: 119,
                level: 0,
                want: vec![8, 9, 27, 63, 64, 96, 97, 112, 118, 119],
            }
        ];
        for t in test_cases {
            let cp = generate_colon_positions(&t.index, t.start, t.end, t.level);
            assert_eq!(t.want, cp);
        }
    }
}
