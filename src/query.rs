use fnv::FnvHashMap;

#[derive(Debug)]
pub struct Query<'a> {
    pub result: Option<(usize, usize, usize)>,
    pub children: Option<FnvHashMap<&'a[u8], Query<'a>>>,
}
