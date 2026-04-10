use game_player::transposition_table::TranspositionTable;

#[test]
fn test_check_nonexistent_entry() {
    let mut table = TranspositionTable::new(100);
    assert_eq!(table.check(12345), None);
    assert_eq!(table.check_min_quality(12345, Some(0)), None);
    assert_eq!(table.check_min_quality(12345, Some(5)), None);
}

#[test]
fn test_update_and_check_basic() {
    let mut table = TranspositionTable::new(100);

    // Add an entry
    table.update(12345, 1.5, 5);

    // Check it's there
    assert_eq!(table.check(12345), Some((1.5, 5)));
    assert_eq!(table.check_min_quality(12345, Some(0)), Some((1.5, 5)));
    assert_eq!(table.check_min_quality(12345, Some(5)), Some((1.5, 5)));
}

#[test]
fn test_check_minimum_quality() {
    let mut table = TranspositionTable::new(100);

    table.update(12345, 2.0, 5);

    // Check with various minimum qualities
    assert_eq!(table.check(12345), Some((2.0, 5))); // No minimum
    assert_eq!(table.check_min_quality(12345, Some(0)), Some((2.0, 5))); // Below stored quality
    assert_eq!(table.check_min_quality(12345, Some(3)), Some((2.0, 5))); // Below stored quality
    assert_eq!(table.check_min_quality(12345, Some(5)), Some((2.0, 5))); // Equal to stored quality
    assert_eq!(table.check_min_quality(12345, Some(6)), None); // Above stored quality
    assert_eq!(table.check_min_quality(12345, Some(10)), None); // Well above stored quality
}

#[test]
fn test_update_quality_replacement_rules() {
    let mut table = TranspositionTable::new(100);

    // Add initial entry
    table.update(12345, 1.0, 5);
    assert_eq!(table.check(12345), Some((1.0, 5)));

    // Try to update with lower quality (should not replace)
    table.update(12345, 2.0, 3);
    assert_eq!(table.check(12345), Some((1.0, 5))); // Original value

    // Update with equal quality (should replace)
    table.update(12345, 3.0, 5);
    assert_eq!(table.check(12345), Some((3.0, 5))); // New value

    // Update with higher quality (should replace)
    table.update(12345, 4.0, 7);
    assert_eq!(table.check(12345), Some((4.0, 7))); // New value
}

#[test]
fn test_set_always_replaces() {
    let mut table = TranspositionTable::new(100);

    // Add initial entry
    table.set(12345, 1.0, 5);
    assert_eq!(table.check(12345), Some((1.0, 5)));

    // Set with lower quality (should still replace)
    table.set(12345, 2.0, 3);
    assert_eq!(table.check(12345), Some((2.0, 3)));

    // Set with higher quality (should replace)
    table.set(12345, 3.0, 7);
    assert_eq!(table.check(12345), Some((3.0, 7)));
}

#[test]
fn test_multiple_entries() {
    let mut table = TranspositionTable::new(100);

    // Add multiple entries
    table.update(1, 1.0, 1);
    table.update(2, 2.0, 2);
    table.update(3, 3.0, 3);

    // Check all entries exist
    assert_eq!(table.check(1), Some((1.0, 1)));
    assert_eq!(table.check(2), Some((2.0, 2)));
    assert_eq!(table.check(3), Some((3.0, 3)));

    // Check non-existent entry
    assert_eq!(table.check(4), None);
}

#[test]
fn test_hash_collision_handling() {
    // With HashMap, different fingerprints are stored independently
    let mut table = TranspositionTable::new(100);

    // Add first entry
    table.update(1, 1.0, 5);
    assert_eq!(table.check(1), Some((1.0, 5)));

    // Add second entry with different fingerprint
    // Both should coexist since HashMap handles them separately
    table.update(2, 2.0, 5);
    assert_eq!(table.check(2), Some((2.0, 5)));
    assert_eq!(table.check(1), Some((1.0, 5))); // First entry still there
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
        table.update(fingerprint, value, 1);
        assert_eq!(table.check(fingerprint), Some((value, 1)));
    }
}

#[test]
fn test_edge_case_qualities() {
    let mut table = TranspositionTable::new(100);

    // Test with quality 0
    table.update(1, 1.0, 0);
    assert_eq!(table.check(1), Some((1.0, 0)));
    assert_eq!(table.check_min_quality(1, Some(0)), Some((1.0, 0)));
    assert_eq!(table.check_min_quality(1, Some(1)), None);

    // Test with high quality
    table.update(2, 2.0, 32767);
    assert_eq!(table.check(2), Some((2.0, 32767)));
    assert_eq!(table.check_min_quality(2, Some(32767)), Some((2.0, 32767)));
}

#[test]
fn test_large_table() {
    let mut table = TranspositionTable::new(10000);

    // Add many entries
    for i in 1..=1000 {
        table.update(i, i as f32 * 0.1, (i % 20) as u32);
    }

    // Check some entries exist
    assert_eq!(table.check(1), Some((0.1, 1)));
    assert_eq!(table.check(500), Some((50.0, 0)));
    assert_eq!(table.check(1000), Some((100.0, 0)));
}

#[test]
fn test_u64_max_fingerprint() {
    let mut table = TranspositionTable::new(100);

    // u64::MAX should work fine with HashMap
    table.update(u64::MAX, 1.5, 5);
    assert_eq!(table.check(u64::MAX), Some((1.5, 5)));
}

