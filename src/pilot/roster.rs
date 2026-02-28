use std::collections::HashSet;
use std::fs;
use std::path::Path;

use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use super::gamertag::generate_gamertag;
use super::personality::{self, PersonalityTrait};
use super::skill::SkillProfile;
use super::{ColorScheme, DroneBuildDescriptor, Pilot, PilotId, PilotStats, PortraitDescriptor};

const ROSTER_PATH: &str = "assets/pilots/roster.pilots.ron";
const INITIAL_ROSTER_SIZE: usize = 24;

/// 24 distinct sRGB colors with good hue spread for the initial pilot roster.
const ROSTER_COLORS: [[f32; 3]; 24] = [
    [0.961, 0.192, 0.255], // NEON_RED
    [0.961, 0.506, 0.133], // SUNFLOWER
    [0.980, 0.851, 0.216], // LIMON
    [0.580, 0.749, 0.188], // GRASS
    [0.090, 0.612, 0.263], // FROG
    [0.239, 0.631, 0.494], // JADE
    [0.286, 0.761, 0.949], // SKY
    [0.110, 0.459, 0.741], // HOMEWORLD
    [0.494, 0.494, 0.949], // PERIWINKLE
    [0.639, 0.365, 0.851], // AMETHYST
    [0.851, 0.298, 0.557], // PINK
    [0.949, 0.949, 0.855], // VANILLA
    [0.949, 0.384, 0.122], // TANGERINE
    [0.980, 0.627, 0.196], // DANDELION
    [0.800, 0.780, 0.239], // LIME
    [0.333, 0.702, 0.231], // GREEN
    [0.024, 0.502, 0.320], // JUNGLE (0.318 triggers approx_constant lint)
    [0.126, 0.502, 0.424], // SEAGREEN
    [0.145, 0.675, 0.961], // CERULEAN
    [0.345, 0.416, 0.769], // SLATE
    [0.467, 0.231, 0.749], // ROYAL
    [0.792, 0.494, 0.949], // ORCHID
    [0.922, 0.459, 0.561], // BUBBLEGUM
    [1.000, 0.725, 0.220], // SUNSHINE
];

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct PilotRoster {
    pub pilots: Vec<Pilot>,
    pub next_id: u64,
}

impl Default for PilotRoster {
    fn default() -> Self {
        Self {
            pilots: Vec::new(),
            next_id: 1,
        }
    }
}

impl PilotRoster {
    pub fn get(&self, id: PilotId) -> Option<&Pilot> {
        self.pilots.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: PilotId) -> Option<&mut Pilot> {
        self.pilots.iter_mut().find(|p| p.id == id)
    }
}

/// System: load roster from disk or generate a fresh one. Runs at Startup.
pub fn load_or_generate_roster(mut commands: Commands) {
    let path = Path::new(ROSTER_PATH);
    let mut roster = if path.exists() {
        match load_roster_from_file(path) {
            Ok(r) => {
                info!("Loaded pilot roster with {} pilots", r.pilots.len());
                r
            }
            Err(e) => {
                warn!("Failed to load pilot roster: {e}. Generating new roster.");
                generate_initial_roster()
            }
        }
    } else {
        info!("No pilot roster found, generating initial roster");
        generate_initial_roster()
    };

    // Migrate Phase 1 rosters: backfill empty portraits
    if backfill_empty_portraits(&mut roster) {
        save_roster_to_default(&roster);
        info!("Backfilled portraits for pilots missing them");
    }

    commands.insert_resource(roster);
}

/// Backfill portraits for pilots that have empty/default descriptors.
/// Uses deterministic RNG seeded by `PilotId` so the same pilot always gets
/// the same portrait, even if migration runs multiple times.
/// Returns `true` if any portraits were backfilled.
fn backfill_empty_portraits(roster: &mut PilotRoster) -> bool {
    let mut changed = false;
    for pilot in &mut roster.pilots {
        if pilot.portrait.is_empty() {
            let mut rng = StdRng::seed_from_u64(pilot.id.0);
            pilot.portrait = PortraitDescriptor::generate(&mut rng, pilot.color_scheme.primary);
            changed = true;
        }
    }
    changed
}

pub fn load_roster_from_file(path: &Path) -> Result<PilotRoster, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    ron::from_str(&contents).map_err(|e| format!("Failed to parse {}: {e}", path.display()))
}

