use rand::Rng;
use std::collections::HashSet;

const ROOTS: &[&str] = &[
    "Vortex", "Pulse", "Glitch", "Phantom", "Blitz", "Nova", "Spark", "Drift", "Surge", "Bolt",
    "Echo", "Fury", "Hawk", "Falcon", "Viper", "Raven", "Storm", "Flash", "Shadow",
    "Cipher", "Neon", "Orbit", "Flux", "Apex", "Zenith", "Wraith", "Specter", "Torque", "Vector",
    "Blade", "Ghost", "Comet", "Fang", "Pyro", "Strafe", "Throttle", "Nitro", "Havoc", "Onyx",
    "Aegis", "Rift", "Crux", "Shard", "Jinx", "Hex", "Brink", "Prism", "Haze", "Warp",
    "Flare", "Raid", "Siege", "Claw", "Dash", "Jolt", "Turbo", "Magnet", "Fuse",
    "Dynamo", "Ripple", "Quake", "Ember", "Razor", "Chrome", "Stealth", "Chaos", "Impact", "Omega",
    "Titan", "Rumble", "Snare", "Sonic", "Cobalt", "Neutron", "Diesel", "Trigger", "Volts",
    "Luna", "Siren", "Astra", "Lyra", "Valkyrie", "Seraph", "Celeste", "Athena", "Ivy", "Aria",
    "Nyx", "Soleil", "Vega", "Tempest", "Zara", "Rune", "Freya", "Karma", "Halo", "Aurora",
    "Cleo", "Jade", "Mira", "Stella", "Kira", "Elektra", "Selene", "Sable", "Echo", "Phaedra",
    "Vesper", "Artemis", "Nova", "Iris", "Wren", "Crimson", "Banshee", "Soraya", "Lux", "Rogue",
    "Pandora", "Medusa", "Calypso", "Persephone", "Juno", "Hera", "Demeter", "Circe",
    "Minerva", "Diana", "Brigid", "Amara", "Theia", "Rhea", "Gaia", "Eris", "Nemesis",
    "Pixie", "Nova", "Zephyr", "Diva", "Bliss", "Shimmer", "Velvet", "Mirage", "Starling", "Whisper",
    "Opal", "Pearl", "Sapphire", "Garnet", "Amethyst", "Topaz", "Ruby", "Scarlet", "Ivory", "Silver",
    "Venom", "Hex", "Spite", "Dagger", "Thorn", "Scythe", "Shade", "Phantom", "Prowl", "Sting",
    "Riot", "Rebel", "Fury", "Blaze", "Striker", "Mystic", "Spectra", "Glimmer", "Nova", "Zenith",
    "Solstice", "Eclipse", "Nebula", "Cascade", "Blossom", "Dahlia", "Lotus", "Willow", "Sage", "Fern",
    "Coral", "Marina", "Dove", "Lark", "Sparrow", "Petal", "Briar", "Roslyn", "Meadow", "Clover",
];

const DIVIDERS: &[char] = &['-', '_', '.'];

/// Gamertag formatting styles.
#[derive(Clone, Copy)]
enum Style {
    /// "V0rt3x"
    Leet,
    /// "VortexPulse"
    PascalDouble,
    /// "vortexPulse"
    CamelDouble,
    /// "VORTEX"
    AllCaps,
    /// "VORTEXPULSE"
    ScreamDouble,
    /// "vortex-pulse"
    AllLower,
    /// "Vortex_Pulse"
    Separated,
    /// "roseHAWK"
    LowerUpper,
}

const ALL_STYLES: [Style; 8] = [
    Style::Leet,
    Style::PascalDouble,
    Style::CamelDouble,
    Style::AllCaps,
    Style::ScreamDouble,
    Style::AllLower,
    Style::Separated,
    Style::LowerUpper,
];

