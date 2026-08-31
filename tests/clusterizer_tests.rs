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
    use bitiodine::address::CompactAddress;
    use bitiodine::hash160::Hash160;
    use bitiodine::visitors::clusterizer::Clusterizer;

    let mut clusterizer = Clusterizer::new();
    let addr_a = CompactAddress::from_hash160(&Hash160([0x11; 20]), 0x00);
    let addr_b = CompactAddress::from_hash160(&Hash160([0x22; 20]), 0x00);
    let addr_c = CompactAddress::from_hash160(&Hash160([0x33; 20]), 0x05);
    let addr_d = CompactAddress::from_hash160(&Hash160([0x44; 20]), 0x05);

    clusterizer.clusters.make_set(addr_b);
    clusterizer.clusters.make_set(addr_a);
    clusterizer.clusters.union(&addr_b, &addr_a);

    clusterizer.clusters.make_set(addr_d);
    clusterizer.clusters.make_set(addr_c);
    clusterizer.clusters.union(&addr_d, &addr_c);

    clusterizer.clusters.finalize();

    let mut csv_buf = Vec::new();
    clusterizer.write_csv(&mut csv_buf).unwrap();
    let csv = String::from_utf8(csv_buf).unwrap();

    let rep_ab = if addr_a < addr_b { addr_a } else { addr_b };
    let rep_cd = if addr_c < addr_d { addr_c } else { addr_d };

    let expected_entries = vec![
        (addr_a, rep_ab),
        (addr_b, rep_ab),
        (addr_c, rep_cd),
        (addr_d, rep_cd),
    ];

    // Row order is map-iteration order (unsorted); compare content as a set.
    let mut actual_lines: Vec<&str> = csv.lines().collect();
    actual_lines.sort_unstable();
    let mut expected_lines: Vec<String> = expected_entries
        .into_iter()
        .map(|(a, rep)| format!("{},{}", a, rep))
        .collect();
    expected_lines.sort_unstable();

    assert_eq!(actual_lines, expected_lines);
}
