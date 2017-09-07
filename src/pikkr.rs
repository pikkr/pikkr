use super::avx;
use super::error::{Error, ErrorKind};
use super::index_builder;
use super::parser;
use super::query::Query;
use super::result::Result;
use super::utf8::{BACKSLASH, COLON, DOLLAR, DOT, LEFT_BRACE, QUOTE, RIGHT_BRACE};
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

    query_strs_len: usize,
    queries: FnvHashMap<&'a [u8], Query<'a>>,
    queries_len: usize,
    level: usize,

    train_num: usize,
    trained_num: usize,
    trained: bool,

    stats: Vec<FnvHashSet<usize>>,
}

impl<'a> Pikkr<'a> {
    /// Creates a JSON parser and returns it.
    #[inline]
    pub fn new<S: ?Sized + AsRef<[u8]>>(query_strs: &[&'a S], train_num: usize) -> Result<Pikkr<'a>> {
        if query_strs.iter().any(|s| !is_valid_query_str(s.as_ref())) {
            return Err(Error::from(ErrorKind::InvalidQuery));
        }

        let mut p = Pikkr {
            backslash: avx::mm256i(BACKSLASH as i8),
            quote: avx::mm256i(QUOTE as i8),
            colon: avx::mm256i(COLON as i8),
            left_brace: avx::mm256i(LEFT_BRACE as i8),
            right_brace: avx::mm256i(RIGHT_BRACE as i8),

            query_strs_len: query_strs.len(),
            queries: FnvHashMap::default(),
            queries_len: 0,
            level: 0,

            train_num: train_num,
            trained_num: 0,
            trained: false,

            stats: Vec::new(),
        };

        let mut qi = 0;
        for (ri, query_str) in query_strs.iter().enumerate() {
            let (level, next_qi) = set_queries(
                &mut p.queries,
                (*query_str).as_ref(),
                ROOT_QUERY_STR_OFFSET,
                qi,
                ri,
            );
            p.level = cmp::max(p.level, level);
            qi = next_qi;
        }

        p.queries_len = p.queries.len();

        for _ in 0..qi {
            p.stats.push(FnvHashSet::default());
        }

        Ok(p)
    }

    /// Parses a JSON record and returns the result.
    #[inline]
    pub fn parse<'b, S: ?Sized + AsRef<[u8]>>(&mut self, rec: &'b S) -> Result<Vec<Option<&'b [u8]>>> {
        let rec = rec.as_ref();

        let rec_len = rec.len();
        if rec_len == 0 {
            return Err(Error::from(ErrorKind::InvalidRecord));
        }

        let rec_m256i_len = (rec_len + 31) / 32;
        let mut rec_m256i = Vec::with_capacity(rec_m256i_len);
        avx::u8_to_m256i(rec, &mut rec_m256i);

        let b_len = (rec_m256i_len + 1) / 2;
        let mut b_backslash = Vec::with_capacity(b_len);
        let mut b_quote = Vec::with_capacity(b_len);
        let mut b_colon = Vec::with_capacity(b_len);
        let mut b_left = Vec::with_capacity(b_len);
        let mut b_right = Vec::with_capacity(b_len);

        index_builder::build_structural_character_bitmap(
            &rec_m256i,
            &mut b_backslash,
            &mut b_quote,
            &mut b_colon,
            &mut b_left,
            &mut b_right,
            &self.backslash,
            &self.quote,
            &self.colon,
            &self.left_brace,
            &self.right_brace,
        );

        index_builder::build_structural_quote_bitmap(&b_backslash, &mut b_quote);

        let mut b_string_mask = Vec::with_capacity(b_len);
        index_builder::build_string_mask_bitmap(&b_quote, &mut b_string_mask);

        for i in 0..b_len {
            let b = b_string_mask[i];
            b_colon[i] &= b;
            b_left[i] &= b;
            b_right[i] &= b;
        }

        let mut index = Vec::with_capacity(self.level);
        index_builder::build_leveled_colon_bitmap(&b_colon, &b_left, &b_right, self.level, &mut index);

        let mut results = Vec::with_capacity(self.query_strs_len);
        for _ in 0..self.query_strs_len {
            results.push(None);
        }

        if self.trained {
            if !parser::speculative_parse(
                rec,
                &index,
                &self.queries,
                0,
                rec_len - 1,
                0,
                &self.stats,
                &mut results,
                &b_quote,
            ) {
                parser::basic_parse(
                    rec,
                    &index,
                    &mut self.queries,
                    0,
                    rec_len - 1,
                    0,
                    self.queries_len,
                    &mut self.stats,
                    false,
                    &mut results,
                    &b_quote,
                );
            }
        } else {
            parser::basic_parse(
                rec,
                &index,
                &mut self.queries,
                0,
                rec_len - 1,
                0,
                self.queries_len,
                &mut self.stats,
                true,
                &mut results,
                &b_quote,
            );
            self.trained_num += 1;
            if self.trained_num >= self.train_num {
                self.trained = true;
            }
        }

        Ok(results)
    }
}


