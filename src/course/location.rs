use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub base_attractiveness: f32,
    pub capacity: u32,
}

pub fn default_locations() -> Vec<Location> {
    vec![
        Location {
            name: "Local Park".to_string(),
            base_attractiveness: 0.2,
            capacity: 40,
        },
        Location {
            name: "Abandoned Warehouse".to_string(),
            base_attractiveness: 0.4,
            capacity: 80,
        },
        Location {
            name: "Golf Course".to_string(),
            base_attractiveness: 0.7,
            capacity: 200,
        },
    ]
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
        assert!((park.base_attractiveness - 0.4).abs() < f32::EPSILON);
        assert_eq!(park.capacity, 80);

        assert!(registry.get("Nonexistent Venue").is_none());
    }

    #[test]
    fn test_default_locations_count() {
        assert_eq!(default_locations().len(), 3);
    }
}