#[test]
fn test_fingerprint_zero() {
    let mut table = TranspositionTable::new(100);

    // Fingerprint 0 is a valid key
    table.update(0, 1.0, 3);
    assert_eq!(table.check(0), Some((1.0, 3)));

    // Other entries are unaffected
    table.update(1, 2.0, 4);
    assert_eq!(table.check(0), Some((1.0, 3)));
    assert_eq!(table.check(1), Some((2.0, 4)));
}

#[test]
fn test_grows_beyond_initial_capacity() {
    let mut table = TranspositionTable::new(10);

    // Insert more entries than the initial capacity
    for i in 0..100u64 {
        table.update(i, i as f32, 1);
    }

    // All entries should be retained
    for i in 0..100u64 {
        assert_eq!(table.check(i), Some((i as f32, 1)));
    }
}

#[test]
fn test_set_then_update_respects_quality() {
    let mut table = TranspositionTable::new(100);

    // Set an entry via set()
    table.set(12345, 1.0, 5);
    assert_eq!(table.check(12345), Some((1.0, 5)));

    // update() with lower quality should not replace
    table.update(12345, 2.0, 3);
    assert_eq!(table.check(12345), Some((1.0, 5)));

    // update() with equal quality should replace
    table.update(12345, 3.0, 5);
    assert_eq!(table.check(12345), Some((3.0, 5)));

    // update() with higher quality should replace
    table.update(12345, 4.0, 7);
    assert_eq!(table.check(12345), Some((4.0, 7)));
}

#[test]
fn test_infinity_values() {
    let mut table = TranspositionTable::new(100);

    // Infinities are natural win/loss sentinels in minimax search
    table.update(1, f32::INFINITY, 5);
    assert_eq!(table.check(1), Some((f32::INFINITY, 5)));

    table.update(2, f32::NEG_INFINITY, 5);
    assert_eq!(table.check(2), Some((f32::NEG_INFINITY, 5)));
}

#[test]
fn test_zero_initial_capacity() {
    let mut table = TranspositionTable::new(0);

    // Table should still work with zero initial capacity hint
    table.update(1, 1.0, 5);
    assert_eq!(table.check(1), Some((1.0, 5)));
}

#[test]
fn test_no_eviction_from_collisions() {
    let mut table = TranspositionTable::new(10);

    // All distinct fingerprints should coexist regardless of capacity
    let fingerprints: Vec<u64> = (0..50).collect();
    for &fp in &fingerprints {
        table.update(fp, fp as f32, 1);
    }

    for &fp in &fingerprints {
        assert_eq!(table.check(fp), Some((fp as f32, 1)));
    }
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_initial_state() {
    let table = TranspositionTable::new(100);
    let stats = table.stats();
    assert_eq!(stats.hit_rate, None);
    assert_eq!(stats.collision_count, 0);
    assert_eq!(stats.fill_factor, 0.0);
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_hit_rate() {
    let mut table = TranspositionTable::new(100);
    table.update(1, 1.0, 3);
    table.update(2, 2.0, 3);

    table.check(1); // hit
    table.check(2); // hit
    table.check(3); // miss
    table.check(4); // miss

    let stats = table.stats();
    assert_eq!(stats.hit_rate, Some(0.5));
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_hit_rate_all_hits() {
    let mut table = TranspositionTable::new(100);
    table.update(42, 7.0, 5);
    table.check(42);
    table.check(42);

    let stats = table.stats();
    assert_eq!(stats.hit_rate, Some(1.0));
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_hit_rate_all_misses() {
    let mut table = TranspositionTable::new(100);
    table.check(1);
    table.check(2);

    let stats = table.stats();
    assert_eq!(stats.hit_rate, Some(0.0));
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_quality_filtered_check_counts_as_miss() {
    let mut table = TranspositionTable::new(100);
    table.update(1, 1.0, 3);

    table.check_min_quality(1, Some(10)); // fingerprint found but quality too low — miss

    let stats = table.stats();
    assert_eq!(stats.hit_rate, Some(0.0));
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_collision_count() {
    let mut table = TranspositionTable::new(100);

    table.update(1, 1.0, 3); // new entry — no collision
    table.update(2, 2.0, 3); // new entry — no collision
    table.update(1, 1.5, 5); // fingerprint 1 already present — collision
    table.update(1, 0.5, 1); // fingerprint 1 already present — collision

    let stats = table.stats();
    assert_eq!(stats.collision_count, 2);
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_fill_factor() {
    let mut table = TranspositionTable::new(100);
    assert_eq!(table.stats().fill_factor, 0.0);

    table.update(1, 1.0, 1);
    table.update(2, 2.0, 2);
    table.update(3, 3.0, 3);

    let stats = table.stats();
    // 3 distinct entries; capacity is at least 100, so fill factor is in (0, 0.03]
    assert!(stats.fill_factor > 0.0);
    assert!(stats.fill_factor <= 0.03 + f32::EPSILON);
}

#[cfg(feature = "analysis_transposition_table")]
#[test]
fn test_stats_fill_factor_grows_with_entries() {
    let mut table = TranspositionTable::new(100);
    let before = table.stats().fill_factor;

    for i in 0..50u64 {
        table.update(i, i as f32, 1);
    }

    assert!(table.stats().fill_factor > before);
    assert!(table.stats().fill_factor <= 1.0);
}