#[inline]
fn is_valid_query_str<'a>(query_str: &'a [u8]) -> bool {
    if query_str.len() < ROOT_QUERY_STR_OFFSET + 1 {
        return false;
    }
    if query_str[0] != DOLLAR || query_str[1] != DOT {
        return false;
    }
    let mut s = ROOT_QUERY_STR_OFFSET - 1;
    for i in s + 1..query_str.len() {
        if query_str[i] != DOT {
            continue;
        }
        if i == s + 1 {
            return false;
        }
        if i == query_str.len() - 1 {
            return false;
        }
        s = i;
    }
    true
}

#[inline]
fn set_queries<'a>(queries: &mut FnvHashMap<&'a [u8], Query<'a>>, s: &'a [u8], i: usize, qi: usize, ri: usize) -> (usize, usize) {
    for j in i..s.len() {
        if s[j] == DOT {
            let t = s.get(i..j).unwrap();
            let query = queries.entry(t).or_insert(Query {
                i: qi,
                ri: ri,
                target: false,
                children: None,
                children_len: 0,
            });
            let mut children = query.children.get_or_insert(FnvHashMap::default());
            let (child_level, next_qi) = set_queries(
                &mut children,
                s,
                j + 1,
                if qi == query.i { qi + 1 } else { qi },
                ri,
            );
            query.children_len = children.len();
            return (child_level + 1, next_qi);
        }
    }
    let t = s.get(i..s.len()).unwrap();
    if !queries.contains_key(t) {
        queries.insert(
            t,
            Query {
                i: qi,
                ri: ri,
                target: true,
                children: None,
                children_len: 0,
            },
        );
        return (1, qi + 1);
    } else {
        queries.get_mut(t).unwrap().target = true;
    }
    (1, qi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pikkr_new() {
        struct TestCase<'a> {
            query_strs: Vec<&'a [u8]>,
            err: bool,
        }
        let test_cases = vec![
            TestCase {
                query_strs: vec![],
                err: false,
            },
            TestCase {
                query_strs: vec!["".as_bytes()],
                err: true,
            },
            TestCase {
                query_strs: vec!["$".as_bytes()],
                err: true,
            },
            TestCase {
                query_strs: vec!["$.".as_bytes()],
                err: true,
            },
            TestCase {
                query_strs: vec!["$.aaaa".as_bytes()],
                err: false,
            },
            TestCase {
                query_strs: vec!["$.aaaa".as_bytes(), "".as_bytes()],
                err: true,
            },
            TestCase {
                query_strs: vec!["$.aaaa".as_bytes(), "$".as_bytes()],
                err: true,
            },
            TestCase {
                query_strs: vec!["$.aaaa".as_bytes(), "$.".as_bytes()],
                err: true,
            },
            TestCase {
                query_strs: vec!["$.aaaa".as_bytes(), "$.bbbb".as_bytes()],
                err: false,
            },
        ];
        for t in test_cases {
            let err = match Pikkr::new(&t.query_strs, 1) {
                Ok(_) => false,
                Err(_) => true,
            };
            assert_eq!(t.err, err);
        }
    }

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
        let mut p = Pikkr::new(&queries, 1000000000).unwrap();
        struct TestCase<'a> {
            rec: &'a str,
            want: Result<Vec<Option<&'a [u8]>>>,
        }
        let test_cases = vec![
            TestCase {
                rec: r#"{}"#,
                want: Ok(vec![None, None, None, None, None, None, None]),
            },
            TestCase {
                rec: r#"{"f0": "a"}"#,
                want: Ok(vec![None, None, None, None, None, None, None]),
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b"}"#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ]),
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    None,
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                    None,
                ]),
            },
            TestCase {
                rec: r#"{"f1": "Português do Brasil,Català,Deutsch,Español,Français,Bahasa,Italiano,עִבְרִית,日本語,한국어,Română,中文（简体）,中文（繁體）,Українська,Ўзбекча,Türkçe"}"#,
                want: Ok(vec![
                    Some(
                        r#""Português do Brasil,Català,Deutsch,Español,Français,Bahasa,Italiano,עִבְרִית,日本語,한국어,Română,中文（简体）,中文（繁體）,Українська,Ўзбекча,Türkçe""#.as_bytes(),
                    ),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ]),
            },
            TestCase {
                rec: r#"{"f1": "\"f1\": \\"}"#,
                want: Ok(vec![
                    Some(r#""\"f1\": \\""#.as_bytes()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ]),
            },
            TestCase {
                rec: r#"
                    	{
                     	"f1" 	 : 	 "b"
                    }
                "#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ]),
            },
            // for issue #10
            TestCase {
                rec: r#""#,
                want: Err(Error::from(ErrorKind::InvalidRecord)),
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
        let mut p = Pikkr::new(&queries, 1).unwrap();
        struct TestCase<'a> {
            rec: &'a str,
            want: Result<Vec<Option<&'a [u8]>>>,
        }
        let test_cases = vec![
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                ]),
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                ]),
            },
            TestCase {
                rec: r#"{"f1": "b", "f0": "a", "f3": [1, 2, 3], "f2": {"f2": {"f2": "d", "f1": "c"}, "f1": 1}}"#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f2": {"f2": "d", "f1": "c"}, "f1": 1}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    Some(r#"[1, 2, 3]"#.as_bytes()),
                ]),
            },
            TestCase {
                rec: r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}}"#,
                want: Ok(vec![
                    Some(r#""b""#.as_bytes()),
                    Some(r#"{"f1": 1, "f2": {"f1": "c", "f2": "d"}}"#.as_bytes()),
                    Some(r#"1"#.as_bytes()),
                    Some(r#""c""#.as_bytes()),
                    None,
                ]),
            },
            TestCase {
                rec: r#"{}"#,
                want: Ok(vec![None, None, None, None, None]),
            },
            // for issue #10
            TestCase {
                rec: r#""#,
                want: Err(Error::from(ErrorKind::InvalidRecord)),
            },
        ];
        for t in test_cases {
            let got = p.parse(t.rec.as_bytes());
            assert_eq!(t.want, got);
        }
    }

    #[test]
    fn test_is_valid_query_str() {
        struct TestCase<'a> {
            query_str: &'a str,
            want: bool,
        }
        let test_cases = vec![
            TestCase {
                query_str: "",
                want: false,
            },
            TestCase {
                query_str: "$",
                want: false,
            },
            TestCase {
                query_str: "$.",
                want: false,
            },
            TestCase {
                query_str: "$..",
                want: false,
            },
            TestCase {
                query_str: "a.a",
                want: false,
            },
            TestCase {
                query_str: "$aa",
                want: false,
            },
            TestCase {
                query_str: "$.a",
                want: true,
            },
            TestCase {
                query_str: "$.aaaa",
                want: true,
            },
            TestCase {
                query_str: "$.aaaa.",
                want: false,
            },
            TestCase {
                query_str: "$.aaaa.b",
                want: true,
            },
            TestCase {
                query_str: "$.aaaa.bbbb",
                want: true,
            },
            TestCase {
                query_str: "$.aaaa.bbbb.",
                want: false,
            },
        ];
        for t in test_cases {
            let got = is_valid_query_str(t.query_str.as_bytes());
            assert_eq!(t.want, got);
        }
    }
}
