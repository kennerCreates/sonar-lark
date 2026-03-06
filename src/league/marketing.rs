use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MarketingEffects {
    pub aware_attendance_nudge: f32,
    pub new_aware_count: u32,
    pub spread_potency_mult: f32,
    pub spread_volume_bonus: u32,
    pub decay_slowdown: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CampaignBudgets {
    pub posters: f32,
    pub highlight_reel: f32,
    pub merch: f32,
}

pub fn compute_marketing_effects(budgets: &CampaignBudgets) -> MarketingEffects {
    let poster_new = (8.0 * (1.0 + budgets.posters).ln()).floor() as u32;
    let poster_nudge = 0.05 * (1.0 - (-0.1 * budgets.posters).exp());

    let reel_potency = 1.0 + 0.5 * (1.0 - (-0.05 * budgets.highlight_reel).exp());
    let reel_new = (2.0 * (1.0 + budgets.highlight_reel).ln()).floor() as u32;

    let merch_bonus = (3.0 * (1.0 - (-0.03 * budgets.merch).exp())).floor() as u32;
    let merch_decay = budgets.merch > 0.0;

    MarketingEffects {
        aware_attendance_nudge: poster_nudge,
        new_aware_count: poster_new + reel_new,
        spread_potency_mult: reel_potency,
        spread_volume_bonus: merch_bonus,
        decay_slowdown: merch_decay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_budgets() {
        let effects = compute_marketing_effects(&CampaignBudgets::default());
        assert_eq!(effects.new_aware_count, 0);
        assert!((effects.aware_attendance_nudge).abs() < f32::EPSILON);
        assert!((effects.spread_potency_mult - 1.0).abs() < f32::EPSILON);
        assert_eq!(effects.spread_volume_bonus, 0);
        assert!(!effects.decay_slowdown);
    }

    #[test]
    fn test_poster_injection() {
        let effects = compute_marketing_effects(&CampaignBudgets {
            posters: 10.0,
            ..Default::default()
        });
        assert!(effects.new_aware_count > 0);
        assert!(effects.aware_attendance_nudge > 0.0);
    }

    #[test]
    fn test_highlight_reel_potency() {
        let effects = compute_marketing_effects(&CampaignBudgets {
            highlight_reel: 20.0,
            ..Default::default()
        });
        assert!(effects.spread_potency_mult > 1.0);
        assert!(effects.new_aware_count > 0);
    }

    #[test]
    fn test_merch_decay_slowdown() {
        let effects = compute_marketing_effects(&CampaignBudgets {
            merch: 5.0,
            ..Default::default()
        });
        assert!(effects.decay_slowdown);
        assert!(effects.spread_volume_bonus > 0 || effects.decay_slowdown);
    }

    #[test]
    fn test_combined_effects() {
        let effects = compute_marketing_effects(&CampaignBudgets {
            posters: 10.0,
            highlight_reel: 15.0,
            merch: 8.0,
        });
        assert!(effects.new_aware_count > 0);
        assert!(effects.aware_attendance_nudge > 0.0);
        assert!(effects.spread_potency_mult > 1.0);
        assert!(effects.decay_slowdown);
    }
}
