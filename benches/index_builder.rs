#![feature(test)]

extern crate pikkr;
extern crate test;

use test::Bencher;
use pikkr::index_builder::IndexBuilder;

#[inline(never)]
fn build_structural_indices(b: &mut Bencher, max_depth: usize, rec: &str) {
    let mut index_builder = IndexBuilder::new(max_depth);
    b.iter(|| {
        index_builder
            .build_structural_indices(rec.as_bytes())
            .unwrap()
    });
}

#[cfg(test)]
mod index_builder {
    use super::*;

    #[bench]
    fn empty_object(b: &mut Bencher) {
        let depth = 1;
        let rec = "{}";
        build_structural_indices(b, depth, rec);
    }

    #[bench]
    fn one_field(b: &mut Bencher) {
        let depth = 1;
        let rec = r#"{"f0": "a"}"#;
        build_structural_indices(b, depth, rec);
    }

    #[bench]
    fn multiple_fields(b: &mut Bencher) {
        let depth = 1;
        let rec = r#"{"f0": "a", "f1": "b"}"#;
        build_structural_indices(b, depth, rec);
    }

    #[bench]
    fn nested_object(b: &mut Bencher) {
        let depth = 3;
        let rec = r#"{"f0": "a", "f1": "b", "f2": {"f1": 1, "f2": {"f1": "c", "f2": "d"}}, "f3": [1, 2, 3]}"#;
        build_structural_indices(b, depth, rec);
    }

    #[bench]
    fn multibyte_characters(b: &mut Bencher) {
        let depth = 1;
        let rec = r#"{"f1": "Português do Brasil,Català,Deutsch,Español,Français,Bahasa,Italiano,עִבְרִית,日本語,한국어,Română,中文（简体）,中文（繁體）,Українська,Ўзбекча,Türkçe"}"#;
        build_structural_indices(b, depth, rec);
    }

    #[bench]
    fn with_quoted(b: &mut Bencher) {
        let depth = 1;
        let rec = r#"{"f1": "\"f1\": \\"}"#;
        build_structural_indices(b, depth, rec);
    }

    #[bench]
    fn with_newlines(b: &mut Bencher) {
        let depth = 1;
        let rec = r#"
                        {
                        "f1"     :   "b"
                    }
                "#;
        build_structural_indices(b, depth, rec);
    }
}
