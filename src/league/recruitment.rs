pub struct RecruitmentTier {
    pub fan_threshold: u32,
    pub pilot_skill_range: (f32, f32),
    pub label: &'static str,
}

pub const RECRUITMENT_TIERS: &[RecruitmentTier] = &[
    RecruitmentTier {
        fan_threshold: 0,
        pilot_skill_range: (0.1, 0.3),
        label: "Amateur",
    },
    RecruitmentTier {
        fan_threshold: 5,
        pilot_skill_range: (0.2, 0.5),
        label: "Local",
    },
    RecruitmentTier {
        fan_threshold: 15,
        pilot_skill_range: (0.3, 0.7),
        label: "Regional",
    },
    RecruitmentTier {
        fan_threshold: 30,
        pilot_skill_range: (0.5, 0.85),
        label: "National",
    },
    RecruitmentTier {
        fan_threshold: 60,
        pilot_skill_range: (0.7, 1.0),
        label: "Elite",
    },
];

pub fn accessible_tier(fan_count: u32) -> &'static RecruitmentTier {
    RECRUITMENT_TIERS
        .iter()
        .rev()
        .find(|t| fan_count >= t.fan_threshold)
        .unwrap_or(&RECRUITMENT_TIERS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amateur_tier_at_zero() {
        let tier = accessible_tier(0);
        assert_eq!(tier.label, "Amateur");
    }

    #[test]
    fn test_local_tier() {
        let tier = accessible_tier(5);
        assert_eq!(tier.label, "Local");
    }

    #[test]
    fn test_elite_tier() {
        let tier = accessible_tier(60);
        assert_eq!(tier.label, "Elite");
        let tier2 = accessible_tier(100);
        assert_eq!(tier2.label, "Elite");
    }

    #[test]
    fn test_tier_boundary() {
        assert_eq!(accessible_tier(4).label, "Amateur");
        assert_eq!(accessible_tier(5).label, "Local");
        assert_eq!(accessible_tier(14).label, "Local");
        assert_eq!(accessible_tier(15).label, "Regional");
        assert_eq!(accessible_tier(29).label, "Regional");
        assert_eq!(accessible_tier(30).label, "National");
        assert_eq!(accessible_tier(59).label, "National");
        assert_eq!(accessible_tier(60).label, "Elite");
    }

    #[test]
    fn test_skill_ranges_ascending() {
        for window in RECRUITMENT_TIERS.windows(2) {
            assert!(
                window[1].pilot_skill_range.0 >= window[0].pilot_skill_range.0,
                "Lower bound should be non-decreasing"
            );
            assert!(
                window[1].pilot_skill_range.1 >= window[0].pilot_skill_range.1,
                "Upper bound should be non-decreasing"
            );
        }
    }
}
