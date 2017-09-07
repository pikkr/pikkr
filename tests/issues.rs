extern crate pikkr;

#[allow(non_snake_case)]
mod issues {
    use pikkr::Pikkr;
    use pikkr::ErrorKind;

    /// The helper macro for test cases.
    ///
    /// The first argument is the parameters used in `Pikkr::new` and `Pikkr::parse`.
    /// The remaining tokens after `=>` are used as the pattern to match success case.
    macro_rules! do_parse {
        ($p:expr => $($ptn:tt)*) => {{
            let res = Pikkr::new($p.0, $p.1).map(|mut p| p.parse($p.2));
            match res {
                $($ptn)* => (),
                r => panic!(concat!("The result {:?} does not match the pattern \"", stringify!($($ptn)*), "\""), r),
            }
        }}
    }


    #[test]
    fn issue10_panic_on_empty_input() {
        let q = &["$.a"];
        let t = 1;
        let r = "";
        do_parse!((q, t, r) => Ok(Err(ref e)) if e.kind() == ErrorKind::InvalidRecord);
    }

    #[test]
    fn issue_15_panic_in_query_parser() {
        let q = &["$"];
        let t = 1;
        do_parse!((q, t, "") => Err(ref e) if e.kind() == ErrorKind::InvalidQuery);
    }

    // test cases for unclosed issues.
    // TODO: remove #[should_panic]

    #[test]
    fn issue11_panic_on_None_unwrapped_in_parser() {
        let q = &["$.a"];
        let t = 1;
        let r = &[40, 0, 0, 0, 159, 159, 159, 0, 0, 0, 0, 58][..];
        let _ = do_parse!((q, t, r) => Ok(Err(ref e)) if e.kind() == ErrorKind::InvalidRecord);
    }

    #[test]
    #[should_panic]
    fn issue12_panic_in_build_leveled_colon_bitmap() {
        let q = &["$.a"];
        let t = 1;
        let r = b"(}";
        let _ = do_parse!((q, t, r) => Ok(_));
    }

    #[test]
    fn issue13_integer_overflow_in_parser() {
        let q = &["$.a"];
        let t = 1;
        let r = b"\\\":";
        let _ = do_parse!((q, t, r) => Ok(Err(ref e)) if e.kind() == ErrorKind::InvalidRecord);
    }
}
