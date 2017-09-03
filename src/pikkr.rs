use super::avx;
use super::bit;
use super::index_builder;
use super::parser;
use super::query::Query;
use super::stat::Stat;
use super::utf8::{BACKSLASH, COLON, DOT, LEFT_BRACE, QUOTE, RIGHT_BRACE};
use std::cmp;
use fnv::{FnvHashMap, FnvHashSet};
use x86intrin::m256i;

const ROOT_QUERY_STR_OFFSET: usize = 2;

/// JSON parser which picks up values directly without performing tokenization
pub struct Pikkr<'a> {
    backslash: m256i,
    quote: m256i,
    colon: m256i,
    left_brace: m256i,
    right_brace: m256i,

    query_strs: &'a Vec<&'a[u8]>,
    query_strs_len: usize,
    queries: FnvHashMap<&'a[u8], Query<'a>>,
    query_num: usize,
    level: usize,

    train_num: usize,
    trained_num: usize,
    trained: bool,

    stats: FnvHashMap<&'a[u8], Stat<'a>>,
}

impl<'a> Pikkr<'a> {
    /// Creates a JSON parser and returns it.
    #[inline]
    pub fn new(query_strs: &'a Vec<&'a[u8]>, train_num: usize) -> Pikkr<'a> {
        let mut p = Pikkr {
            backslash: avx::mm256i(BACKSLASH as i8),
            quote: avx::mm256i(QUOTE as i8),
            colon: avx::mm256i(COLON as i8),
            left_brace: avx::mm256i(LEFT_BRACE as i8),
            right_brace: avx::mm256i(RIGHT_BRACE as i8),

            query_strs: query_strs,
            query_strs_len: query_strs.len(),
            queries: FnvHashMap::default(),
            query_num: 0,
            level: 0,

            train_num: train_num,
            trained_num: 0,
            trained: false,

            stats: FnvHashMap::default(),
        };

        let mut level = 0;
        for query_str in query_strs {
            let query_num = set_queries(&mut p.queries, query_str, ROOT_QUERY_STR_OFFSET);
            p.query_num += query_num;
            level = cmp::max(level, query_num);
        }
        p.level = level;

        p
    }

    /// Parses a JSON record and returns the result.
    #[inline]
    pub fn parse<'b>(&mut self, rec: &'b[u8]) -> Vec<Option<&'b[u8]>> {
        let rec_len = rec.len();

        let rec_m256i_len = (rec_len + 31) / 32;
        let mut rec_m256i = Vec::with_capacity(rec_m256i_len);
        avx::u8_to_m256i(rec, &mut rec_m256i);

        let b_len = (rec_m256i_len + 1) / 2;
        let mut b_backslash = Vec::with_capacity(b_len);
        index_builder::build_structural_character_bitmap(&rec_m256i, &mut b_backslash, self.backslash);
        let mut b_quote = Vec::with_capacity(b_len);
        index_builder::build_structural_character_bitmap(&rec_m256i, &mut b_quote, self.quote);
        let mut b_colon = Vec::with_capacity(b_len);
        index_builder::build_structural_character_bitmap(&rec_m256i, &mut b_colon, self.colon);
        let mut b_left = Vec::with_capacity(b_len);
        index_builder::build_structural_character_bitmap(&rec_m256i, &mut b_left, self.left_brace);
        let mut b_right = Vec::with_capacity(b_len);
        index_builder::build_structural_character_bitmap(&rec_m256i, &mut b_right, self.right_brace);

        index_builder::build_structural_quote_bitmap(&b_backslash, &mut b_quote);

        index_builder::build_string_mask_bitmap(&mut b_quote);
        let b_string_mask = b_quote;

        bit::and(&b_string_mask, &mut b_colon);
        bit::and(&b_string_mask, &mut b_left);
        bit::and(&b_string_mask, &mut b_right);

        let mut index = Vec::with_capacity(self.level);
        index_builder::build_leveled_colon_bitmap(&b_colon, &b_left, &b_right, self.level, &mut index);

        clear_query_results(&mut self.queries);

        if self.trained {
            if !parser::speculative_parse(rec, &index, &mut self.queries, 0, rec_len-1, 0, &self.stats) {
                parser::basic_parse(rec, &index, &mut self.queries, 0, rec_len-1, 0, self.query_num, 0);
            }
        } else {
            parser::basic_parse(rec, &index, &mut self.queries, 0, rec_len-1, 0, self.query_num, 0);
            set_stats(&self.queries, &mut self.stats);
            self.trained_num += 1;
            if self.trained_num >= self.train_num {
                self.trained = true;
            }
        }

        let mut results = Vec::with_capacity(self.query_strs_len);
        for query_str in self.query_strs {
            set_result(rec, &self.queries, query_str, &mut results, ROOT_QUERY_STR_OFFSET);
        }

        results
    }
}

#[inline]
fn set_queries<'a>(queries: &mut FnvHashMap<&'a[u8], Query<'a>>, s: &'a[u8], i: usize) -> usize {
    for j in i..s.len() {
        if s[j] == DOT {
            let t = s.get(i..j).unwrap();
            let query = queries.entry(t).or_insert(Query {
                result: None,
                children: None,
            });
            let mut children = query.children.get_or_insert(FnvHashMap::default());
            return set_queries(&mut children, s, j+1) + 1;
        }
    }
    let t = s.get(i..s.len()).unwrap();
    if !queries.contains_key(t) {
        queries.insert(t, Query {
            result: None,
            children: None,
        });
        return 1;
    }
    0
}

