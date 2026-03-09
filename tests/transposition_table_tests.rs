use game_player::TranspositionTable;

#[test]
fn test_check_nonexistent_entry() {
    let table = TranspositionTable::new(100);
    assert_eq!(table.check(12345, -1), None);
    assert_eq!(table.check(12345, 0), None);
    assert_eq!(table.check(12345, 5), None);
}

#[test]
fn test_update_and_check_basic() {
    let mut table = TranspositionTable::new(100);

    // Add an entry
    table.update(12345, (1.5, 5));

    // Check it's there
    assert_eq!(table.check(12345, -1), Some((1.5, 5)));
    assert_eq!(table.check(12345, 0), Some((1.5, 5)));
    assert_eq!(table.check(12345, 5), Some((1.5, 5)));
}

#[test]
fn test_check_minimum_quality() {
    let mut table = TranspositionTable::new(100);

    table.update(12345, (2.0, 5));

    // Check with various minimum qualities
    assert_eq!(table.check(12345, -1), Some((2.0, 5))); // No minimum
    assert_eq!(table.check(12345, 0), Some((2.0, 5))); // Below stored quality
    assert_eq!(table.check(12345, 3), Some((2.0, 5))); // Below stored quality
    assert_eq!(table.check(12345, 5), Some((2.0, 5))); // Equal to stored quality
    assert_eq!(table.check(12345, 6), None); // Above stored quality
    assert_eq!(table.check(12345, 10), None); // Well above stored quality
}

#[test]
fn test_update_quality_replacement_rules() {
    let mut table = TranspositionTable::new(100);

    // Add initial entry
    table.update(12345, (1.0, 5));
    assert_eq!(table.check(12345, -1), Some((1.0, 5)));

    // Try to update with lower quality (should not replace)
    table.update(12345, (2.0, 3));
    assert_eq!(table.check(12345, -1), Some((1.0, 5))); // Original value

    // Update with equal quality (should replace)
    table.update(12345, (3.0, 5));
    assert_eq!(table.check(12345, -1), Some((3.0, 5))); // New value

    // Update with higher quality (should replace)
    table.update(12345, (4.0, 7));
    assert_eq!(table.check(12345, -1), Some((4.0, 7))); // New value
}

#[test]
fn test_set_always_replaces() {
    let mut table = TranspositionTable::new(100);

    // Add initial entry
    table.set(12345, 1.0, 5);
    assert_eq!(table.check(12345, -1), Some((1.0, 5)));

    // Set with lower quality (should still replace)
    table.set(12345, 2.0, 3);
    assert_eq!(table.check(12345, -1), Some((2.0, 3)));

    // Set with higher quality (should replace)
    table.set(12345, 3.0, 7);
    assert_eq!(table.check(12345, -1), Some((3.0, 7)));
}

#[test]
fn test_multiple_entries() {
    let mut table = TranspositionTable::new(100);

    // Add multiple entries
    table.update(1, (1.0, 1));
    table.update(2, (2.0, 2));
    table.update(3, (3.0, 3));

    // Check all entries exist
    assert_eq!(table.check(1, -1), Some((1.0, 1)));
    assert_eq!(table.check(2, -1), Some((2.0, 2)));
    assert_eq!(table.check(3, -1), Some((3.0, 3)));

    // Check non-existent entry
    assert_eq!(table.check(4, -1), None);
}

#[test]
fn test_hash_collision_handling() {
    // With HashMap, different fingerprints are stored independently
    let mut table = TranspositionTable::new(100);

    // Add first entry
    table.update(1, (1.0, 5));
    assert_eq!(table.check(1, -1), Some((1.0, 5)));

    // Add second entry with different fingerprint
    // Both should coexist since HashMap handles them separately
    table.update(2, (2.0, 5));
    assert_eq!(table.check(2, -1), Some((2.0, 5)));
    assert_eq!(table.check(1, -1), Some((1.0, 5))); // First entry still there
}

#[test]
fn test_floating_point_values() {
    let mut table = TranspositionTable::new(100);

    // Test various floating point values
    let test_values = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        3.14159,
        -2.71828,
        f32::MIN,
        f32::MAX,
        f32::EPSILON,
        -f32::EPSILON,
    ];

    for (i, &value) in test_values.iter().enumerate() {
        let fingerprint = (i + 1) as u64;
        table.update(fingerprint, (value, 1));
        assert_eq!(table.check(fingerprint, -1), Some((value, 1)));
    }
}

#[test]
fn test_edge_case_qualities() {
    let mut table = TranspositionTable::new(100);

    // Test with quality 0
    table.update(1, (1.0, 0));
    assert_eq!(table.check(1, -1), Some((1.0, 0)));
    assert_eq!(table.check(1, 0), Some((1.0, 0)));
    assert_eq!(table.check(1, 1), None);

    // Test with high quality
    table.update(2, (2.0, i16::MAX));
    assert_eq!(table.check(2, -1), Some((2.0, i16::MAX)));
    assert_eq!(table.check(2, i16::MAX), Some((2.0, i16::MAX)));
}

#[test]
#[should_panic(expected = "assertion failed: quality >= 0")]
fn test_update_with_negative_quality() {
    let mut table = TranspositionTable::new(100);
    table.update(12345, (1.0, -1));
}

#[test]
#[should_panic(expected = "assertion failed: quality >= 0")]
fn test_set_with_negative_quality() {
    let mut table = TranspositionTable::new(100);
    table.set(12345, 1.0, -1);
}

#[test]
fn test_large_table() {
    let mut table = TranspositionTable::new(10000);

    // Add many entries
    for i in 1..=1000 {
        table.update(i, (i as f32 * 0.1, (i % 20) as i16));
    }

    // Check some entries exist
    assert_eq!(table.check(1, -1), Some((0.1, 1)));
    assert_eq!(table.check(500, -1), Some((50.0, 0)));
    assert_eq!(table.check(1000, -1), Some((100.0, 0)));
}

#[test]
fn test_u64_max_fingerprint() {
    let mut table = TranspositionTable::new(100);

    // u64::MAX should work fine with HashMap
    table.update(u64::MAX, (1.5, 5));
    assert_eq!(table.check(u64::MAX, -1), Some((1.5, 5)));
}

#[test]
fn test_fingerprint_zero() {
    let mut table = TranspositionTable::new(100);

    // Fingerprint 0 is a valid key
    table.update(0, (1.0, 3));
    assert_eq!(table.check(0, -1), Some((1.0, 3)));

    // Other entries are unaffected
    table.update(1, (2.0, 4));
    assert_eq!(table.check(0, -1), Some((1.0, 3)));
    assert_eq!(table.check(1, -1), Some((2.0, 4)));
}

#[test]
fn test_grows_beyond_initial_capacity() {
    let mut table = TranspositionTable::new(10);

    // Insert more entries than the initial capacity
    for i in 0..100u64 {
        table.update(i, (i as f32, 1));
    }

    // All entries should be retained
    for i in 0..100u64 {
        assert_eq!(table.check(i, -1), Some((i as f32, 1)));
    }
}

#[test]
fn test_no_eviction_from_collisions() {
    let mut table = TranspositionTable::new(10);

    // All distinct fingerprints should coexist regardless of capacity
    let fingerprints: Vec<u64> = (0..50).collect();
    for &fp in &fingerprints {
        table.update(fp, (fp as f32, 1));
    }

    for &fp in &fingerprints {
        assert_eq!(table.check(fp, -1), Some((fp as f32, 1)));
    }
}
