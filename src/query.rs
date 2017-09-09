use std::cmp;
use fnv::FnvHashMap;
use error::{Error, ErrorKind};
use result::Result;
use utf8::{DOLLAR, DOT};

const ROOT_QUERY_STR_OFFSET: usize = 2;


#[derive(Debug)]
pub struct Query<'a> {
    pub i: usize,
    pub ri: usize,
    pub target: bool,
    pub children: Option<FnvHashMap<&'a [u8], Query<'a>>>,
}

/// A pattern tree associated with the queries
pub struct QueryTree<'a> {
    pub root: FnvHashMap<&'a [u8], Query<'a>>,
    pub num_queries: usize,

    pub level: usize,
    pub qi: usize,
}

impl<'a> QueryTree<'a> {
    pub fn new<S: ?Sized + AsRef<[u8]>>(queries: &[&'a S]) -> Result<Self> {
        let mut root = FnvHashMap::default();
        let mut level = 0;
        let mut qi = 0;

        for (ri, s) in queries.into_iter().map(|s| (*s).as_ref()).enumerate() {
            if !is_valid_query_str(s) {
                return Err(Error::from(ErrorKind::InvalidQuery));
            }

            let l = set_queries(&mut root, &s[ROOT_QUERY_STR_OFFSET..], &mut qi, ri);
            level = cmp::max(level, l);
        }

        Ok(Self {
            root,
            num_queries: queries.len(),
            level,
            qi,
        })
    }
}

#[inline]
fn is_valid_query_str(query_str: &[u8]) -> bool {
    if query_str.len() < ROOT_QUERY_STR_OFFSET + 1 || query_str[0] != DOLLAR || query_str[1] != DOT {
        return false;
    }
    let mut s = ROOT_QUERY_STR_OFFSET - 1;
    for i in s + 1..query_str.len() {
        if query_str[i] != DOT {
            continue;
        }
        if i == s + 1 || i == query_str.len() - 1 {
            return false;
        }
        s = i;
    }
    true
}

#[inline]
fn set_queries<'a>(queries: &mut FnvHashMap<&'a [u8], Query<'a>>, s: &'a [u8], qi: &mut usize, ri: usize) -> usize {
    for j in 0..s.len() {
        if s[j] == DOT {
            let (t, u) = s.split_at(j);
            let query = queries.entry(t).or_insert_with(|| {
                let query = Query {
                    i: *qi,
                    ri: ri,
                    target: false,
                    children: None,
                };
                *qi += 1;
                query
            });
            let mut children = query.children.get_or_insert(FnvHashMap::default());
            return set_queries(&mut children, &u[1..], qi, ri) + 1;
        }
    }

    let query = queries.entry(s).or_insert_with(|| {
        let query = Query {
            i: *qi,
            ri: ri,
            target: false,
            children: None,
        };
        *qi += 1;
        query
    });
    query.target = true;
    1
}

#[cfg(test)]
mod tests {
    use super::*;

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