#[inline]
fn clear_query_results(queries: &mut FnvHashMap<&[u8], Query>) {
    for (_, q) in queries.iter_mut() {
        q.result = None;
        if let Some(ref mut children) = q.children {
            clear_query_results(children);
        }
    }
}

#[inline]
fn set_stats<'a>(queries: &FnvHashMap<&'a[u8], Query<'a>>, stats: &mut FnvHashMap<&'a[u8], Stat<'a>>) {
    for (s, q) in queries.iter() {
        if let Some(result) = q.result {
            let st = stats.entry(s).or_insert(Stat {
                locs: FnvHashSet::default(),
                children: None,
            });
            st.locs.insert(result.2);
            if let Some(ref children) = q.children {
                let st_children = st.children.get_or_insert(FnvHashMap::default());
                set_stats(&children, st_children);
            }
        }
    }
}

#[inline]
fn set_result<'a>(rec: &'a[u8], queries: &FnvHashMap<&[u8], Query>, s: &[u8], d: &mut Vec<Option<&'a[u8]>>, i: usize) {
    for j in i..s.len() {
        if s[j] == DOT {
            let t = s.get(i..j).unwrap();
            match queries.get(t) {
                Some(query) => {
                    match query.children {
                        Some(ref children) => set_result(rec, children, s, d, j+1),
                        _ => d.push(None)
                    }
                },
                _ => d.push(None)
            }
            return;
        }
    }
    let t = s.get(i..s.len()).unwrap();
    d.push(match queries.get(t) {
        Some(query) => {
            match query.result {
                Some(result) => {
                    Some(rec.get(result.0..result.1).unwrap())
                },
                _ => None,
            }
        },
        _ => None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pikkr_basic_parse() {
        let queries = vec![
            "$.f1".as_bytes(),
            "$.f2".as_bytes(),
            "$.f2.f1".as_bytes(),
            "$.f2.f2.f1".as_bytes(),
            "$.f2.f3".as_bytes(),
            "$.f3".as_bytes(),
            "$.f4".as_bytes(),
        ];
        let mut p = Pikkr::new(&queries, 1000000000);
        struct TestCase<'a> {
            rec: &'a str,
            want: Vec<Option<&'a[u8]>>,
        }
        let test_cases = vec![
            TestCase {
                rec: r#"{}"#,
                want: vec![None, None, None, None, None, None, None],
            },
            TestCase {
                rec: r#"{"f0": "a"}"#,
                want: vec![None, None, None, None, None, None, None],
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b"}"#,
                want: vec![Some(r#""b""#.as_bytes()), None, None, None, None, None, None],
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#,
                want: vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    None,
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                    None,
                ]
            },
            TestCase {
                rec: r#"{"f1": "Português do Brasil,Català,Deutsch,Español,Français,Bahasa,Italiano,עִבְרִית,日本語,한국어,Română,中文（简体）,中文（繁體）,Українська,Ўзбекча,Türkçe"}"#,
                want: vec![Some(r#""Português do Brasil,Català,Deutsch,Español,Français,Bahasa,Italiano,עִבְרִית,日本語,한국어,Română,中文（简体）,中文（繁體）,Українська,Ўзбекча,Türkçe""#.as_bytes()), None, None, None, None, None, None],
            },
            TestCase {
                rec: r#"{"f1": "\"f1\": \\"}"#,
                want: vec![Some(r#""\"f1\": \\""#.as_bytes()), None, None, None, None, None, None],
            },
            TestCase {
                rec: r#"
                    	{
                     	"f1" 	 : 	 "b"
                    }
                "#,
                want: vec![Some(r#""b""#.as_bytes()), None, None, None, None, None, None],
            },
            // for issue #10
            TestCase {
                rec: r#""#,
                want: vec![None, None, None, None, None],
            },

        ];
        for t in test_cases {
            let got = p.parse(t.rec.as_bytes());
            assert_eq!(t.want, got);
        }
    }

    #[test]
    fn test_pikkr_speculative_parse() {
        let queries = vec![
            "$.f1".as_bytes(),
            "$.f2".as_bytes(),
            "$.f2.f1".as_bytes(),
            "$.f2.f2.f1".as_bytes(),
            "$.f3".as_bytes(),
        ];
        let mut p = Pikkr::new(&queries, 1);
        struct TestCase<'a> {
            rec: &'a str,
            want: Vec<Option<&'a[u8]>>,
        }
        let test_cases = vec![
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#,
                want: vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                ]
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#,
                want: vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                ]
            },
            TestCase {
                rec: r#"{"f1": "b", "f0": "a", "f3": [1, 2, 3], "f2": {"f2": {"f2": "d", "f1": "c"}, "f1": 1}}"#,
                want: vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f2": {"f2": "d", "f1": "c"}, "f1": 1}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                ]
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}}"#,
                want: vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    None,
                ]
            },
            TestCase {
                rec: r#"{}"#,
                want: vec![None, None, None, None, None],
            },
            // for issue #10
            TestCase {
                rec: r#""#,
                want: vec![None, None, None, None, None],
            },
        ];
        for t in test_cases {
            let got = p.parse(t.rec.as_bytes());
            assert_eq!(t.want, got);
        }
    }
}
