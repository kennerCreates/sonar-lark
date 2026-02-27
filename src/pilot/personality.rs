use serde::{Deserialize, Serialize};

/// Personality traits that affect flying behavior and provide flavor text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PersonalityTrait {
    Aggressive,
    Cautious,
    Flashy,
    Methodical,
    Reckless,
    Smooth,
    Technical,
    Hotdog,
}

pub const ALL_TRAITS: [PersonalityTrait; 8] = [
    PersonalityTrait::Aggressive,
    PersonalityTrait::Cautious,
    PersonalityTrait::Flashy,
    PersonalityTrait::Methodical,
    PersonalityTrait::Reckless,
    PersonalityTrait::Smooth,
    PersonalityTrait::Technical,
    PersonalityTrait::Hotdog,
];

/// Modifier values applied on top of skill-based DroneConfig.
/// Additive fields are added directly. Scale fields multiply the randomized magnitude.
pub struct TraitModifiers {
    /// Additive adjustment to cornering_aggression
    pub cornering_aggression: f32,
    /// Additive adjustment to braking_distance
    pub braking_distance: f32,
    /// Multiplier on racing_line_bias magnitude
    pub racing_line_bias_scale: f32,
    /// Additive adjustment to noise_amplitude
    pub noise_amplitude: f32,
    /// Additive adjustment to gate_pass_offset
    pub gate_pass_offset: f32,
    /// Additive adjustment to approach_offset_scale
    pub approach_offset_scale: f32,
    /// Multiplier on PID variation magnitude
    pub pid_variation_scale: f32,
}

impl PersonalityTrait {
    pub fn modifiers(self) -> TraitModifiers {
        use PersonalityTrait::*;
        match self {
            Aggressive => TraitModifiers {
                cornering_aggression: 0.10,
                braking_distance: -0.08,
                racing_line_bias_scale: 1.3,
                noise_amplitude: 0.1,
                gate_pass_offset: 0.05,
                approach_offset_scale: -0.04,
                pid_variation_scale: 1.0,
            },
            Cautious => TraitModifiers {
                cornering_aggression: -0.10,
                braking_distance: 0.10,
                racing_line_bias_scale: 0.7,
                noise_amplitude: -0.2,
                gate_pass_offset: -0.05,
                approach_offset_scale: 0.05,
                pid_variation_scale: 0.8,
            },
            Flashy => TraitModifiers {
                cornering_aggression: 0.05,
                braking_distance: -0.05,
                racing_line_bias_scale: 1.5,
                noise_amplitude: 0.2,
                gate_pass_offset: 0.08,
                approach_offset_scale: -0.02,
                pid_variation_scale: 1.1,
            },
            Methodical => TraitModifiers {
                cornering_aggression: -0.05,
                braking_distance: 0.05,
                racing_line_bias_scale: 0.6,
                noise_amplitude: -0.3,
                gate_pass_offset: -0.03,
                approach_offset_scale: 0.03,
                pid_variation_scale: 0.7,
            },
            Reckless => TraitModifiers {
                cornering_aggression: 0.15,
                braking_distance: -0.12,
                racing_line_bias_scale: 1.4,
                noise_amplitude: 0.3,
                gate_pass_offset: 0.10,
                approach_offset_scale: -0.06,
                pid_variation_scale: 1.2,
            },
            Smooth => TraitModifiers {
                cornering_aggression: 0.0,
                braking_distance: 0.0,
                racing_line_bias_scale: 0.8,
                noise_amplitude: -0.4,
                gate_pass_offset: 0.0,
                approach_offset_scale: 0.02,
                pid_variation_scale: 0.6,
            },
            Technical => TraitModifiers {
                cornering_aggression: 0.03,
                braking_distance: 0.02,
                racing_line_bias_scale: 0.9,
                noise_amplitude: -0.1,
                gate_pass_offset: -0.02,
                approach_offset_scale: 0.04,
                pid_variation_scale: 0.8,
            },
            Hotdog => TraitModifiers {
                cornering_aggression: 0.08,
                braking_distance: -0.10,
                racing_line_bias_scale: 1.6,
                noise_amplitude: 0.4,
                gate_pass_offset: 0.12,
                approach_offset_scale: -0.05,
                pid_variation_scale: 1.3,
            },
        }
    }

