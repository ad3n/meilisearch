/*!
This module tests the `geo_sort` ranking rule:

1. an error is returned if the sort ranking rule exists but no fields-to-sort were given at search time
2. an error is returned if the fields-to-sort are not sortable
3. it is possible to add multiple fields-to-sort at search time
4. custom sort ranking rules can be added to the settings, they interact with the generic `sort` ranking rule as expected
5. numbers appear before strings
6. documents with either: (1) no value, (2) null, or (3) an object for the field-to-sort appear at the end of the bucket
7. boolean values are translated to strings
8. if a field contains an array, it is sorted by the best value in the array according to the sort rule
*/

use big_s::S;
use heed::RoTxn;
use maplit::hashset;

use crate::index::tests::TempIndex;
use crate::search::new::tests::collect_field_values;
use crate::{AscDesc, Criterion, GeoSortStrategy, Member, Search, SearchResult};

fn create_index() -> TempIndex {
    let index = TempIndex::new();

    index
        .update_settings(|s| {
            s.set_primary_key("id".to_owned());
            s.set_sortable_fields(hashset! { S("_geo") });
            s.set_criteria(vec![Criterion::Words, Criterion::Sort]);
        })
        .unwrap();
    index
}

#[track_caller]
fn execute_iterative_and_rtree_returns_the_same<'a>(
    rtxn: &RoTxn<'a>,
    index: &TempIndex,
    search: &mut Search<'a>,
) -> Vec<usize> {
    search.geo_sort_strategy(GeoSortStrategy::AlwaysIterative(2));
    let SearchResult { documents_ids, .. } = search.execute().unwrap();
    let iterative_ids_bucketed = collect_field_values(&index, rtxn, "id", &documents_ids);

    search.geo_sort_strategy(GeoSortStrategy::AlwaysIterative(1000));
    let SearchResult { documents_ids, .. } = search.execute().unwrap();
    let iterative_ids = collect_field_values(&index, rtxn, "id", &documents_ids);

    assert_eq!(iterative_ids_bucketed, iterative_ids, "iterative bucket");

    search.geo_sort_strategy(GeoSortStrategy::AlwaysRtree(2));
    let SearchResult { documents_ids, .. } = search.execute().unwrap();
    let rtree_ids_bucketed = collect_field_values(&index, rtxn, "id", &documents_ids);

    search.geo_sort_strategy(GeoSortStrategy::AlwaysRtree(1000));
    let SearchResult { documents_ids, .. } = search.execute().unwrap();
    let rtree_ids = collect_field_values(&index, rtxn, "id", &documents_ids);

    assert_eq!(rtree_ids_bucketed, rtree_ids, "rtree bucket");

    assert_eq!(iterative_ids, rtree_ids, "iterative vs rtree");

    iterative_ids.into_iter().map(|id| id.parse().unwrap()).collect()
}

#[test]
fn test_geo_sort() {
    let index = create_index();

    index
        .add_documents(documents!([
            { "id": 2, "_geo": { "lat": 2, "lng": -1 } },
            { "id": 3, "_geo": { "lat": -2, "lng": -2 } },
            { "id": 5, "_geo": { "lat": 6, "lng": -5 } },
            { "id": 4, "_geo": { "lat": 3, "lng": 5 } },
            { "id": 0, "_geo": { "lat": 0, "lng": 0 } },
            { "id": 1, "_geo": { "lat": 1, "lng": 1 } },
            { "id": 6 }, { "id": 8 }, { "id": 7 }, { "id": 10 }, { "id": 9 },
        ]))
        .unwrap();

    let txn = index.read_txn().unwrap();

    let mut s = Search::new(&txn, &index);

    // --- asc
    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([0., 0.]))]);

    s.geo_sort_strategy(GeoSortStrategy::Dynamic(100));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["0", "1", "2", "3", "4", "5", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::Dynamic(3));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["0", "1", "2", "3", "4", "5", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysIterative(100));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["0", "1", "2", "3", "4", "5", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysIterative(3));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["0", "1", "2", "3", "4", "5", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysRtree(100));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["0", "1", "2", "3", "4", "5", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysRtree(3));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["0", "1", "2", "3", "4", "5", "6", "8", "7", "10", "9"]"###);

    // --- desc
    s.sort_criteria(vec![AscDesc::Desc(Member::Geo([0., 0.]))]);

    s.geo_sort_strategy(GeoSortStrategy::Dynamic(100));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["5", "4", "3", "2", "1", "0", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::Dynamic(3));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["5", "4", "3", "2", "1", "0", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysIterative(100));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["5", "4", "3", "2", "1", "0", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysIterative(3));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["5", "4", "3", "2", "1", "0", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysRtree(100));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["5", "4", "3", "2", "1", "0", "6", "8", "7", "10", "9"]"###);

    s.geo_sort_strategy(GeoSortStrategy::AlwaysRtree(3));
    let SearchResult { documents_ids, .. } = s.execute().unwrap();
    let ids = collect_field_values(&index, &txn, "id", &documents_ids);
    insta::assert_snapshot!(format!("{ids:?}"), @r###"["5", "4", "3", "2", "1", "0", "6", "8", "7", "10", "9"]"###);
}

