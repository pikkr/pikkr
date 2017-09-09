use super::avx;
use super::error::{Error, ErrorKind};
use super::index_builder;
use super::parser;
use super::query::QueryTree;
use super::result::Result;
use super::utf8::{BACKSLASH, COLON, LEFT_BRACE, QUOTE, RIGHT_BRACE};
use fnv::FnvHashSet;
use x86intrin::m256i;

/// JSON parser which picks up values directly without performing tokenization
pub struct Pikkr<'a> {
    backslash: m256i,
    quote: m256i,
    colon: m256i,
    left_brace: m256i,
    right_brace: m256i,

    queries: QueryTree<'a>,

    b_backslash: Vec<u64>,
    b_quote: Vec<u64>,
    b_colon: Vec<u64>,
    b_left: Vec<u64>,
    b_right: Vec<u64>,
    b_string_mask: Vec<u64>,

    index: Vec<Vec<u64>>,

    train_num: usize,
    trained_num: usize,
    trained: bool,

    stats: Vec<FnvHashSet<usize>>,
}

impl<'a> Pikkr<'a> {
    /// Creates a JSON parser and returns it.
    #[inline]
    pub fn new<S: ?Sized + AsRef<[u8]>>(query_strs: &[&'a S], train_num: usize) -> Result<Pikkr<'a>> {
        let queries = QueryTree::new(query_strs)?;

        let index = vec![Vec::new(); queries.level];
        let stats = vec![Default::default(); queries.qi];

        Ok(Pikkr {
            backslash: avx::mm256i(BACKSLASH as i8),
            quote: avx::mm256i(QUOTE as i8),
            colon: avx::mm256i(COLON as i8),
            left_brace: avx::mm256i(LEFT_BRACE as i8),
            right_brace: avx::mm256i(RIGHT_BRACE as i8),

            queries,

            b_backslash: Vec::new(),
            b_quote: Vec::new(),
            b_colon: Vec::new(),
            b_left: Vec::new(),
            b_right: Vec::new(),
            b_string_mask: Vec::new(),

            index,

            train_num,
            trained_num: 0,
            trained: false,

            stats,
        })
    }

    #[inline(always)]
    fn build_structural_indices(&mut self, rec: &[u8]) -> Result<()> {
        let b_len = (rec.len() + 63) / 64;

        self.b_backslash.clear();
        self.b_quote.clear();
        self.b_colon.clear();
        self.b_left.clear();
        self.b_right.clear();
        self.b_string_mask.clear();
        for b in self.index.iter_mut() {
            b.clear();
        }

        if b_len > self.b_backslash.capacity() {
            self.b_backslash.reserve_exact(b_len);
            self.b_quote.reserve_exact(b_len);
            self.b_colon.reserve_exact(b_len);
            self.b_left.reserve_exact(b_len);
            self.b_right.reserve_exact(b_len);
            self.b_string_mask.reserve_exact(b_len);
            for b in self.index.iter_mut() {
                b.reserve_exact(b_len);
            }
        }

        index_builder::build_structural_character_bitmap(
            rec,
            &mut self.b_backslash,
            &mut self.b_quote,
            &mut self.b_colon,
            &mut self.b_left,
            &mut self.b_right,
            &self.backslash,
            &self.quote,
            &self.colon,
            &self.left_brace,
            &self.right_brace,
        );

        index_builder::build_structural_quote_bitmap(&self.b_backslash, &mut self.b_quote);

        index_builder::build_string_mask_bitmap(&self.b_quote, &mut self.b_string_mask);

        for (i, b) in self.b_string_mask.iter().enumerate() {
            self.b_colon[i] &= *b;
            self.b_left[i] &= *b;
            self.b_right[i] &= *b;
        }

        index_builder::build_leveled_colon_bitmap(
            &self.b_colon,
            &self.b_left,
            &self.b_right,
            self.queries.level,
            &mut self.index,
        )
    }

    /// Parses a JSON record and returns the result.
    #[inline]
    pub fn parse<'b, S: ?Sized + AsRef<[u8]>>(&mut self, rec: &'b S) -> Result<Vec<Option<&'b [u8]>>> {
        let rec = rec.as_ref();
        if rec.len() == 0 {
            return Err(Error::from(ErrorKind::InvalidRecord));
        }

        self.build_structural_indices(rec)?;

        let mut results = vec![None; self.queries.query_strs_len];

        if self.trained {
            let found = parser::speculative_parse(
                rec,
                &self.index,
                &self.queries.queries,
                0,
                rec.len() - 1,
                0,
                &self.stats,
                &mut results,
                &self.b_quote,
            )?;
            if !found {
                parser::basic_parse(
                    rec,
                    &self.index,
                    &mut self.queries.queries,
                    0,
                    rec.len() - 1,
                    0,
                    self.queries.queries_len,
                    &mut self.stats,
                    false,
                    &mut results,
                    &self.b_quote,
                )?;
            }
        } else {
            parser::basic_parse(
                rec,
                &self.index,
                &mut self.queries.queries,
                0,
                rec.len() - 1,
                0,
                self.queries.queries_len,
                &mut self.stats,
                true,
                &mut results,
                &self.b_quote,
            )?;
            self.trained_num += 1;
            if self.trained_num >= self.train_num {
                self.trained = true;
            }
        }

        Ok(results)
    }
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
            let err = Pikkr::new(&t.query_strs, 1).is_err();
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
                        "f1"     :   "b"
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
        ];
        for t in test_cases {
            let got = p.parse(t.rec.as_bytes());
            assert_eq!(t.want, got);
        }
    }
}