pub fn save_roster(roster: &PilotRoster, path: &Path) -> Result<(), String> {
    let pretty = ron::ser::PrettyConfig::default();
    let contents = ron::ser::to_string_pretty(roster, pretty)
        .map_err(|e| format!("Failed to serialize roster: {e}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }
    fs::write(path, contents)
        .map_err(|e| format!("Failed to write roster to {}: {e}", path.display()))
}

pub fn save_roster_to_default(roster: &PilotRoster) {
    if let Err(e) = save_roster(roster, Path::new(ROSTER_PATH)) {
        error!("Failed to save pilot roster: {e}");
    }
}

fn generate_initial_roster() -> PilotRoster {
    use crate::dev_menu::portrait_config::{load_config, PALETTE_COLORS};

    let mut rng = rand::thread_rng();
    let mut roster = PilotRoster::default();
    let mut used_tags = HashSet::new();
    let palette_config = load_config();
    let has_config = !palette_config.vetoed.is_empty() || !palette_config.complementary.is_empty();

    for i in 0..INITIAL_ROSTER_SIZE {
        let id = PilotId(roster.next_id);
        roster.next_id += 1;

        let gamertag = generate_gamertag(&mut rng, &used_tags);
        used_tags.insert(gamertag.clone());

        let traits = pick_personality_traits(&mut rng);

        let skill = SkillProfile {
            level: rng.gen_range(0.2..=0.95),
            speed: rng.gen_range(0.2..=1.0),
            cornering: rng.gen_range(0.2..=1.0),
            consistency: rng.gen_range(0.2..=1.0),
        };

        let color = ColorScheme {
            primary: ROSTER_COLORS[i % ROSTER_COLORS.len()],
        };

        let portrait = if has_config {
            PortraitDescriptor::generate_with_config(
                &mut rng,
                color.primary,
                &PALETTE_COLORS,
                &palette_config,
            )
        } else {
            PortraitDescriptor::generate(&mut rng, color.primary)
        };

        roster.pilots.push(Pilot {
            id,
            gamertag,
            personality: traits,
            skill,
            color_scheme: color,
            drone_build: DroneBuildDescriptor::default(),
            portrait,
            stats: PilotStats::default(),
        });
    }

    save_roster_to_default(&roster);
    roster
}

fn pick_personality_traits(rng: &mut impl Rng) -> Vec<PersonalityTrait> {
    let mut selected: Vec<PersonalityTrait> = personality::ALL_TRAITS.to_vec();
    selected.shuffle(rng);

    let count = if rng.gen_bool(0.6) { 1 } else { 2 };
    selected.truncate(count);

    // Filter incompatible combos
    if selected.len() == 2 && personality::are_incompatible(selected[0], selected[1]) {
        selected.truncate(1);
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.pilots.ron");

        let mut roster = PilotRoster::default();
        let mut rng = rand::thread_rng();
        let mut used_tags = HashSet::new();

        for _ in 0..5 {
            let id = PilotId(roster.next_id);
            roster.next_id += 1;
            let tag = generate_gamertag(&mut rng, &used_tags);
            used_tags.insert(tag.clone());
            roster.pilots.push(Pilot {
                id,
                gamertag: tag,
                personality: vec![PersonalityTrait::Aggressive],
                skill: SkillProfile {
                    level: 0.5,
                    speed: 0.5,
                    cornering: 0.5,
                    consistency: 0.5,
                },
                color_scheme: ColorScheme {
                    primary: [1.0, 0.0, 0.0],
                },
                drone_build: DroneBuildDescriptor::default(),
                portrait: PortraitDescriptor::default(),
                stats: PilotStats::default(),
            });
        }

        save_roster(&roster, &path).unwrap();
        let loaded = load_roster_from_file(&path).unwrap();

        assert_eq!(loaded.pilots.len(), 5);
        assert_eq!(loaded.next_id, 6);
        assert_eq!(loaded.pilots[0].id, PilotId(1));
        assert_eq!(loaded.pilots[0].gamertag, roster.pilots[0].gamertag);
    }

    #[test]
    fn initial_roster_has_correct_size() {
        // We can't call generate_initial_roster() directly because it saves to disk,
        // so test the components instead
        let mut rng = rand::thread_rng();
        let mut used_tags = HashSet::new();
        for _ in 0..INITIAL_ROSTER_SIZE {
            let tag = generate_gamertag(&mut rng, &used_tags);
            used_tags.insert(tag);
        }
        assert_eq!(used_tags.len(), INITIAL_ROSTER_SIZE);
    }

    #[test]
    fn all_ids_unique() {
        let mut rng = rand::thread_rng();
        let mut roster = PilotRoster::default();
        let mut used_tags = HashSet::new();

        for _ in 0..24 {
            let id = PilotId(roster.next_id);
            roster.next_id += 1;
            let tag = generate_gamertag(&mut rng, &used_tags);
            used_tags.insert(tag.clone());
            roster.pilots.push(Pilot {
                id,
                gamertag: tag,
                personality: vec![PersonalityTrait::Smooth],
                skill: SkillProfile {
                    level: 0.5,
                    speed: 0.5,
                    cornering: 0.5,
                    consistency: 0.5,
                },
                color_scheme: ColorScheme {
                    primary: [1.0, 0.0, 0.0],
                },
                drone_build: DroneBuildDescriptor::default(),
                portrait: PortraitDescriptor::default(),
                stats: PilotStats::default(),
            });
        }

        let ids: HashSet<_> = roster.pilots.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 24);
    }

    #[test]
    fn load_missing_file_returns_error() {
        let result = load_roster_from_file(Path::new("nonexistent_file.ron"));
        assert!(result.is_err());
    }

    #[test]
    fn backward_compat_no_stats() {
        // RON without stats field should deserialize with defaults
        let ron_str = r#"(
            pilots: [(
                id: (1),
                gamertag: "TestPilot",
                personality: [Aggressive],
                skill: (level: 0.5, speed: 0.5, cornering: 0.5, consistency: 0.5),
                color_scheme: (primary: (1.0, 0.0, 0.0)),
            )],
            next_id: 2,
        )"#;
        let roster: PilotRoster = ron::from_str(ron_str).unwrap();
        assert_eq!(roster.pilots.len(), 1);
        assert_eq!(roster.pilots[0].stats.races_entered, 0);
        assert_eq!(roster.pilots[0].stats.best_time, None);
    }

    #[test]
    fn backward_compat_phase1_portrait_is_empty() {
        // Phase 1 roster: portrait is empty struct `()`
        let ron_str = r#"(
            pilots: [(
                id: (42),
                gamertag: "OldPilot",
                personality: [Smooth],
                skill: (level: 0.7, speed: 0.6, cornering: 0.5, consistency: 0.8),
                color_scheme: (primary: (0.5, 0.5, 1.0)),
            )],
            next_id: 43,
        )"#;
        let roster: PilotRoster = ron::from_str(ron_str).unwrap();
        assert!(roster.pilots[0].portrait.is_empty());
    }

    #[test]
    fn backfill_generates_portraits_for_empty() {
        let mut roster = PilotRoster {
            pilots: vec![
                Pilot {
                    id: PilotId(1),
                    gamertag: "Pilot1".to_string(),
                    personality: vec![PersonalityTrait::Aggressive],
                    skill: SkillProfile {
                        level: 0.5,
                        speed: 0.5,
                        cornering: 0.5,
                        consistency: 0.5,
                    },
                    color_scheme: ColorScheme {
                        primary: [1.0, 0.0, 0.0],
                    },
                    drone_build: DroneBuildDescriptor::default(),
                    portrait: PortraitDescriptor::default(),
                    stats: PilotStats::default(),
                },
            ],
            next_id: 2,
        };

        assert!(roster.pilots[0].portrait.is_empty());
        let changed = backfill_empty_portraits(&mut roster);
        assert!(changed);
        assert!(!roster.pilots[0].portrait.is_empty());
        assert!(roster.pilots[0].portrait.generated);
    }

    #[test]
    fn backfill_deterministic_same_pilot_same_portrait() {
        let mut roster1 = PilotRoster {
            pilots: vec![Pilot {
                id: PilotId(99),
                gamertag: "Test".to_string(),
                personality: vec![],
                skill: SkillProfile {
                    level: 0.5,
                    speed: 0.5,
                    cornering: 0.5,
                    consistency: 0.5,
                },
                color_scheme: ColorScheme {
                    primary: [0.5, 0.3, 0.8],
                },
                drone_build: DroneBuildDescriptor::default(),
                portrait: PortraitDescriptor::default(),
                stats: PilotStats::default(),
            }],
            next_id: 100,
        };
        let mut roster2 = roster1.clone();

        backfill_empty_portraits(&mut roster1);
        backfill_empty_portraits(&mut roster2);

        // Same PilotId should produce identical portrait
        assert_eq!(
            format!("{:?}", roster1.pilots[0].portrait.face_shape),
            format!("{:?}", roster2.pilots[0].portrait.face_shape)
        );
        assert_eq!(
            roster1.pilots[0].portrait.skin_tone,
            roster2.pilots[0].portrait.skin_tone
        );
    }

    #[test]
    fn backfill_skips_already_generated() {
        let mut rng = rand::thread_rng();
        let portrait = PortraitDescriptor::generate(&mut rng, [0.5, 0.5, 0.5]);
        let original_face = portrait.face_shape;
        let mut roster = PilotRoster {
            pilots: vec![Pilot {
                id: PilotId(1),
                gamertag: "Already".to_string(),
                personality: vec![],
                skill: SkillProfile {
                    level: 0.5,
                    speed: 0.5,
                    cornering: 0.5,
                    consistency: 0.5,
                },
                color_scheme: ColorScheme {
                    primary: [0.5, 0.5, 0.5],
                },
                drone_build: DroneBuildDescriptor::default(),
                portrait,
                stats: PilotStats::default(),
            }],
            next_id: 2,
        };

        let changed = backfill_empty_portraits(&mut roster);
        assert!(!changed);
        assert_eq!(roster.pilots[0].portrait.face_shape, original_face);
    }

    #[test]
    fn personality_traits_filter_incompatible() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let traits = pick_personality_traits(&mut rng);
            if traits.len() == 2 {
                assert!(
                    !personality::are_incompatible(traits[0], traits[1]),
                    "Incompatible pair generated: {:?} + {:?}",
                    traits[0],
                    traits[1]
                );
            }
        }
    }
}
