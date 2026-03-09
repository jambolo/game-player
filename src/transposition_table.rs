//! Transposition Table

use std::collections::HashMap;

/// A map of game state values referenced by the states' fingerprints.
///
/// A game state can be the result of different sequences of the same (or a different) set of moves. This technique is used to cache
/// the value of a game state regardless of the moves used to reach it, thus the name "transposition" table. The purpose of the
/// "transposition" table has been extended to become simply a cache of game state values, so it is more aptly named "game state
/// value cache" -- but the old name persists.
///
/// # Note
/// The fingerprint is assumed to be a random and uniformly distributed 64-bit value.
///
/// # Examples
///
/// ```rust
/// # use game_player::transposition_table::TranspositionTable;
/// let mut table = TranspositionTable::new(1000);
///
/// // Store a value
/// table.update(12345, (0.75, 5));
///
/// // Retrieve the value
/// if let Some((value, quality)) = table.check(12345, 0) {
///     assert_eq!(value, 0.75);
///     assert_eq!(quality, 5);
/// }
/// ```
pub struct TranspositionTable {
    /// The table of entries
    table: HashMap<u64, Entry>,
}

// Table entry
#[derive(Clone)]
struct Entry {
    value: f32, // The state's value
    q: i16,     // The quality of the value, literally the number of plies above the leaf node
}

impl Entry {
    fn new(value: f32, q: i16) -> Self {
        Self { value, q }
    }
}

impl TranspositionTable {
    /// Creates a new TranspositionTable
    ///
    /// # Arguments
    /// * `size` - Capacity hint for the underlying HashMap
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let table = TranspositionTable::new(1000);
    /// // Table is ready to use with initial capacity of 1000
    /// ```
    pub fn new(size: usize) -> Self {
        Self {
            table: HashMap::with_capacity(size),
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
    /// Optional result as (value, quality)
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
    ///
    /// // Store a value with quality 5
    /// table.update(12345, (1.5, 5));
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
    pub fn check(&self, fingerprint: u64, min_q: i16) -> Option<(f32, i16)> {
        self.table.get(&fingerprint).and_then(|entry| {
            // Check the quality if min_q >= 0
            if min_q >= 0 && entry.q < min_q {
                None
            } else {
                Some((entry.value, entry.q))
            }
        })
    }

    /// Updates (or adds) an entry in the table if its quality is greater than or equal to the existing entry's quality
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be stored
    /// * `entry` - Tuple of (value, quality)
    ///
    /// # Panics
    /// Panics if `quality` is negative.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
    ///
    /// // Add a new entry
    /// table.update(12345, (1.0, 5));
    /// assert_eq!(table.check(12345, -1), Some((1.0, 5)));
    ///
    /// // Try to update with lower quality (should not replace)
    /// table.update(12345, (2.0, 3));
    /// assert_eq!(table.check(12345, -1), Some((1.0, 5))); // Original value
    ///
    /// // Update with higher quality (should replace)
    /// table.update(12345, (2.0, 7));
    /// assert_eq!(table.check(12345, -1), Some((2.0, 7))); // New value
    /// ```
    pub fn update(&mut self, fingerprint: u64, (value, quality): (f32, i16)) {
        assert!(quality >= 0);

        self.table
            .entry(fingerprint)
            .and_modify(|entry| {
                // If new quality >= existing quality, replace the entry
                if quality >= entry.q {
                    *entry = Entry::new(value, quality);
                }
            })
            .or_insert_with(|| Entry::new(value, quality));
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
    /// Panics if `quality` is negative.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
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
        assert!(quality >= 0);

        self.table.insert(fingerprint, Entry::new(value, quality));
    }
}
