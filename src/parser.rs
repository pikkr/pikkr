use super::bit;
use super::error::{Error, ErrorKind};
use super::index_builder::IndexBuilder;
use super::query::Query;
use super::result::Result;
use super::utf8::{COMMA, CR, HT, LF, RIGHT_BRACE, SPACE};
use fnv::{FnvHashMap, FnvHashSet};
use std::cell::RefCell;

pub struct Parser {
    pub index_builder: IndexBuilder,
    stats: Vec<FnvHashSet<usize>>,
    colon_positions: RefCell<Vec<Vec<usize>>>,
}

impl Parser {
    pub fn new(max_depth: usize, num_nodes: usize) -> Self {
        let index_builder = IndexBuilder::new(max_depth);
        let colon_positions = RefCell::new(vec![Vec::new(); max_depth]);
        let stats = vec![Default::default(); num_nodes];
        Self {
            index_builder,
            stats,
            colon_positions,
        }
    }
}

#[inline]
pub fn basic_parse<'a>(
    parser: &mut Parser,
    rec: &'a [u8],
    queries: &FnvHashMap<&[u8], Query>,
    start: usize,
    end: usize,
    level: usize,
    set_stats: bool,
    results: &mut Vec<Option<&'a [u8]>>,
) -> Result<()> {
    generate_colon_positions(
        &parser.index_builder.index,
        start,
        end,
        level,
        &mut *parser.colon_positions.borrow_mut(),
    );

    let mut found_num = 0;
    let mut vei = end;
    let cp_len = parser.colon_positions.borrow()[level].len();
    for i in (0..cp_len).rev() {
        let (fsi, fei) = search_pre_field_indices(
            &parser.index_builder.b_quote,
            if i > 0 {
                parser.colon_positions.borrow()[level][i - 1]
            } else {
                start
            },
            parser.colon_positions.borrow()[level][i],
        )?;
        let field = &rec[fsi + 1..fei];
        if let Some(query) = queries.get(field) {
            let (vsi, vei) = search_post_value_indices(
                rec,
                parser.colon_positions.borrow()[level][i] + 1,
                vei,
                if i == cp_len - 1 { RIGHT_BRACE } else { COMMA },
            )?;
            found_num += 1;
            if set_stats && !parser.stats[query.i].contains(&i) {
                parser.stats[query.i].insert(i);
            }
            if let Some(ref children) = query.children {
                basic_parse(
                    parser,
                    rec,
                    children,
                    vsi,
                    vei,
                    level + 1,
                    set_stats,
                    results,
                )?;
            }
            if query.target {
                results[query.ri] = Some(&rec[vsi..vei + 1]);
            }
            if found_num == queries.len() {
                return Ok(());
            }
        }
        vei = fsi - 1;
    }
    Ok(())
}

