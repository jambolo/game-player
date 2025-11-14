//! Transposition Table Module
//!
//! This module implements a transposition table, which is a map of game state values referenced by the states' fingerprints.

/// A map of game state values referenced by the states' fingerprints.
///
/// A game state can be the result of different sequences of the same (or a different) set of moves. This technique is used to
/// cache the value of a game state regardless of the moves used to reach it, thus the name "transposition" table. The purpose of
/// the "transposition" table has been extended to become simply a cache of game state values, so it is more aptly named "game
/// state value cache" -- but the old name persists.
///
/// As a speed and memory optimization in this implementation, slots in the table are not unique to the state being stored, and a
/// value may be overwritten when a new value is added. A value is overwritten only when its "quality" is less than or equal to the
/// "quality" of the value being added.
///
/// # Note
/// The fingerprint is assumed to be a random and uniformly distributed 64-bit value. It is assumed to never be u64::MAX.
///
/// # Examples
///
/// ```rust
/// # use game_player::transposition_table::TranspositionTable;
/// let mut table = TranspositionTable::new(1000, 100);
///
/// // Store a value
/// table.update(12345, 0.75, 5);
///
/// // Retrieve the value
/// if let Some((value, quality)) = table.check(12345, 0) {
///     assert_eq!(value, 0.75);
///     assert_eq!(quality, 5);
/// }
/// ```
pub struct TranspositionTable {
    /// The table of entries
    table: Vec<Entry>,
    /// The maximum age of entries allowed in the table
    max_age: i16,
}

// Table entry
//
// A note about age and quality: There are expected to be collisions in the table, so the quality is used to determine if a new
// entry should replace an existing one. Now, an entry that has not been referenced for a while will probably never be
// referenced again, so it should eventually be allowed to be replaced by a newer entry, regardless of the quality of the new
// entry.
#[derive(Clone)]
#[repr(C, packed)] // 16 bytes
struct Entry {
    fingerprint: u64, // The state's fingerprint
    value: f32,       // The state's value
    q: i16,           // The quality of the value
    age: i16,         // The number of turns since the entry has been referenced
}

impl Entry {
    const UNUSED: u64 = u64::MAX;

    fn clear(&mut self) {
        self.fingerprint = Self::UNUSED;
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            fingerprint: Self::UNUSED,
            value: 0.0,
            q: 0,
            age: 0,
        }
    }
}

// Check that the size of Entry is 16 bytes. The size is not required to be 16 bytes, but 16 bytes is an optimal size.
static_assertions::assert_eq_size!(f32, [u8; 4]); // float should be 32 bits
static_assertions::assert_eq_size!(Entry, [u8; 16]); // Entry should be 16 bytes

impl TranspositionTable {
    /// Creates a new TranspositionTable
    ///
    /// # Arguments
    /// * `size` - Number of entries in the table
    /// * `max_age` - Maximum age of entries allowed in the table
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let table = TranspositionTable::new(1000, 50);
    /// // Table is ready to use with 1000 entries and max age of 50
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0 or `max_age` is 0 or negative.
    ///
    /// ```should_panic
    /// # use game_player::transposition_table::TranspositionTable;
    /// let table = TranspositionTable::new(0, 50); // This will panic
    /// ```
    pub fn new(size: usize, max_age: i16) -> Self {
        assert!(size > 0);
        assert!(max_age > 0);
        Self {
            table: vec![Entry::default(); size],
            max_age,
        }
    }

    /// Returns the value and quality of a state if they are stored in the table and its quality is above the specified minimum (if
    /// specified). Otherwise, None is returned.
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be checked for
    /// * `min_q` - Minimum quality. If less than 0, it is not used.
    ///
    /// # Returns
    /// optional result as (value, quality)
    ///
    /// # Panics
    /// Panics if `fingerprint` is `u64::MAX`.
    ///
    /// # Side Effects
    /// * Resets the age of the entry to 0 if found.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100, 10);
    ///
    /// // Store a value with quality 5
    /// table.update(12345, 1.5, 5);
    ///
    /// // Check with no minimum quality
    /// assert_eq!(table.check(12345, -1), Some((1.5, 5)));
    ///
    /// // Check with minimum quality of 3 (should succeed)
    /// assert_eq!(table.check(12345, 3), Some((1.5, 5)));
    ///
    /// // Check with minimum quality of 10 (should fail)
    /// assert_eq!(table.check(12345, 10), None);
    ///
    /// // Check non-existent entry
    /// assert_eq!(table.check(99999, -1), None);
    /// ```
    pub fn check(&mut self, fingerprint: u64, min_q: i16) -> Option<(f32, i16)> {
        assert_ne!(fingerprint, Entry::UNUSED, "fingerprint != u64::MAX");

        // Find the entry
        let entry = self.find(fingerprint);
        if entry.fingerprint != fingerprint {
            return None; // Not found
        }

        // The entry was accessed so reset its age
        entry.age = 0;

        // Check the quality if min_q >= 0
        if min_q >= 0 && entry.q < min_q {
            return None; // Insufficient quality
        }

        Some((entry.value, entry.q))
    }

