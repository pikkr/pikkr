#![feature(test)]

extern crate pikkr;
extern crate test;

use test::Bencher;
use pikkr::parser::Parser;
use pikkr::query::QueryTree;

#[bench]
fn basic_parse(b: &mut Bencher) {
    let json_rec_str = r#"{ "aaa" : "AAA", "bbb" : 111, "ccc": ["C1", "C2"], "ddd" : { "d1" : "D1", "d2" : "D2", "d3": 333 }, "eee": { "e1": "EEE" } } "#;
    let json_rec = json_rec_str.as_bytes();
    let query_strs = &["$.ddd.d1", "$.ddd.d3", "$.aaa", "$.bbb", "$.ccc", "$.eee"];

    let queries = QueryTree::new(query_strs).unwrap();

    let mut parser = Parser::new(&queries);
    parser
        .index_builder
        .build_structural_indices(json_rec)
        .unwrap();

    b.iter(|| {
        let mut results = vec![None; query_strs.len()];
        parser
            .basic_parse(
                json_rec,
                &queries.as_node(),
                0,
                json_rec.len() - 1,
                0,
                false,
                &mut results,
            )
            .unwrap();
    });
}

#[bench]
fn speculative_parse(b: &mut Bencher) {
    let json_rec_str = r#"{ "aaa" : "AAA", "bbb" : 111, "ccc": ["C1", "C2"], "ddd" : { "d1" : "D1", "d2" : "D2", "d3": 333 }, "eee": { "e1": "EEE" } } "#;
    let json_rec = json_rec_str.as_bytes();
    let query_strs = &["$.ddd.d1", "$.ddd.d3", "$.aaa", "$.bbb", "$.ccc", "$.eee"];

    let queries = QueryTree::new(query_strs).unwrap();

    let mut parser = Parser::new(&queries);
    parser
        .index_builder
        .build_structural_indices(json_rec)
        .unwrap();

    let mut results = vec![None; query_strs.len()];
    parser
        .basic_parse(
            json_rec,
            &queries.as_node(),
            0,
            json_rec.len() - 1,
            0,
            true,
            &mut results,
        )
        .unwrap();

    b.iter(|| {
        let mut results = vec![None; query_strs.len()];
        parser
            .speculative_parse(
                json_rec,
                &queries.as_node(),
                0,
                json_rec.len() - 1,
                0,
                &mut results,
            )
            .unwrap();
    });
}
