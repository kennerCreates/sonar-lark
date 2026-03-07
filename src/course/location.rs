use serde::{Deserialize, Serialize};

use crate::course::data::CourseData;
use crate::obstacle::definition::ObstacleId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub base_attractiveness: f32,
    pub capacity: u32,
    pub rental_fee: f32,
}

impl Location {
    /// Deterministic slug for file paths: lowercase, spaces → underscores.
    pub fn slug(&self) -> String {
        self.name.to_lowercase().replace(' ', "_")
    }

    /// Save path for this location's course data.
    pub fn save_path(&self) -> String {
        format!("assets/locations/{}.location.ron", self.slug())
    }
}

pub fn default_locations() -> Vec<Location> {
    vec![
        Location {
            name: "Local Park".to_string(),
            base_attractiveness: 0.2,
            capacity: 40,
            rental_fee: 5.0,
        },
        Location {
            name: "Abandoned Warehouse".to_string(),
            base_attractiveness: 0.4,
            capacity: 80,
            rental_fee: 0.0,
        },
        Location {
            name: "Golf Course".to_string(),
            base_attractiveness: 0.7,
            capacity: 200,
            rental_fee: 50.0,
        },
    ]
}

/// Tracks which location the player selected from the menu.
#[derive(Clone, Copy, Debug, bevy::prelude::Resource)]
pub struct SelectedLocation(pub usize);

/// Per-location gate inventory: gates the player owns but has stored (not placed).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GateInventory {
    pub entries: Vec<GateInventoryEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateInventoryEntry {
    pub obstacle_id: ObstacleId,
    pub count: u32,
}

impl GateInventory {
    /// Number of stored gates of this type.
    pub fn count(&self, obstacle_id: &ObstacleId) -> u32 {
        self.entries
            .iter()
            .find(|e| e.obstacle_id == *obstacle_id)
            .map(|e| e.count)
            .unwrap_or(0)
    }

    /// Add a gate to inventory (stored, not placed).
    pub fn add(&mut self, obstacle_id: &ObstacleId) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.obstacle_id == *obstacle_id) {
            entry.count += 1;
        } else {
            self.entries.push(GateInventoryEntry {
                obstacle_id: obstacle_id.clone(),
                count: 1,
            });
        }
    }

    /// Remove one gate from inventory. Returns true if successful.
    pub fn remove(&mut self, obstacle_id: &ObstacleId) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.obstacle_id == *obstacle_id)
            && entry.count > 0
        {
            entry.count -= 1;
            return true;
        }
        false
    }
}

/// On-disk format for a location save file. Wraps CourseData + gate inventory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocationSaveData {
    pub course: CourseData,
    #[serde(default)]
    pub inventory: GateInventory,
}

#[derive(Clone, Debug, Default, bevy::prelude::Resource)]
pub struct LocationRegistry {
    pub locations: Vec<Location>,
}

impl LocationRegistry {
    pub fn new() -> Self {
        Self {
            locations: default_locations(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Location> {
        self.locations.iter().find(|l| l.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_roundtrip() {
        let location = Location {
            name: "Test Arena".to_string(),
            base_attractiveness: 0.5,
            capacity: 100,
            rental_fee: 10.0,
        };
        let serialized = ron::to_string(&location).unwrap();
        let deserialized: Location = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "Test Arena");
        assert!((deserialized.base_attractiveness - 0.5).abs() < f32::EPSILON);
        assert_eq!(deserialized.capacity, 100);
    }

    #[test]
    fn test_registry_lookup() {
        let registry = LocationRegistry::new();
        let found = registry.get("Local Park");
        assert!(found.is_some());
        let park = found.unwrap();
        assert!((park.base_attractiveness - 0.2).abs() < f32::EPSILON);
        assert_eq!(park.capacity, 40);

        assert!(registry.get("Nonexistent Venue").is_none());
    }

    #[test]
    fn test_default_locations_count() {
        assert_eq!(default_locations().len(), 3);
    }
}