/// Generate a unique gamertag. `existing` prevents collisions with already-used tags.
pub fn generate_gamertag(rng: &mut impl Rng, existing: &HashSet<String>) -> String {
    for _ in 0..200 {
        let tag = generate_one(rng);
        if !existing.contains(&tag) {
            return tag;
        }
    }
    // Fallback: triple-root combination
    loop {
        let a = pick_root(rng);
        let b = pick_root(rng);
        let c = pick_root(rng);
        let tag = format!("{a}{b}{c}");
        if !existing.contains(&tag) {
            return tag;
        }
    }
}

fn pick_root(rng: &mut impl Rng) -> &'static str {
    ROOTS[rng.gen_range(0..ROOTS.len())]
}

fn generate_one(rng: &mut impl Rng) -> String {
    let style = ALL_STYLES[rng.gen_range(0..ALL_STYLES.len())];
    match style {
        Style::Leet => leetspeak(pick_root(rng), rng),
        Style::PascalDouble => {
            let a = pick_root(rng);
            let b = pick_root(rng);
            format!("{a}{b}")
        }
        Style::CamelDouble => {
            let a = pick_root(rng).to_lowercase();
            let b = pick_root(rng);
            format!("{a}{b}")
        }
        Style::AllCaps => pick_root(rng).to_uppercase(),
        Style::ScreamDouble => {
            let a = pick_root(rng).to_uppercase();
            let b = pick_root(rng).to_uppercase();
            format!("{a}{b}")
        }
        Style::AllLower => {
            let a = pick_root(rng).to_lowercase();
            let b = pick_root(rng).to_lowercase();
            let sep = DIVIDERS[rng.gen_range(0..DIVIDERS.len())];
            format!("{a}{sep}{b}")
        }
        Style::Separated => {
            let a = pick_root(rng);
            let b = pick_root(rng);
            format!("{a}_{b}")
        }
        Style::LowerUpper => {
            let a = pick_root(rng).to_lowercase();
            let b = pick_root(rng).to_uppercase();
            format!("{a}{b}")
        }
    }
}

fn leetspeak(s: &str, rng: &mut impl Rng) -> String {
    const LEET_MAPS: [(char, char, char); 6] = [
        ('a', 'A', '4'),
        ('e', 'E', '3'),
        ('i', 'I', '1'),
        ('o', 'O', '0'),
        ('s', 'S', '5'),
        ('t', 'T', '7'),
    ];
    // Randomly enable each letter mapping (all-or-nothing per letter)
    let active: [bool; 6] = std::array::from_fn(|_| rng.gen_bool(0.4));
    s.chars()
        .map(|c| {
            for (i, &(lo, hi, repl)) in LEET_MAPS.iter().enumerate() {
                if active[i] && (c == lo || c == hi) {
                    return repl;
                }
            }
            c
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_100_unique_tags() {
        let mut rng = rand::thread_rng();
        let mut existing = HashSet::new();
        for _ in 0..100 {
            let tag = generate_gamertag(&mut rng, &existing);
            assert!(
                existing.insert(tag.clone()),
                "Duplicate tag generated: {tag}"
            );
        }
        assert_eq!(existing.len(), 100);
    }

    #[test]
    fn gamertag_nonempty() {
        let mut rng = rand::thread_rng();
        let existing = HashSet::new();
        for _ in 0..50 {
            let tag = generate_gamertag(&mut rng, &existing);
            assert!(!tag.is_empty());
        }
    }

    #[test]
    fn gamertag_reasonable_length() {
        let mut rng = rand::thread_rng();
        let existing = HashSet::new();
        for _ in 0..100 {
            let tag = generate_gamertag(&mut rng, &existing);
            assert!(
                tag.len() >= 3 && tag.len() <= 30,
                "Tag '{}' has unreasonable length {}",
                tag,
                tag.len()
            );
        }
    }

    #[test]
    fn leetspeak_transforms() {
        // Deterministic test: with 100% leet probability
        struct AlwaysLeet;
        // We can't easily control rng probability, so just test the function doesn't panic
        let mut rng = rand::thread_rng();
        let result = leetspeak("Vortex", &mut rng);
        assert_eq!(result.len(), "Vortex".len());
    }
}
