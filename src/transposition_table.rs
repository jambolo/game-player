//! Transposition table implementation

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
/// The fingerprint is assumed to be random and uniformly distributed. It is assumed to never be u64::MAX.
pub struct TranspositionTable {
    /// The table of entries
    table: Vec<Entry>,
    /// The maximum age of entries allowed in the table
    max_age: i16,
}

/// Result type returned by check().
pub struct CheckResult {
    /// Value of the state
    pub value: f32,
    /// Quality of the returned value
    pub quality: i16,
}

// A note about age and quality: There are expected to be collisions in the table, so the quality is used to determine if a new
// entry should replace an existing one. Now, an entry that has not been referenced for a while will probably never be
// referenced again, so it should eventually be allowed to be replaced by a newer entry, regardless of the quality of the new
// entry.
#[derive(Clone)]
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
    /// optional CheckResult
    pub fn check(&mut self, fingerprint: u64, min_q: i16) -> Option<CheckResult> {
        assert_ne!(fingerprint, Entry::UNUSED);

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

        Some(CheckResult {
            value: entry.value,
            quality: entry.q,
        })
    }

    /// Updates (or adds) an entry in the table if its quality is greater than or equal to the existing entry's quality
    ///
    /// # Arguments
    /// * `fingerprint` - Fingerprint of state to be stored
    /// * `value` - Value to be stored
    /// * `quality` - Quality of the value
    pub fn update(&mut self, fingerprint: u64, value: f32, quality: i16) {
        assert_ne!(fingerprint, Entry::UNUSED);
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
    pub fn set(&mut self, fingerprint: u64, value: f32, quality: i16) {
        assert_ne!(fingerprint, Entry::UNUSED);
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
    pub fn age(&mut self) {
        for entry in &mut self.table {
            if entry.fingerprint != Entry::UNUSED {
                entry.age += 1;
                if entry.age > self.max_age {
                    entry.clear();
                }
            }
        }
    }

    fn find(&mut self, hash: u64) -> &mut Entry {
        let i = (hash as usize) % self.table.len();
        &mut self.table[i]
    }
}