    #[allow(dead_code)]
    pub fn catchphrases(self) -> &'static [&'static str] {
        use PersonalityTrait::*;
        match self {
            Aggressive => &[
                "No brakes, no mercy.",
                "I didn't come here to finish second.",
                "Full send, every corner.",
                "If you're not first, you're last.",
                "Get out of my line.",
            ],
            Cautious => &[
                "Slow is smooth, smooth is fast.",
                "Patience wins championships.",
                "I'll be here at the finish.",
                "Consistency is king.",
                "Every gate, every time.",
            ],
            Flashy => &[
                "Did you see that?!",
                "Style points count double.",
                "Making it look easy.",
                "The crowd loves me.",
                "Hold my controller.",
            ],
            Methodical => &[
                "Stick to the plan.",
                "Calculated.",
                "Every line is deliberate.",
                "Data doesn't lie.",
                "Optimal trajectory locked.",
            ],
            Reckless => &[
                "YOLO every lap.",
                "Gates are suggestions.",
                "What's a braking zone?",
                "Chaos is a ladder.",
                "Crash? Never heard of it.",
            ],
            Smooth => &[
                "Like butter through gates.",
                "Flow state activated.",
                "No wasted movement.",
                "Clean lines only.",
                "Turbulence? Not from me.",
            ],
            Technical => &[
                "PID tuned to perfection.",
                "0.02 seconds faster on that apex.",
                "Read the telemetry.",
                "Micro-adjustments matter.",
                "Optimized my line by 3%.",
            ],
            Hotdog => &[
                "Watch this!",
                "Hold my energy drink.",
                "Bet you can't do THAT.",
                "I live for the highlight reel.",
                "Risk it for the biscuit.",
            ],
        }
    }
}

/// Returns true if two traits are personality-incompatible and shouldn't be combined.
pub fn are_incompatible(a: PersonalityTrait, b: PersonalityTrait) -> bool {
    use PersonalityTrait::*;
    matches!(
        (a, b),
        (Aggressive, Cautious)
            | (Cautious, Aggressive)
            | (Reckless, Methodical)
            | (Methodical, Reckless)
            | (Reckless, Cautious)
            | (Cautious, Reckless)
            | (Smooth, Hotdog)
            | (Hotdog, Smooth)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_traits_have_catchphrases() {
        for trait_ in &ALL_TRAITS {
            let phrases = trait_.catchphrases();
            assert!(
                phrases.len() >= 4,
                "{:?} has only {} catchphrases",
                trait_,
                phrases.len()
            );
            for phrase in phrases {
                assert!(!phrase.is_empty());
            }
        }
    }

    #[test]
    fn modifiers_within_reasonable_bounds() {
        for trait_ in &ALL_TRAITS {
            let m = trait_.modifiers();
            assert!(m.cornering_aggression.abs() <= 0.2);
            assert!(m.braking_distance.abs() <= 0.15);
            assert!((0.5..=2.0).contains(&m.racing_line_bias_scale));
            assert!(m.noise_amplitude.abs() <= 0.5);
            assert!(m.gate_pass_offset.abs() <= 0.15);
            assert!(m.approach_offset_scale.abs() <= 0.1);
            assert!((0.5..=1.5).contains(&m.pid_variation_scale));
        }
    }

    #[test]
    fn incompatible_traits_are_symmetric() {
        for &a in &ALL_TRAITS {
            for &b in &ALL_TRAITS {
                assert_eq!(
                    are_incompatible(a, b),
                    are_incompatible(b, a),
                    "Asymmetric incompatibility: {:?} vs {:?}",
                    a,
                    b
                );
            }
        }
    }

    #[test]
    fn aggressive_and_cautious_incompatible() {
        assert!(are_incompatible(
            PersonalityTrait::Aggressive,
            PersonalityTrait::Cautious
        ));
    }

    #[test]
    fn same_trait_not_incompatible() {
        for &t in &ALL_TRAITS {
            assert!(!are_incompatible(t, t));
        }
    }
}
