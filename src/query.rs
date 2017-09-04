use fnv::FnvHashMap;

#[derive(Debug)]
pub struct Query<'a> {
    pub i: usize,
    pub ri: usize,
    pub target: bool,
    pub children: Option<FnvHashMap<&'a[u8], Query<'a>>>,
    pub children_len: usize,
}