    /// Updates (or adds) an entry in the table if its quality is greater than or equal to the existing entry's quality
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be stored
    /// * `value` - Value to be stored
    /// * `quality` - Quality of the value
    ///
    /// # Panics
    /// Panics if `fingerprint` is `u64::MAX`.
    /// Panics if `quality` is negative.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100, 10);
    ///
    /// // Add a new entry
    /// table.update(12345, 1.0, 5);
    /// assert_eq!(table.check(12345, -1), Some((1.0, 5)));
    ///
    /// // Try to update with lower quality (should not replace)
    /// table.update(12345, 2.0, 3);
    /// assert_eq!(table.check(12345, -1), Some((1.0, 5))); // Original value
    ///
    /// // Update with higher quality (should replace)
    /// table.update(12345, 2.0, 7);
    /// assert_eq!(table.check(12345, -1), Some((2.0, 7))); // New value
    /// ```
    pub fn update(&mut self, fingerprint: u64, value: f32, quality: i16) {
        assert_ne!(fingerprint, Entry::UNUSED, "fingerprint != u64::MAX");
        assert!(quality >= 0);

        // Find the entry for the fingerprint
        let entry = self.find(fingerprint);
        let is_unused = entry.fingerprint == Entry::UNUSED;

        // If the entry is unused or if the new quality >= the stored quality, then store the new value. Note: It is assumed to be
        // better to replace values of equal quality in order to dispose of old entries that are less likely to be relevant.

        if is_unused || quality >= entry.q {
            *entry = Entry {
                fingerprint,
                value,
                q: quality,
                age: 0,
            };
        }
    }

    /// Sets an entry in the table.
    ///
    /// This method adds or updates an entry in the table, regardless of its quality.
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be stored
    /// * `value` - Value to be stored
    /// * `quality` - Quality of the value
    ///
    /// # Panics
    /// Panics if `fingerprint` is `u64::MAX`.
    /// Panics if `quality` is negative.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100, 10);
    ///
    /// // Set an entry
    /// table.set(12345, 1.5, 5);
    /// assert_eq!(table.check(12345, -1), Some((1.5, 5)));
    ///
    /// // Set again with lower quality (should still replace)
    /// table.set(12345, 2.5, 3);
    /// assert_eq!(table.check(12345, -1), Some((2.5, 3)));
    /// ```
    pub fn set(&mut self, fingerprint: u64, value: f32, quality: i16) {
        assert_ne!(fingerprint, Entry::UNUSED, "fingerprint != u64::MAX");
        assert!(quality >= 0);

        // Find the entry for the fingerprint
        let entry = self.find(fingerprint);

        // Store the state, value and quality
        *entry = Entry {
            fingerprint,
            value,
            q: quality,
            age: 0,
        };
    }

    /// The T-table is persistent. So in order to gradually dispose of entries that are no longer relevant, entries that have not
    /// been referenced for a while are removed.
    ///
    /// This method increments the age of all entries and removes entries that exceed the maximum age.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100, 2); // max_age = 2
    ///
    /// // Add an entry
    /// table.set(12345, 1.0, 5);
    /// assert_eq!(table.check(12345, -1), Some((1.0, 5)));
    ///
    /// // Age the table once - entry should still be there
    /// table.age();
    /// assert_eq!(table.check(12345, -1), Some((1.0, 5))); // Age reset by check
    ///
    /// // Age twice more without accessing - entry should be removed
    /// table.age();
    /// table.age();
    /// table.age();
    /// assert_eq!(table.check(12345, -1), None); // Entry aged out
    /// ```
    pub fn age(&mut self) {
        self.table
            .iter_mut()
            .filter(|entry| entry.fingerprint != Entry::UNUSED)
            .for_each(|entry| {
                entry.age += 1;
                if entry.age > self.max_age {
                    entry.clear();
                }
            });
    }

