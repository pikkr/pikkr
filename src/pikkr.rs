use super::error::{Error, ErrorKind};
use super::index_builder::IndexBuilder;
use super::parser;
use super::query::QueryTree;
use super::result::Result;
use fnv::FnvHashSet;

/// JSON parser which picks up values directly without performing tokenization
pub struct Pikkr<'a> {
    queries: QueryTree<'a>,
    index_builder: IndexBuilder,

    stats: Vec<FnvHashSet<usize>>,
    colon_positions: Vec<Vec<usize>>,

    train_num: usize,
    trained_num: usize,
    trained: bool,
}

impl<'a> Pikkr<'a> {
    /// Creates a JSON parser and returns it.
    #[inline]
    pub fn new<S: ?Sized + AsRef<[u8]>>(query_strs: &[&'a S], train_num: usize) -> Result<Pikkr<'a>> {
        let queries = QueryTree::new(query_strs)?;

        let index_builder = IndexBuilder::new(queries.max_depth);
        let colon_positions = vec![Vec::new(); queries.max_depth];
        let stats = vec![Default::default(); queries.num_nodes];

        Ok(Pikkr {
            queries,
            index_builder,

            stats,
            colon_positions,

            train_num,
            trained_num: 0,
            trained: false,
        })
    }

    /// Parses a JSON record and returns the result.
    #[inline]
    pub fn parse<'b, S: ?Sized + AsRef<[u8]>>(&mut self, rec: &'b S) -> Result<Vec<Option<&'b [u8]>>> {
        let rec = rec.as_ref();
        if rec.len() == 0 {
            return Err(Error::from(ErrorKind::InvalidRecord));
        }

        self.index_builder.build_structural_indices(rec)?;

        if self.trained {
            self.speculative_parse(rec)
        } else {
            let results = self.basic_parse(rec)?;
            self.trained_num += 1;
            if self.trained_num >= self.train_num {
                self.trained = true;
            }
            Ok(results)
        }
    }

    fn speculative_parse<'b>(&mut self, rec: &'b [u8]) -> Result<Vec<Option<&'b [u8]>>> {
        let mut results = vec![None; self.queries.num_queries];
        let found = parser::speculative_parse(
            rec,
            &self.index_builder.index(),
            &self.queries.root,
            0,
            rec.len() - 1,
            0,
            &self.stats,
            &mut results,
            &self.index_builder.b_quote(),
            &mut self.colon_positions,
        )?;
        if !found {
            let queries_len = self.queries.root.len();
            parser::basic_parse(
                rec,
                &self.index_builder.index(),
                &mut self.queries.root,
                0,
                rec.len() - 1,
                0,
                queries_len,
                &mut self.stats,
                false,
                &mut results,
                &self.index_builder.b_quote(),
                &mut self.colon_positions,
            )?;
        }
        Ok(results)
    }

    fn basic_parse<'b>(&mut self, rec: &'b [u8]) -> Result<Vec<Option<&'b [u8]>>> {
        let mut results = vec![None; self.queries.num_queries];
        let queries_len = self.queries.root.len();
        parser::basic_parse(
            rec,
            &self.index_builder.index(),
            &mut self.queries.root,
            0,
            rec.len() - 1,
            0,
            queries_len,
            &mut self.stats,
            true,
            &mut results,
            &self.index_builder.b_quote(),
            &mut self.colon_positions,
        )?;
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
