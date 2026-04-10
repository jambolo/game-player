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
/// table.update(12345, 0.75, 5);
///
/// // Retrieve the value
/// if let Some((value, quality)) = table.check(12345) {
///     assert_eq!(value, 0.75);
///     assert_eq!(quality, 5);
/// }
/// ```
/// Statistics from a [`TranspositionTable`].
#[cfg(feature = "analysis_transposition_table")]
pub struct TranspositionTableStats {
    /// Fraction of `check()` calls that returned a hit (0.0–1.0). `None` if no checks have been made.
    pub hit_rate: Option<f32>,
    /// Fraction of the table's capacity that is occupied (0.0–1.0).
    pub fill_factor: f32,
    /// Number of `update()` calls that encountered an already-occupied slot (i.e. a transposition or fingerprint collision).
    pub collision_count: u64,
}

pub struct TranspositionTable {
    /// The table of entries
    table: HashMap<u64, Entry>,
    /// Total number of `check()` calls
    #[cfg(feature = "analysis_transposition_table")]
    checks: u64,
    /// Number of `check()` calls that returned `Some`
    #[cfg(feature = "analysis_transposition_table")]
    hits: u64,
    /// Number of `update()` calls that found the fingerprint already present
    #[cfg(feature = "analysis_transposition_table")]
    collisions: u64,
}

// Table entry
#[derive(Clone)]
struct Entry {
    value: f32, // The state's value
    q: u32,     // The quality of the value, literally the number of plies above the leaf node
}

impl Entry {
    fn new(value: f32, q: u32) -> Self {
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
            #[cfg(feature = "analysis_transposition_table")]
            checks: 0,
            #[cfg(feature = "analysis_transposition_table")]
            hits: 0,
            #[cfg(feature = "analysis_transposition_table")]
            collisions: 0,
        }
    }

    /// Returns the value and quality of a state if they are stored in the table and its quality is above the specified minimum (if
    /// specified). Otherwise, None is returned.
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be checked for
    /// * `min_quality` - Minimum quality. If `None`, any quality is accepted. If `Some(n)`, only entries with
    ///   quality `>= n` are returned.
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
    /// table.update(12345, 1.5, 5);
    ///
    /// // Check with no minimum quality
    /// assert_eq!(table.check_min_quality(12345, None), Some((1.5, 5)));
    ///
    /// // Check with minimum quality of 3 (should succeed)
    /// assert_eq!(table.check_min_quality(12345, Some(3)), Some((1.5, 5)));
    ///
    /// // Check with minimum quality of 10 (should fail)
    /// assert_eq!(table.check_min_quality(12345, Some(10)), None);
    ///
    /// // Check non-existent entry
    /// assert_eq!(table.check_min_quality(99999, None), None);
    /// ```
    pub fn check_min_quality(&mut self, fingerprint: u64, min_quality: Option<u32>) -> Option<(f32, u32)> {
        #[cfg(feature = "analysis_transposition_table")]
        { self.checks += 1; }
        let result = self.table.get(&fingerprint).and_then(|entry| {
            if let Some(min) = min_quality && entry.q < min {
                return None;
            }
            Some((entry.value, entry.q))
        });
        #[cfg(feature = "analysis_transposition_table")]
        if result.is_some() {
            self.hits += 1;
        }
        result
    }

    /// Returns the value and quality of a state if it is stored in the table. Otherwise, `None` is returned.
    ///
    /// This is a convenience wrapper for [`check_min_quality`](Self::check_min_quality) with no minimum quality requirement.
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be checked for
    ///
    /// # Returns
    /// Optional result as (value, quality)
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
    ///
    /// table.update(12345, 1.5, 5);
    ///
    /// assert_eq!(table.check(12345), Some((1.5, 5)));
    /// assert_eq!(table.check(99999), None);
    /// ```
    pub fn check(&mut self, fingerprint: u64) -> Option<(f32, u32)> {
        self.check_min_quality(fingerprint, None)
    }

    /// Updates (or adds) an entry in the table if its quality is greater than or equal to the existing entry's quality
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be stored
    /// * `value` - Value to be stored
    /// * `quality` - Quality of the value
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
    ///
    /// // Add a new entry
    /// table.update(12345, 1.0, 5);
    /// assert_eq!(table.check(12345), Some((1.0, 5)));
    ///
    /// // Try to update with lower quality (should not replace)
    /// table.update(12345, 2.0, 3);
    /// assert_eq!(table.check(12345), Some((1.0, 5))); // Original value
    ///
    /// // Update with higher quality (should replace)
    /// table.update(12345, 2.0, 7);
    /// assert_eq!(table.check(12345), Some((2.0, 7))); // New value
    /// ```
    pub fn update(&mut self, fingerprint: u64, value: f32, quality: u32) {

        #[cfg(feature = "analysis_transposition_table")]
        {
            let occupied = self.table.contains_key(&fingerprint);
            if occupied {
                self.collisions += 1;
            }
        }
        self.table
            .entry(fingerprint)
            .and_modify(|entry| {
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
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
    ///
    /// // Set an entry
    /// table.set(12345, 1.5, 5);
    /// assert_eq!(table.check(12345), Some((1.5, 5)));
    ///
    /// // Set again with lower quality (should still replace)
    /// table.set(12345, 2.5, 3);
    /// assert_eq!(table.check(12345), Some((2.5, 3)));
    /// ```
    #[allow(dead_code)]
    pub fn set(&mut self, fingerprint: u64, value: f32, quality: u32) {
        self.table.insert(fingerprint, Entry::new(value, quality));
    }

    /// Returns a snapshot of table statistics.
    ///
    /// # Examples
    /// ```rust
    /// # use game_player::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new(100);
    /// table.update(1, 0.5, 3);
    /// table.update(1, 0.8, 5); // collision — fingerprint already present
    /// table.check(1);      // hit
    /// table.check(99);     // miss
    ///
    /// let stats = table.stats();
    /// assert_eq!(stats.hit_rate, Some(0.5));
    /// assert!(stats.fill_factor > 0.0);
    /// assert_eq!(stats.collision_count, 1);
    /// ```
    #[cfg(feature = "analysis_transposition_table")]
    pub fn stats(&self) -> TranspositionTableStats {
        let hit_rate = if self.checks > 0 {
            Some(self.hits as f32 / self.checks as f32)
        } else {
            None
        };
        let fill_factor = if self.table.capacity() > 0 {
            self.table.len() as f32 / self.table.capacity() as f32
        } else {
            0.0
        };
        TranspositionTableStats {
            hit_rate,
            fill_factor,
            collision_count: self.collisions,
        }
    }
}
