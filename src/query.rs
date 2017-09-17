use std::cmp;
use std::collections::hash_map;
use fnv::FnvHashMap;
use error::{Error, ErrorKind};
use result::Result;
use utf8::{DOLLAR, DOT};

const ROOT_QUERY_STR_OFFSET: usize = 2;


/// A node in pattern tree
#[derive(Debug, Default)]
pub struct QueryNode<'a> {
    /// The identifier of this node
    node_id: Option<usize>,

    /// A identifier of path associated with this node
    path_id: Option<usize>,

    /// Children of this node
    children: FnvHashMap<&'a [u8], QueryNode<'a>>,
}

impl<'a> QueryNode<'a> {
    /// Returns whether this node is a leaf or not.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the identifier of this node, if avaialble.
    ///
    /// This function will return a `None` if the node is root.
    #[inline]
    pub fn node_id(&self) -> Option<usize> {
        self.node_id
    }

    /// Returns the identifier of this node.
    ///
    /// # Panics
    /// This function will panic if the node is root.
    #[inline]
    pub fn id(&self) -> usize {
        self.node_id.expect("The node is a root")
    }

    /// Returns the path identifier associated with this node.
    ///
    /// This function will return a `None` if the node is not a target.
    #[inline]
    pub fn path_id(&self) -> Option<usize> {
        self.path_id
    }

    /// Returns the reference of a child whose filed name is `field`, if available.
    #[inline]
    pub fn get_child(&self, field: &[u8]) -> Option<&QueryNode> {
        self.children.get(field)
    }

    /// Returns the number of childrens of this node.
    ///
    /// This function will return a zero if it is a leaf.
    #[inline]
    pub fn num_children(&self) -> usize {
        self.children.len()
    }

    #[inline]
    pub fn iter(&self) -> hash_map::Iter<&'a [u8], QueryNode<'a>> {
        self.children.iter()
    }
}


/// A pattern tree associated with the queries
#[derive(Debug, Default)]
pub struct QueryTree<'a> {
    root_node: QueryNode<'a>,
    paths: Vec<&'a [u8]>,
    max_level: usize,
    num_nodes: usize,
}

impl<'a> QueryTree<'a> {
    /// Create a new instance of `QueryTree` with given path sequence.
    pub fn new<S: ?Sized + AsRef<[u8]>>(paths: &[&'a S]) -> Result<Self> {
        let mut tree = Self::default();
        for path in paths {
            tree.add_path((*path).as_ref())?;
        }
        Ok(tree)
    }

    /// Add a path into the pattern tree.
    fn add_path(&mut self, path: &'a [u8]) -> Result<()> {
        if !is_valid_query_str(path) {
            return Err(Error::from(ErrorKind::InvalidQuery));
        }

        let mut cur = &mut self.root_node;
        let mut level = 0;
        for field in path[ROOT_QUERY_STR_OFFSET..].split(|&b| b == DOT) {
            level = level + 1;

            let num_nodes = &mut self.num_nodes;
            let cur1 = cur; // workaround for lifetime error
            cur = cur1.children.entry(field).or_insert_with(|| {
                let node = QueryNode {
                    node_id: Some(*num_nodes),
                    ..Default::default()
                };
                *num_nodes += 1;
                node
            });
        }
        // mark the last node as a target
        cur.path_id = Some(self.paths.len());

        self.max_level = cmp::max(self.max_level, level);
        self.paths.push(path);

        Ok(())
    }

    /// Returns the reference of root node of this pattern tree.
    #[inline]
    pub fn as_node(&self) -> &QueryNode {
        &self.root_node
    }

    /// Returns the max level of pattern tree.
    #[inline]
    pub fn max_level(&self) -> usize {
        self.max_level
    }

    /// Returns the number of query paths, registered in this pattern tree.
    #[inline]
    pub fn num_paths(&self) -> usize {
        self.paths.len()
    }

    /// Returns the number of nodes excluding root node in this pattern tree.
    #[inline]
    pub fn num_nodes(&self) -> usize {
        self.num_nodes
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
