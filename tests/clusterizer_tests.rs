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