#[inline]
pub fn speculative_parse<'a>(parser: &Parser, rec: &'a [u8], queries: &FnvHashMap<&[u8], Query>, start: usize, end: usize, level: usize, results: &mut Vec<Option<&'a [u8]>>) -> Result<bool> {
    generate_colon_positions(
        &parser.index_builder.index,
        start,
        end,
        level,
        &mut *parser.colon_positions.borrow_mut(),
    );

    for (&s, q) in queries {
        let mut found = false;
        for &i in &parser.stats[q.i] {
            let cp_len = parser.colon_positions.borrow()[level].len();
            if i >= cp_len {
                continue;
            }
            let (fsi, fei) = search_pre_field_indices(
                &parser.index_builder.b_quote,
                if i > 0 {
                    parser.colon_positions.borrow()[level][i - 1]
                } else {
                    start
                },
                parser.colon_positions.borrow()[level][i],
            )?;
            let field = &rec[fsi + 1..fei];
            if s == field {
                let vei = if i < cp_len - 1 {
                    let (nfsi, _) = search_pre_field_indices(
                        &parser.index_builder.b_quote,
                        parser.colon_positions.borrow()[level][i],
                        parser.colon_positions.borrow()[level][i + 1],
                    )?;
                    nfsi - 1
                } else {
                    end
                };
                let (vsi, vei) = search_post_value_indices(
                    rec,
                    parser.colon_positions.borrow()[level][i] + 1,
                    vei,
                    if i == cp_len - 1 { RIGHT_BRACE } else { COMMA },
                )?;
                if let Some(ref children) = q.children {
                    found = speculative_parse(parser, rec, children, vsi, vei, level + 1, results)?;
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
fn generate_colon_positions(index: &[Vec<u64>], start: usize, end: usize, level: usize, colon_positions: &mut Vec<Vec<usize>>) {
    let cp = &mut colon_positions[level];
    cp.clear();
    for i in start / 64..(end + 63) / 64 {
        let mut m_colon = index[level][i];
        while m_colon != 0 {
            let m_bit = bit::e(m_colon);
            let offset = i * 64 + (m_bit.trailing_zeros() as usize);
            if start <= offset && offset <= end {
                cp.push(offset);
            }
            m_colon = bit::r(m_colon);
        }
    }
}

#[inline]
fn search_pre_field_indices(b_quote: &[u64], start: usize, end: usize) -> Result<(usize, usize)> {
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
fn search_post_value_indices(rec: &[u8], si: usize, ei: usize, ignore_once_char: u8) -> Result<(usize, usize)> {
    let mut si = si;
    let mut ei = ei;
    let mut ignore_once_char_ignored = false;
    let n = rec.len();
    while si < n {
        match rec[si] {
            SPACE | HT | LF | CR => {
                si += 1;
            }
            _ => {
                break;
            }
        }
    }
    if si == n {
        return Err(Error::from(ErrorKind::InvalidRecord));
    }
    while si <= ei {
        match rec[ei] {
            SPACE | HT | LF | CR => {
                ei -= 1;
            }
            char if char == ignore_once_char => {
                if ignore_once_char_ignored {
                    break;
                }
                ignore_once_char_ignored = true;
                ei -= 1;
            }
            _ => {
                break;
            }
        }
    }
    if ei < si {
        return Err(Error::from(ErrorKind::InvalidRecord));
    }
    Ok((si, ei))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parse() {
        let json_rec_str = r#"{ "aaa" : "AAA", "bbb" : 111, "ccc": ["C1", "C2"], "ddd" : { "d1" : "D1", "d2" : "D2", "d3": 333 }, "eee": { "e1": "EEE" } } "#;
        let json_rec = json_rec_str.as_bytes();

        let l = 10;
        let mut parser = Parser::new(l, 7);
        let r = parser.index_builder.build_structural_indices(json_rec);
        assert_eq!(Ok(()), r);

        let mut children = FnvHashMap::default();
        children.insert(
            "d1".as_bytes(),
            Query {
                i: 0,
                ri: 0,
                target: true,
                children: None,
            },
        );
        children.insert(
            "d3".as_bytes(),
            Query {
                i: 1,
                ri: 1,
                target: true,
                children: None,
            },
        );
        let mut queries = FnvHashMap::default();
        queries.insert(
            "aaa".as_bytes(),
            Query {
                i: 2,
                ri: 2,
                target: true,
                children: None,
            },
        );
        queries.insert(
            "bbb".as_bytes(),
            Query {
                i: 3,
                ri: 3,
                target: true,
                children: None,
            },
        );
        queries.insert(
            "ccc".as_bytes(),
            Query {
                i: 4,
                ri: 4,
                target: true,
                children: None,
            },
        );
        queries.insert(
            "ddd".as_bytes(),
            Query {
                i: 5,
                ri: 0,
                target: false,
                children: Some(children),
            },
        );
        queries.insert(
            "eee".as_bytes(),
            Query {
                i: 6,
                ri: 5,
                target: true,
                children: None,
            },
        );
        let mut results = vec![None, None, None, None, None, None];
        let result = basic_parse(
            &mut parser,
            json_rec,
            &mut queries,
            0,
            json_rec.len() - 1,
            0,
            true,
            &mut results,
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
            let mut cp = vec![Vec::new(); t.level + 1];
            generate_colon_positions(&t.index, t.start, t.end, t.level, &mut cp);
            assert_eq!(t.want, cp[0]);
        }
    }
}
