use bitiodine::visitors::clusterizer::DisjointSet;

#[test]
fn test_disjoint_set_union_find() {
    let mut ds: DisjointSet<&str> = DisjointSet::new();

    ds.make_set("addr1");
    ds.make_set("addr2");
    ds.make_set("addr3");
    ds.make_set("addr4");

    assert_eq!(ds.size(), 4);

    // Duplicate make_set should not increase size
    let tag = ds.make_set("addr1");
    assert_eq!(ds.size(), 4);
    assert_eq!(ds.find(&"addr1"), Some(tag));

    // Initial roots should be distinct
    let r1 = ds.find(&"addr1").unwrap();
    let r2 = ds.find(&"addr2").unwrap();
    let r3 = ds.find(&"addr3").unwrap();
    let _r4 = ds.find(&"addr4").unwrap();

    assert_ne!(r1, r2);
    assert_ne!(r2, r3);

    // Union addr1 and addr2
    ds.union(&"addr1", &"addr2").unwrap();
    assert_eq!(ds.find(&"addr1"), ds.find(&"addr2"));

    // Union addr3 and addr4
    ds.union(&"addr3", &"addr4").unwrap();
    assert_eq!(ds.find(&"addr3"), ds.find(&"addr4"));
    assert_ne!(ds.find(&"addr1"), ds.find(&"addr3"));

    // Union the two clusters together
    ds.union(&"addr2", &"addr3").unwrap();
    assert_eq!(ds.find(&"addr1"), ds.find(&"addr4"));

    // Finalize
    ds.finalize();
    assert_eq!(ds.find(&"addr1"), ds.find(&"addr4"));
}

#[test]
fn test_clusterizer_deterministic_representatives() {
    use bitiodine::address::Address;
    use bitiodine::visitors::clusterizer::Clusterizer;

    let mut clusterizer = Clusterizer::new();
    let addr_a = Address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string());
    let addr_b = Address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2".to_string());
    let addr_c = Address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy".to_string());
    let addr_d = Address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string());

    clusterizer.clusters.make_set(addr_b.clone());
    clusterizer.clusters.make_set(addr_a.clone());
    clusterizer.clusters.union(&addr_b, &addr_a);

    clusterizer.clusters.make_set(addr_d.clone());
    clusterizer.clusters.make_set(addr_c.clone());
    clusterizer.clusters.union(&addr_d, &addr_c);

    clusterizer.clusters.finalize();

    let mut csv_buf = Vec::new();
    clusterizer.write_csv(&mut csv_buf).unwrap();
    let csv = String::from_utf8(csv_buf).unwrap();

    let expected = "\
1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa,1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa\n\
1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2,1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa\n\
3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy,3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy\n\
bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq,3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy\n";

    assert_eq!(csv, expected);
}
