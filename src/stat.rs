use fnv::{FnvHashMap, FnvHashSet};

#[derive(Debug)]
pub struct Stat<'a> {
    pub locs: FnvHashSet<usize>,
    pub children: Option<FnvHashMap<&'a[u8], Stat<'a>>>,
}
