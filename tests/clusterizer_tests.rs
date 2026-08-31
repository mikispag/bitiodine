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
    let addr_a = Address::Base58 {
        version: 0x00,
        hash: [0x11; 20],
    };
    let addr_b = Address::Base58 {
        version: 0x00,
        hash: [0x22; 20],
    };
    let addr_c = Address::Base58 {
        version: 0x05,
        hash: [0x33; 20],
    };
    let addr_d = Address::Base58 {
        version: 0x05,
        hash: [0x44; 20],
    };

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

    let rep_ab = if addr_a < addr_b { &addr_a } else { &addr_b };
    let rep_cd = if addr_c < addr_d { &addr_c } else { &addr_d };

    let expected_entries = vec![
        (addr_a.clone(), rep_ab.clone()),
        (addr_b.clone(), rep_ab.clone()),
        (addr_c.clone(), rep_cd.clone()),
        (addr_d.clone(), rep_cd.clone()),
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
