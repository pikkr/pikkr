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

            let (l, next_qi) = set_queries(&mut root, s, ROOT_QUERY_STR_OFFSET, qi, ri);
            level = cmp::max(level, l);
            qi = next_qi;
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
fn set_queries<'a>(queries: &mut FnvHashMap<&'a [u8], Query<'a>>, s: &'a [u8], i: usize, qi: usize, ri: usize) -> (usize, usize) {
    for j in i..s.len() {
        if s[j] == DOT {
            let t = &s[i..j];
            let query = queries.entry(t).or_insert(Query {
                i: qi,
                ri: ri,
                target: false,
                children: None,
            });
            let mut children = query.children.get_or_insert(FnvHashMap::default());
            let (child_level, next_qi) = set_queries(
                &mut children,
                s,
                j + 1,
                if qi == query.i { qi + 1 } else { qi },
                ri,
            );
            return (child_level + 1, next_qi);
        }
    }
    let t = &s[i..];
    if !queries.contains_key(t) {
        queries.insert(
            t,
            Query {
                i: qi,
                ri: ri,
                target: true,
                children: None,
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