    // Find the entry slot for a fingerprint using simple modulo hashing
    fn find(&mut self, hash: u64) -> &mut Entry {
        let i = (hash as usize) % self.table.len();
        &mut self.table[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_parameters() {
        let table = TranspositionTable::new(100, 10);
        assert_eq!(table.table.len(), 100);
        assert_eq!(table.max_age, 10);
    }

    #[test]
    #[should_panic(expected = "assertion failed: size > 0")]
    fn test_new_zero_size_panics() {
        TranspositionTable::new(0, 10);
    }

    #[test]
    #[should_panic(expected = "assertion failed: max_age > 0")]
    fn test_new_zero_max_age_panics() {
        TranspositionTable::new(100, 0);
    }

    #[test]
    #[should_panic(expected = "assertion failed: max_age > 0")]
    fn test_new_negative_max_age_panics() {
        TranspositionTable::new(100, -1);
    }

    #[test]
    fn test_check_nonexistent_entry() {
        let mut table = TranspositionTable::new(100, 10);
        assert_eq!(table.check(12345, -1), None);
        assert_eq!(table.check(12345, 0), None);
        assert_eq!(table.check(12345, 5), None);
    }

    #[test]
    fn test_update_and_check_basic() {
        let mut table = TranspositionTable::new(100, 10);

        // Add an entry
        table.update(12345, 1.5, 5);

        // Check it's there
        assert_eq!(table.check(12345, -1), Some((1.5, 5)));
        assert_eq!(table.check(12345, 0), Some((1.5, 5)));
        assert_eq!(table.check(12345, 5), Some((1.5, 5)));
    }

    #[test]
    fn test_check_minimum_quality() {
        let mut table = TranspositionTable::new(100, 10);

        table.update(12345, 2.0, 5);

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
        let mut table = TranspositionTable::new(100, 10);

        // Add initial entry
        table.update(12345, 1.0, 5);
        assert_eq!(table.check(12345, -1), Some((1.0, 5)));

        // Try to update with lower quality (should not replace)
        table.update(12345, 2.0, 3);
        assert_eq!(table.check(12345, -1), Some((1.0, 5))); // Original value

        // Update with equal quality (should replace)
        table.update(12345, 3.0, 5);
        assert_eq!(table.check(12345, -1), Some((3.0, 5))); // New value

        // Update with higher quality (should replace)
        table.update(12345, 4.0, 7);
        assert_eq!(table.check(12345, -1), Some((4.0, 7))); // New value
    }

    #[test]
    fn test_set_always_replaces() {
        let mut table = TranspositionTable::new(100, 10);

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
    fn test_age_resets_on_check() {
        let mut table = TranspositionTable::new(100, 2);

        // Add an entry
        table.set(12345, 1.0, 5);

        // Age once
        table.age();

        // Check (should reset age)
        assert_eq!(table.check(12345, -1), Some((1.0, 5)));
    }

    #[test]
    fn test_age_removes_old_entries() {
        let mut table = TranspositionTable::new(100, 2);

        // Add an entry
        table.set(12345, 1.0, 5);
        assert_eq!(table.check(12345, -1), Some((1.0, 5)));

        // Age beyond max_age without accessing
        table.age(); // age = 1
        table.age(); // age = 2
        table.age(); // age = 3, should be removed (> max_age = 2)

        // Entry should be gone
        assert_eq!(table.check(12345, -1), None);
    }

    #[test]
    fn test_multiple_entries() {
        let mut table = TranspositionTable::new(100, 10);

        // Add multiple entries
        table.update(1, 1.0, 1);
        table.update(2, 2.0, 2);
        table.update(3, 3.0, 3);

        // Check all entries exist
        assert_eq!(table.check(1, -1), Some((1.0, 1)));
        assert_eq!(table.check(2, -1), Some((2.0, 2)));
        assert_eq!(table.check(3, -1), Some((3.0, 3)));

        // Check non-existent entry
        assert_eq!(table.check(4, -1), None);
    }

    #[test]
    fn test_hash_collision_handling() {
        // Create a small table to force collisions
        let mut table = TranspositionTable::new(1, 10); // Only 1 slot

        // Add first entry
        table.update(1, 1.0, 5);
        assert_eq!(table.check(1, -1), Some((1.0, 5)));

        // Add second entry that will hash to same slot
        // Since we have quality-based replacement, this should replace
        table.update(2, 2.0, 5);
        assert_eq!(table.check(2, -1), Some((2.0, 5)));
        assert_eq!(table.check(1, -1), None); // First entry should be gone
    }

    #[test]
    fn test_floating_point_values() {
        let mut table = TranspositionTable::new(100, 10);

        // Test various floating point values
        let test_values = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            std::f32::consts::PI,
            -std::f32::consts::E,
            f32::MIN,
            f32::MAX,
            f32::EPSILON,
            -f32::EPSILON,
        ];

        for (i, &value) in test_values.iter().enumerate() {
            let fingerprint = (i + 1) as u64;
            table.update(fingerprint, value, 1);
            assert_eq!(table.check(fingerprint, -1), Some((value, 1)));
        }
    }

    #[test]
    fn test_edge_case_qualities() {
        let mut table = TranspositionTable::new(100, 10);

        // Test with quality 0
        table.update(1, 1.0, 0);
        assert_eq!(table.check(1, -1), Some((1.0, 0)));
        assert_eq!(table.check(1, 0), Some((1.0, 0)));
        assert_eq!(table.check(1, 1), None);

        // Test with high quality
        table.update(2, 2.0, i16::MAX);
        assert_eq!(table.check(2, -1), Some((2.0, i16::MAX)));
        assert_eq!(table.check(2, i16::MAX), Some((2.0, i16::MAX)));
    }

    #[test]
    #[should_panic(expected = "assertion `left != right` failed: fingerprint != u64::MAX")]
    fn test_check_with_invalid_fingerprint() {
        let mut table = TranspositionTable::new(100, 10);
        table.check(u64::MAX, -1);
    }

    #[test]
    #[should_panic(expected = "assertion `left != right` failed: fingerprint != u64::MAX")]
    fn test_update_with_invalid_fingerprint() {
        let mut table = TranspositionTable::new(100, 10);
        table.update(u64::MAX, 1.0, 5);
    }

    #[test]
    #[should_panic(expected = "assertion failed: quality >= 0")]
    fn test_update_with_negative_quality() {
        let mut table = TranspositionTable::new(100, 10);
        table.update(12345, 1.0, -1);
    }

    #[test]
    #[should_panic(expected = "assertion `left != right` failed: fingerprint != u64::MAX")]
    fn test_set_with_invalid_fingerprint() {
        let mut table = TranspositionTable::new(100, 10);
        table.set(u64::MAX, 1.0, 5);
    }

    #[test]
    #[should_panic(expected = "assertion failed: quality >= 0")]
    fn test_set_with_negative_quality() {
        let mut table = TranspositionTable::new(100, 10);
        table.set(12345, 1.0, -1);
    }

    #[test]
    fn test_large_table() {
        let mut table = TranspositionTable::new(10000, 100);

        // Add many entries
        for i in 1..=1000 {
            table.update(i, i as f32 * 0.1, (i % 20) as i16);
        }

        // Check some entries exist
        assert_eq!(table.check(1, -1), Some((0.1, 1)));
        assert_eq!(table.check(500, -1), Some((50.0, 0)));
        assert_eq!(table.check(1000, -1), Some((100.0, 0)));
    }

    #[test]
    fn test_aging_multiple_entries() {
        let mut table = TranspositionTable::new(100, 3);

        // Add multiple entries
        table.set(1, 1.0, 1);
        table.set(2, 2.0, 2);
        table.set(3, 3.0, 3);

        // Age twice
        table.age();
        table.age();

        // Access one entry to reset its age
        assert_eq!(table.check(2, -1), Some((2.0, 2)));

        // Age beyond max_age
        table.age();
        table.age();

        // Only the accessed entry should remain
        assert_eq!(table.check(1, -1), None);
        assert_eq!(table.check(2, -1), Some((2.0, 2)));
        assert_eq!(table.check(3, -1), None);
    }
}