#[test]
fn test_geo_sort_around_the_edge_of_the_flat_earth() {
    let index = create_index();

    index
        .add_documents(documents!([
            { "id": 0, "_geo": { "lat": 0, "lng": 0 } },
            { "id": 1, "_geo": { "lat": 88, "lng": 0 } },
            { "id": 2, "_geo": { "lat": -89, "lng": 0 } },

            { "id": 3, "_geo": { "lat": 0, "lng": 178 } },
            { "id": 4, "_geo": { "lat": 0, "lng": -179 } },
        ]))
        .unwrap();

    let rtxn = index.read_txn().unwrap();

    let mut s = Search::new(&rtxn, &index);

    // --- asc
    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([0., 0.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[0, 1, 2, 3, 4]");

    // ensuring the lat doesn't wrap around
    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([85., 0.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[1, 0, 3, 4, 2]");

    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([-85., 0.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[2, 0, 3, 4, 1]");

    // ensuring the lng does wrap around
    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([0., 175.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[3, 4, 2, 1, 0]");

    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([0., -175.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[4, 3, 2, 1, 0]");

    // --- desc
    s.sort_criteria(vec![AscDesc::Desc(Member::Geo([0., 0.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[4, 3, 2, 1, 0]");

    // ensuring the lat doesn't wrap around
    s.sort_criteria(vec![AscDesc::Desc(Member::Geo([85., 0.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[2, 4, 3, 0, 1]");

    s.sort_criteria(vec![AscDesc::Desc(Member::Geo([-85., 0.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[1, 4, 3, 0, 2]");

    // ensuring the lng does wrap around
    s.sort_criteria(vec![AscDesc::Desc(Member::Geo([0., 175.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[0, 1, 2, 4, 3]");

    s.sort_criteria(vec![AscDesc::Desc(Member::Geo([0., -175.]))]);
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[0, 1, 2, 3, 4]");
}

#[test]
fn geo_sort_mixed_with_words() {
    let index = create_index();

    index
        .add_documents(documents!([
            { "id": 0, "doggo": "jean", "_geo": { "lat": 0, "lng": 0 } },
            { "id": 1, "doggo": "intel", "_geo": { "lat": 88, "lng": 0 } },
            { "id": 2, "doggo": "jean bob", "_geo": { "lat": -89, "lng": 0 } },
            { "id": 3, "doggo": "jean michel", "_geo": { "lat": 0, "lng": 178 } },
            { "id": 4, "doggo": "bob marley", "_geo": { "lat": 0, "lng": -179 } },
        ]))
        .unwrap();

    let rtxn = index.read_txn().unwrap();

    let mut s = Search::new(&rtxn, &index);
    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([0., 0.]))]);

    s.query("jean");
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[0, 2, 3]");

    s.query("bob");
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[2, 4]");

    s.query("intel");
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[1]");
}

#[test]
fn geo_sort_without_any_geo_faceted_documents() {
    let index = create_index();

    index
        .add_documents(documents!([
            { "id": 0, "doggo": "jean" },
            { "id": 1, "doggo": "intel" },
            { "id": 2, "doggo": "jean bob" },
            { "id": 3, "doggo": "jean michel" },
            { "id": 4, "doggo": "bob marley" },
        ]))
        .unwrap();

    let rtxn = index.read_txn().unwrap();

    let mut s = Search::new(&rtxn, &index);
    s.sort_criteria(vec![AscDesc::Asc(Member::Geo([0., 0.]))]);

    s.query("jean");
    let ids = execute_iterative_and_rtree_returns_the_same(&rtxn, &index, &mut s);
    insta::assert_snapshot!(format!("{ids:?}"), @"[0, 2, 3]");
}
