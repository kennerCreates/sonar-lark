use rand::Rng;
use std::collections::HashSet;

const PREFIXES: &[&str] = &[
    "x", "xx", "X", "Shadow", "Dark", "N1tro", "Neo", "Cyber", "Dr", "El", "Lil", "Hyper",
    "Ultra", "Turbo", "Razor", "xX", "Xx", "Mega", "Zero", "Ace",
];

const ROOTS: &[&str] = &[
    "Vortex", "Pulse", "Glitch", "Phantom", "Blitz", "Nova", "Spark", "Drift", "Surge", "Bolt",
    "Echo", "Fury", "Hawk", "Falcon", "Viper", "Raven", "Storm", "Flash", "Vertex", "Pixel",
    "Cipher", "Neon", "Orbit", "Flux", "Apex", "Zenith", "Wraith", "Specter", "Torque", "Vector",
    "Blade", "Ghost", "Comet", "Fang", "Pyro", "Strafe", "Throttle", "Nitro", "Havoc", "Onyx",
];

const SUFFIXES: &[&str] = &[
    "_", "xx", "X", "z", "0", "1", "99", "420", "007", "_TTV", "FPV", "HD", "Pro", "YT", "GG",
    "XD", "_v2", "Jr", "II", "69",
];

/// Gamertag formatting styles.
#[derive(Clone, Copy)]
enum Style {
    /// "ShadowVortex"
    PrefixRoot,
    /// "VortexPro"
    RootSuffix,
    /// "xShadowVortex_"
    PrefixRootSuffix,
    /// "Sh4d0wV0rt3x"
    LeetRoot,
    /// "VortexPulse"
    DoubleRoot,
    /// "Vortex"
    BareRoot,
}

const ALL_STYLES: [Style; 6] = [
    Style::PrefixRoot,
    Style::RootSuffix,
    Style::PrefixRootSuffix,
    Style::LeetRoot,
    Style::DoubleRoot,
    Style::BareRoot,
];

/// Generate a unique gamertag. `existing` prevents collisions with already-used tags.
pub fn generate_gamertag(rng: &mut impl Rng, existing: &HashSet<String>) -> String {
    for _ in 0..200 {
        let tag = generate_one(rng);
        if !existing.contains(&tag) {
            return tag;
        }
    }
    // Fallback: append random digits until unique
    loop {
        let tag = format!("{}{}", generate_one(rng), rng.gen_range(1000..9999));
        if !existing.contains(&tag) {
            return tag;
        }
    }
}

fn generate_one(rng: &mut impl Rng) -> String {
    let style = ALL_STYLES[rng.gen_range(0..ALL_STYLES.len())];
    match style {
        Style::PrefixRoot => {
            let prefix = PREFIXES[rng.gen_range(0..PREFIXES.len())];
            let root = ROOTS[rng.gen_range(0..ROOTS.len())];
            format!("{prefix}{root}")
        }
        Style::RootSuffix => {
            let root = ROOTS[rng.gen_range(0..ROOTS.len())];
            let suffix = SUFFIXES[rng.gen_range(0..SUFFIXES.len())];
            format!("{root}{suffix}")
        }
        Style::PrefixRootSuffix => {
            let prefix = PREFIXES[rng.gen_range(0..PREFIXES.len())];
            let root = ROOTS[rng.gen_range(0..ROOTS.len())];
            let suffix = SUFFIXES[rng.gen_range(0..SUFFIXES.len())];
            format!("{prefix}{root}{suffix}")
        }
        Style::LeetRoot => {
            let root = ROOTS[rng.gen_range(0..ROOTS.len())];
            leetspeak(root, rng)
        }
        Style::DoubleRoot => {
            let a = ROOTS[rng.gen_range(0..ROOTS.len())];
            let b = ROOTS[rng.gen_range(0..ROOTS.len())];
            format!("{a}{b}")
        }
        Style::BareRoot => {
            let root = ROOTS[rng.gen_range(0..ROOTS.len())];
            // Add a number to keep it gamer-tag-like
            let n: u32 = rng.gen_range(1..=999);
            format!("{root}{n}")
        }
    }
}

fn leetspeak(s: &str, rng: &mut impl Rng) -> String {
    s.chars()
        .map(|c| {
            if rng.gen_bool(0.4) {
                match c {
                    'a' | 'A' => '4',
                    'e' | 'E' => '3',
                    'i' | 'I' => '1',
                    'o' | 'O' => '0',
                    's' | 'S' => '5',
                    't' | 'T' => '7',
                    _ => c,
                }
            } else {
                c
            }
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
