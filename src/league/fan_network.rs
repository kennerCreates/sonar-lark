use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use super::marketing::MarketingEffects;

pub type PersonId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FanTier {
    Aware,
    Attendee,
    Fan,
    Superfan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub recruited_by: Option<PersonId>,
    pub tier: FanTier,
    pub races_attended: u16,
    pub races_since_attended: u8,
    pub spread_count: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FanNetwork {
    pub people: Vec<Person>,
    next_id: PersonId,
}

/// Inputs to the per-race fan simulation.
pub struct RaceAttractionInputs {
    pub track_quality: f32,
    pub location_attractiveness: f32,
    pub capacity: u32,
    pub marketing: MarketingEffects,
    /// Ticket price in whole dollars. 0 = free entry.
    pub ticket_price: u32,
    pub seed: u32,
}

/// Output of the per-race fan simulation.
#[derive(Resource)]
pub struct RaceAttractionResult {
    pub demand: u32,
    pub actual_attendance: u32,
    pub turned_away: u32,
    pub new_aware_from_spread: u32,
    pub promotions: u32,
    pub demotions: u32,
    pub removed: u32,
    pub fan_count: u32,
    pub network_size: u32,
}

fn det_hash(seed: u32, a: u32, b: u32) -> u32 {
    let mut h = seed;
    h = h.wrapping_mul(2654435761).wrapping_add(a);
    h = h.wrapping_mul(2654435761).wrapping_add(b);
    h ^ (h >> 16)
}

fn det_f32(seed: u32, a: u32, b: u32) -> f32 {
    (det_hash(seed, a, b) & 0x00FF_FFFF) as f32 / 16_777_216.0
}

impl FanNetwork {
    pub fn new_seeded(seed: u32) -> Self {
        let count = 3 + (det_hash(seed, 0, 0) % 3); // 3-5
        let mut people = Vec::with_capacity(count as usize);
        for i in 0..count {
            people.push(Person {
                id: i,
                recruited_by: None,
                tier: FanTier::Aware,
                races_attended: 0,
                races_since_attended: 0,
                spread_count: 0,
            });
        }
        FanNetwork {
            people,
            next_id: count,
        }
    }

    pub fn fan_count(&self) -> u32 {
        self.people
            .iter()
            .filter(|p| matches!(p.tier, FanTier::Fan | FanTier::Superfan))
            .count() as u32
    }

    pub fn network_size(&self) -> u32 {
        self.people.len() as u32
    }

    fn add_aware(&mut self, recruited_by: Option<PersonId>) -> PersonId {
        let id = self.next_id;
        self.next_id += 1;
        self.people.push(Person {
            id,
            recruited_by,
            tier: FanTier::Aware,
            races_attended: 0,
            races_since_attended: 0,
            spread_count: 0,
        });
        id
    }
}

fn tier_order(tier: FanTier) -> u8 {
    match tier {
        FanTier::Superfan => 3,
        FanTier::Fan => 2,
        FanTier::Attendee => 1,
        FanTier::Aware => 0,
    }
}

pub fn simulate_race(
    network: &mut FanNetwork,
    inputs: &RaceAttractionInputs,
) -> RaceAttractionResult {
    let decay_slowdown = inputs.marketing.decay_slowdown;
    let demotion_threshold: u8 = if decay_slowdown { 4 } else { 3 };
    let removal_threshold: u8 = if decay_slowdown { 6 } else { 5 };

    // Step 1: Marketing injection
    for _ in 0..inputs.marketing.new_aware_count {
        network.add_aware(None);
    }

    // Step 2: Attendance roll
    let location_mod = 0.5 + 0.5 * inputs.location_attractiveness;
    let hype = inputs.track_quality * 0.5 + inputs.location_attractiveness * 0.5;
    let mut wants_to_attend: Vec<usize> = Vec::new();

    for (idx, person) in network.people.iter().enumerate() {
        let base = match person.tier {
            FanTier::Aware => 0.15,
            FanTier::Attendee => 0.40,
            FanTier::Fan => 0.75,
            FanTier::Superfan => 0.95,
        };
        let mut prob = base * location_mod;
        if person.tier == FanTier::Aware {
            prob += inputs.marketing.aware_attendance_nudge;
        }

        // Pricing curve:
        // - Free ($0): bonus to attendance (people love free events)
        // - $1: neutral (no effect)
        // - $2+: soft-cap penalty that plateaus (die-hard fans still show up)
        if inputs.ticket_price == 0 {
            let free_bonus = match person.tier {
                FanTier::Aware => 0.15,
                FanTier::Attendee => 0.08,
                FanTier::Fan => 0.03,
                FanTier::Superfan => 0.01,
            };
            prob += free_bonus;
        } else if inputs.ticket_price > 1 {
            let overage = (inputs.ticket_price - 1) as f32;
            let max_penalty = match person.tier {
                FanTier::Aware => 0.30,
                FanTier::Attendee => 0.25,
                FanTier::Fan => 0.15,
                FanTier::Superfan => 0.08,
            };
            let price_penalty =
                max_penalty * (1.0 - (-overage * 0.12).exp()) * (1.0 - hype * 0.7);
            prob -= price_penalty;
        }

        prob = prob.clamp(0.0, 1.0);

        let roll = det_f32(inputs.seed, person.id, 0);
        if roll < prob {
            wants_to_attend.push(idx);
        }
    }

    let demand = wants_to_attend.len() as u32;

    // Step 3: Capacity cap — prioritize by tier (Superfans first), then by id order
    wants_to_attend.sort_by(|&a, &b| {
        let tier_a = tier_order(network.people[a].tier);
        let tier_b = tier_order(network.people[b].tier);
        tier_b.cmp(&tier_a).then(network.people[a].id.cmp(&network.people[b].id))
    });

    let mut attending_set = vec![false; network.people.len()];
    let mut turned_away_set = vec![false; network.people.len()];
    let actual_attendance = demand.min(inputs.capacity);
    let turned_away = demand.saturating_sub(inputs.capacity);

    for (i, &idx) in wants_to_attend.iter().enumerate() {
        if (i as u32) < inputs.capacity {
            attending_set[idx] = true;
        } else {
            turned_away_set[idx] = true;
        }
    }

    // Step 4: Update attendance counters
    for (idx, person) in network.people.iter_mut().enumerate() {
        if attending_set[idx] {
            person.races_attended += 1;
            person.races_since_attended = 0;
        } else if !turned_away_set[idx] {
            person.races_since_attended = person.races_since_attended.saturating_add(1);
        }
        // Turned-away people: no change to races_since_attended
    }

    // Step 5: Promotions
    let mut promotions = 0u32;
    for (idx, person) in network.people.iter_mut().enumerate() {
        let just_attended = attending_set[idx];
        match person.tier {
            FanTier::Aware => {
                if person.races_attended >= 1 && just_attended {
                    person.tier = FanTier::Attendee;
                    promotions += 1;
                }
            }
            FanTier::Attendee => {
                if person.races_attended >= 3 && person.races_since_attended == 0 {
                    person.tier = FanTier::Fan;
                    promotions += 1;
                }
            }
            FanTier::Fan => {
                if person.races_attended >= 8 && person.races_since_attended == 0 {
                    person.tier = FanTier::Superfan;
                    promotions += 1;
                }
            }
            FanTier::Superfan => {}
        }
    }

    // Step 6: Demotions
    let mut demotions = 0u32;
    let mut to_remove: Vec<PersonId> = Vec::new();
    for person in network.people.iter_mut() {
        match person.tier {
            FanTier::Superfan => {
                if person.races_since_attended >= demotion_threshold {
                    person.tier = FanTier::Fan;
                    demotions += 1;
                }
            }
            FanTier::Fan => {
                if person.races_since_attended >= demotion_threshold {
                    person.tier = FanTier::Attendee;
                    demotions += 1;
                }
            }
            FanTier::Attendee => {
                if person.races_since_attended >= demotion_threshold {
                    person.tier = FanTier::Aware;
                    demotions += 1;
                }
            }
            FanTier::Aware => {
                if person.races_since_attended >= removal_threshold {
                    to_remove.push(person.id);
                }
            }
        }
    }

    let removed = to_remove.len() as u32;
    network.people.retain(|p| !to_remove.contains(&p.id));

    // Rebuild attending_set indices after removal — we need person ids for spreading
    // Collect ids of people who attended and are eligible to spread
    let mut spreaders: Vec<(PersonId, FanTier)> = Vec::new();
    for person in &network.people {
        // Check if this person attended (by id, since indices may have shifted)
        // We use a simpler approach: check races_since_attended == 0 AND races_attended > 0
        // which means they just attended this race
        if person.races_since_attended == 0
            && person.races_attended > 0
            && matches!(person.tier, FanTier::Attendee | FanTier::Fan | FanTier::Superfan)
        {
            spreaders.push((person.id, person.tier));
        }
    }

    // Step 7: Spreading
    let excitement = inputs.track_quality * 0.5 + inputs.location_attractiveness * 0.5;
    let roll_count = 1 + inputs.marketing.spread_volume_bonus;
    let mut new_aware_from_spread = 0u32;

    for &(spreader_id, tier) in &spreaders {
        let base_chance = match tier {
            FanTier::Attendee => 0.10,
            FanTier::Fan => 0.20,
            FanTier::Superfan => 0.35,
            FanTier::Aware => unreachable!(),
        };
        let free_event_mult = if inputs.ticket_price == 0 { 1.3 } else { 1.0 };
        let final_chance =
            base_chance * inputs.marketing.spread_potency_mult * free_event_mult * (0.5 + excitement);

        for roll_idx in 0..roll_count {
            let roll = det_f32(inputs.seed, spreader_id, 1000 + roll_idx);
            if roll < final_chance {
                network.add_aware(Some(spreader_id));
                new_aware_from_spread += 1;
                // Update spread_count on the spreader
                if let Some(spreader) = network.people.iter_mut().find(|p| p.id == spreader_id) {
                    spreader.spread_count = spreader.spread_count.saturating_add(1);
                }
            }
        }
    }

    RaceAttractionResult {
        demand,
        actual_attendance,
        turned_away,
        new_aware_from_spread,
        promotions,
        demotions,
        removed,
        fan_count: network.fan_count(),
        network_size: network.network_size(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::marketing::MarketingEffects;

    fn default_marketing() -> MarketingEffects {
        MarketingEffects {
            aware_attendance_nudge: 0.0,
            new_aware_count: 0,
            spread_potency_mult: 1.0,
            spread_volume_bonus: 0,
            decay_slowdown: false,
        }
    }

    fn default_inputs(seed: u32) -> RaceAttractionInputs {
        RaceAttractionInputs {
            track_quality: 0.5,
            location_attractiveness: 0.5,
            capacity: 1000,
            marketing: default_marketing(),
            ticket_price: 0,
            seed,
        }
    }

    #[test]
    fn test_new_seeded_size() {
        for seed in 0..20 {
            let net = FanNetwork::new_seeded(seed);
            assert!(
                (3..=5).contains(&net.people.len()),
                "Seed {seed} produced {} people",
                net.people.len()
            );
            for person in &net.people {
                assert_eq!(person.tier, FanTier::Aware);
                assert_eq!(person.races_attended, 0);
                assert_eq!(person.races_since_attended, 0);
            }
        }
    }

    #[test]
    fn test_new_seeded_determinism() {
        let a = FanNetwork::new_seeded(42);
        let b = FanNetwork::new_seeded(42);
        assert_eq!(a.people.len(), b.people.len());
        for (pa, pb) in a.people.iter().zip(b.people.iter()) {
            assert_eq!(pa.id, pb.id);
            assert_eq!(pa.tier, pb.tier);
        }
    }

    #[test]
    fn test_single_race_no_marketing() {
        let mut net = FanNetwork::new_seeded(99);
        let inputs = default_inputs(99);
        let result = simulate_race(&mut net, &inputs);
        assert!(result.network_size >= 3);
        assert_eq!(result.turned_away, 0); // capacity 1000 >> small network
    }

    #[test]
    fn test_promotion_aware_to_attendee() {
        let mut net = FanNetwork {
            people: vec![Person {
                id: 0,
                recruited_by: None,
                tier: FanTier::Aware,
                races_attended: 0,
                races_since_attended: 0,
                spread_count: 0,
            }],
            next_id: 1,
        };

        // Run many races with high attendance probability to guarantee attendance
        // Use high location_attractiveness to boost Aware attendance prob
        for seed in 0..50 {
            let inputs = RaceAttractionInputs {
                track_quality: 1.0,
                location_attractiveness: 1.0,
                capacity: 1000,
                marketing: MarketingEffects {
                    aware_attendance_nudge: 0.85, // push prob to ~1.0
                    ..default_marketing()
                },
                ticket_price: 0,
                seed,
            };
            simulate_race(&mut net, &inputs);
        }

        // After attending at least once, should have been promoted
        let person = net.people.iter().find(|p| p.id == 0);
        assert!(
            person.is_none()
                || person.unwrap().tier != FanTier::Aware
                || person.unwrap().races_attended == 0,
            "Person should have been promoted from Aware after attending"
        );
    }

    #[test]
    fn test_promotion_attendee_to_fan() {
        let mut net = FanNetwork {
            people: vec![Person {
                id: 0,
                recruited_by: None,
                tier: FanTier::Attendee,
                races_attended: 2,
                races_since_attended: 0,
                spread_count: 0,
            }],
            next_id: 1,
        };

        // Run races with near-guaranteed attendance for Attendee tier
        for seed in 0..20 {
            let inputs = RaceAttractionInputs {
                track_quality: 1.0,
                location_attractiveness: 1.0,
                capacity: 1000,
                marketing: default_marketing(),
                ticket_price: 0,
                seed,
            };
            simulate_race(&mut net, &inputs);
        }

        // After enough attendances (started at 2, needs 3 total), should be Fan or higher
        let person = net.people.iter().find(|p| p.id == 0);
        if let Some(p) = person {
            if p.races_attended >= 3 {
                assert!(
                    matches!(p.tier, FanTier::Fan | FanTier::Superfan),
                    "Expected Fan+ after 3+ attendances, got {:?} (attended: {})",
                    p.tier,
                    p.races_attended
                );
            }
        }
    }

    #[test]
    fn test_promotion_fan_to_superfan() {
        let mut net = FanNetwork {
            people: vec![Person {
                id: 0,
                recruited_by: None,
                tier: FanTier::Fan,
                races_attended: 7,
                races_since_attended: 0,
                spread_count: 0,
            }],
            next_id: 1,
        };

        // Fan base prob = 0.75, with location 1.0 => 0.75 * 1.0 = 0.75
        for seed in 0..30 {
            let inputs = RaceAttractionInputs {
                track_quality: 1.0,
                location_attractiveness: 1.0,
                capacity: 1000,
                marketing: default_marketing(),
                ticket_price: 0,
                seed,
            };
            simulate_race(&mut net, &inputs);
        }

        let person = net.people.iter().find(|p| p.id == 0);
        if let Some(p) = person {
            if p.races_attended >= 8 {
                assert_eq!(
                    p.tier,
                    FanTier::Superfan,
                    "Expected Superfan after 8+ attendances"
                );
            }
        }
    }

    #[test]
    fn test_demotion_after_missed_races() {
        let mut net = FanNetwork {
            people: vec![Person {
                id: 0,
                recruited_by: None,
                tier: FanTier::Fan,
                races_attended: 5,
                races_since_attended: 0,
                spread_count: 0,
            }],
            next_id: 1,
        };

        // Use seed/location that makes attendance very unlikely for Fan
        // Actually, set capacity to 0 so nobody can attend
        for seed in 0..5 {
            let inputs = RaceAttractionInputs {
                track_quality: 0.0,
                location_attractiveness: 0.0,
                capacity: 0,
                marketing: default_marketing(),
                ticket_price: 0,
                seed: 500 + seed,
            };
            simulate_race(&mut net, &inputs);
        }

        // After 5 races with 0 capacity, person misses all:
        // Race 3: races_since_attended=3 → Fan demoted to Attendee
        // Race 3 (same pass): Attendee with races_since_attended=3 is NOT re-demoted (already 3)
        // Actually demotions check current tier after promotion pass, so:
        // After 3 misses: Fan→Attendee. After 3 more: Attendee→Aware. After 5 total from Aware: removed.
        // With 5 races total, person should be demoted at minimum.
        let person = net.people.iter().find(|p| p.id == 0);
        match person {
            None => {} // removed entirely — that's fine, demotion chain completed
            Some(p) => assert!(
                matches!(p.tier, FanTier::Attendee | FanTier::Aware),
                "Fan should have been demoted after missing 3+ races, got {:?} (missed: {})",
                p.tier,
                p.races_since_attended
            ),
        }
    }

    #[test]
    fn test_aware_removal_after_5_missed() {
        let mut net = FanNetwork {
            people: vec![Person {
                id: 0,
                recruited_by: None,
                tier: FanTier::Aware,
                races_attended: 0,
                races_since_attended: 0,
                spread_count: 0,
            }],
            next_id: 1,
        };

        // capacity 0 ensures nobody attends, location 0 ensures low desire too
        for seed in 0..10 {
            let inputs = RaceAttractionInputs {
                track_quality: 0.0,
                location_attractiveness: 0.0,
                capacity: 0,
                marketing: default_marketing(),
                ticket_price: 0,
                seed: 800 + seed,
            };
            simulate_race(&mut net, &inputs);
        }

        // After 5+ missed races with no attendance, should be removed
        let person = net.people.iter().find(|p| p.id == 0);
        assert!(
            person.is_none(),
            "Aware person should be removed after missing 5+ races"
        );
    }

    #[test]
    fn test_turned_away_no_decay() {
        // Create a Superfan and an Aware person. Capacity=1 so only Superfan gets in.
        let mut net = FanNetwork {
            people: vec![
                Person {
                    id: 0,
                    recruited_by: None,
                    tier: FanTier::Superfan,
                    races_attended: 10,
                    races_since_attended: 0,
                    spread_count: 0,
                },
                Person {
                    id: 1,
                    recruited_by: None,
                    tier: FanTier::Fan,
                    races_attended: 5,
                    races_since_attended: 0,
                    spread_count: 0,
                },
            ],
            next_id: 2,
        };

        // High location so both want to attend; capacity=1 so Fan is turned away
        // We need to find a seed where both want to attend.
        // Superfan prob = 0.95, Fan prob = 0.75 with location=1.0
        // Try multiple seeds to find one where both want to attend
        let mut found_seed = None;
        for candidate_seed in 0..100 {
            let sf_roll = det_f32(candidate_seed, 0, 0);
            let fan_roll = det_f32(candidate_seed, 1, 0);
            if sf_roll < 0.95 && fan_roll < 0.75 {
                found_seed = Some(candidate_seed);
                break;
            }
        }
        let seed = found_seed.expect("Should find a seed where both want to attend");

        let inputs = RaceAttractionInputs {
            track_quality: 1.0,
            location_attractiveness: 1.0,
            capacity: 1,
            marketing: default_marketing(),
            ticket_price: 0,
            seed,
        };

        let result = simulate_race(&mut net, &inputs);
        assert_eq!(result.actual_attendance, 1);
        assert_eq!(result.turned_away, 1);

        // The Fan (id=1) was turned away — races_since_attended should still be 0
        let fan = net.people.iter().find(|p| p.id == 1).unwrap();
        assert_eq!(
            fan.races_since_attended, 0,
            "Turned-away person should not have decay incremented"
        );
    }

    #[test]
    fn test_capacity_overflow() {
        let mut net = FanNetwork::new_seeded(42);
        // Add many people
        for _ in 0..100 {
            net.add_aware(None);
        }

        let inputs = RaceAttractionInputs {
            track_quality: 1.0,
            location_attractiveness: 1.0,
            capacity: 5,
            marketing: MarketingEffects {
                aware_attendance_nudge: 0.85,
                ..default_marketing()
            },
            ticket_price: 0,
            seed: 42,
        };

        let result = simulate_race(&mut net, &inputs);
        assert!(result.demand > 5, "Should have demand > capacity");
        assert_eq!(result.actual_attendance, 5);
        assert_eq!(result.turned_away, result.demand - 5);
    }

    #[test]
    fn test_capacity_priority_by_tier() {
        let mut net = FanNetwork {
            people: vec![
                Person {
                    id: 0,
                    recruited_by: None,
                    tier: FanTier::Aware,
                    races_attended: 0,
                    races_since_attended: 0,
                    spread_count: 0,
                },
                Person {
                    id: 1,
                    recruited_by: None,
                    tier: FanTier::Superfan,
                    races_attended: 10,
                    races_since_attended: 0,
                    spread_count: 0,
                },
            ],
            next_id: 2,
        };

        // Find a seed where both want to attend
        let mut found_seed = None;
        for candidate_seed in 0..200 {
            let aware_prob: f32 = 0.15 * (0.5 + 0.5 * 1.0) + 0.85; // with nudge
            let sf_prob: f32 = 0.95 * (0.5 + 0.5 * 1.0);
            let aware_roll = det_f32(candidate_seed, 0, 0);
            let sf_roll = det_f32(candidate_seed, 1, 0);
            if aware_roll < aware_prob.min(1.0) && sf_roll < sf_prob.min(1.0) {
                found_seed = Some(candidate_seed);
                break;
            }
        }
        let seed = found_seed.expect("Should find seed where both want to attend");

        let inputs = RaceAttractionInputs {
            track_quality: 1.0,
            location_attractiveness: 1.0,
            capacity: 1,
            marketing: MarketingEffects {
                aware_attendance_nudge: 0.85,
                ..default_marketing()
            },
            ticket_price: 0,
            seed,
        };

        let result = simulate_race(&mut net, &inputs);
        assert_eq!(result.actual_attendance, 1);

        // Superfan should have attended (priority)
        let sf = net.people.iter().find(|p| p.id == 1).unwrap();
        assert_eq!(sf.races_attended, 11, "Superfan should have attended");
    }

    #[test]
    fn test_spreading_creates_new_aware() {
        let mut net = FanNetwork {
            people: vec![Person {
                id: 0,
                recruited_by: None,
                tier: FanTier::Fan,
                races_attended: 5,
                races_since_attended: 0,
                spread_count: 0,
            }],
            next_id: 1,
        };

        let initial_size = net.network_size();
        // Run multiple races with high spread settings
        for seed in 0..20 {
            let inputs = RaceAttractionInputs {
                track_quality: 1.0,
                location_attractiveness: 1.0,
                capacity: 1000,
                marketing: MarketingEffects {
                    spread_potency_mult: 3.0,
                    spread_volume_bonus: 5,
                    ..default_marketing()
                },
                ticket_price: 0,
                seed,
            };
            simulate_race(&mut net, &inputs);
        }

        assert!(
            net.network_size() > initial_size,
            "Network should have grown from spreading"
        );
        // Check that some new people were recruited by person 0
        let recruited = net
            .people
            .iter()
            .filter(|p| p.recruited_by == Some(0))
            .count();
        assert!(recruited > 0, "Should have recruited new aware people");
    }

    #[test]
    fn test_determinism_same_seed() {
        let mut net_a = FanNetwork::new_seeded(77);
        let mut net_b = FanNetwork::new_seeded(77);

        let inputs_a = default_inputs(123);
        let inputs_b = default_inputs(123);

        let result_a = simulate_race(&mut net_a, &inputs_a);
        let result_b = simulate_race(&mut net_b, &inputs_b);

        assert_eq!(result_a.demand, result_b.demand);
        assert_eq!(result_a.actual_attendance, result_b.actual_attendance);
        assert_eq!(result_a.promotions, result_b.promotions);
        assert_eq!(result_a.demotions, result_b.demotions);
        assert_eq!(result_a.removed, result_b.removed);
        assert_eq!(result_a.fan_count, result_b.fan_count);
        assert_eq!(result_a.network_size, result_b.network_size);
        assert_eq!(net_a.people.len(), net_b.people.len());
    }

    #[test]
    fn test_multi_race_sequence() {
        let mut net = FanNetwork::new_seeded(42);
        let initial_size = net.network_size();

        for race_num in 0..10 {
            let inputs = RaceAttractionInputs {
                track_quality: 0.6,
                location_attractiveness: 0.7,
                capacity: 500,
                marketing: MarketingEffects {
                    new_aware_count: 3,
                    spread_potency_mult: 1.0,
                    spread_volume_bonus: 0,
                    aware_attendance_nudge: 0.05,
                    decay_slowdown: false,
                },
                ticket_price: 0,
                seed: 1000 + race_num,
            };
            let result = simulate_race(&mut net, &inputs);
            assert!(result.network_size > 0, "Network should never be empty");
        }

        // After 10 races with marketing injection, network should have grown
        assert!(
            net.network_size() > initial_size,
            "Network should grow over 10 races with marketing"
        );
    }

    #[test]
    fn test_high_ticket_price_reduces_attendance() {
        // Same network, same seed — only difference is ticket price.
        let make_net = || FanNetwork {
            people: vec![
                Person { id: 0, recruited_by: None, tier: FanTier::Aware, races_attended: 0, races_since_attended: 0, spread_count: 0 },
                Person { id: 1, recruited_by: None, tier: FanTier::Attendee, races_attended: 2, races_since_attended: 0, spread_count: 0 },
                Person { id: 2, recruited_by: None, tier: FanTier::Fan, races_attended: 5, races_since_attended: 0, spread_count: 0 },
                Person { id: 3, recruited_by: None, tier: FanTier::Superfan, races_attended: 10, races_since_attended: 0, spread_count: 0 },
            ],
            next_id: 4,
        };

        // Accumulate demand across many seeds to smooth out randomness
        let mut total_demand_free = 0u32;
        let mut total_demand_expensive = 0u32;

        for seed in 0..100 {
            let mut net_free = make_net();
            let mut net_expensive = make_net();

            let free_inputs = RaceAttractionInputs {
                track_quality: 0.5,
                location_attractiveness: 0.5,
                capacity: 1000,
                marketing: default_marketing(),
                ticket_price: 0,
                seed,
            };
            let expensive_inputs = RaceAttractionInputs {
                track_quality: 0.5,
                location_attractiveness: 0.5,
                capacity: 1000,
                marketing: default_marketing(),
                ticket_price: 15,
                seed,
            };

            total_demand_free += simulate_race(&mut net_free, &free_inputs).demand;
            total_demand_expensive += simulate_race(&mut net_expensive, &expensive_inputs).demand;
        }

        assert!(
            total_demand_expensive < total_demand_free,
            "Expensive tickets (${}) should reduce demand: free={}, expensive={}",
            15, total_demand_free, total_demand_expensive
        );
    }

    #[test]
    fn test_high_hype_mitigates_price_penalty() {
        // With high hype, price penalty is smaller, so demand gap narrows
        let make_net = || FanNetwork {
            people: vec![
                Person { id: 0, recruited_by: None, tier: FanTier::Attendee, races_attended: 2, races_since_attended: 0, spread_count: 0 },
                Person { id: 1, recruited_by: None, tier: FanTier::Fan, races_attended: 5, races_since_attended: 0, spread_count: 0 },
                Person { id: 2, recruited_by: None, tier: FanTier::Superfan, races_attended: 10, races_since_attended: 0, spread_count: 0 },
            ],
            next_id: 3,
        };

        let ticket_price = 10;
        let mut demand_gap_low_hype = 0i32;
        let mut demand_gap_high_hype = 0i32;

        for seed in 0..200 {
            let mut net_free_low = make_net();
            let mut net_paid_low = make_net();
            let mut net_free_high = make_net();
            let mut net_paid_high = make_net();

            let free_low = simulate_race(&mut net_free_low, &RaceAttractionInputs {
                track_quality: 0.1, location_attractiveness: 0.1, capacity: 1000,
                marketing: default_marketing(), ticket_price: 0, seed,
            });
            let paid_low = simulate_race(&mut net_paid_low, &RaceAttractionInputs {
                track_quality: 0.1, location_attractiveness: 0.1, capacity: 1000,
                marketing: default_marketing(), ticket_price, seed,
            });
            let free_high = simulate_race(&mut net_free_high, &RaceAttractionInputs {
                track_quality: 1.0, location_attractiveness: 1.0, capacity: 1000,
                marketing: default_marketing(), ticket_price: 0, seed,
            });
            let paid_high = simulate_race(&mut net_paid_high, &RaceAttractionInputs {
                track_quality: 1.0, location_attractiveness: 1.0, capacity: 1000,
                marketing: default_marketing(), ticket_price, seed,
            });

            demand_gap_low_hype += free_low.demand as i32 - paid_low.demand as i32;
            demand_gap_high_hype += free_high.demand as i32 - paid_high.demand as i32;
        }

        assert!(
            demand_gap_low_hype > demand_gap_high_hype,
            "Price penalty should hurt more at low hype: low_hype_gap={}, high_hype_gap={}",
            demand_gap_low_hype, demand_gap_high_hype
        );
    }

    #[test]
    fn test_free_entry_boosts_attendance_over_neutral() {
        // Free ($0) should attract more than neutral ($3) pricing
        let make_net = || FanNetwork {
            people: vec![
                Person { id: 0, recruited_by: None, tier: FanTier::Aware, races_attended: 0, races_since_attended: 0, spread_count: 0 },
                Person { id: 1, recruited_by: None, tier: FanTier::Attendee, races_attended: 2, races_since_attended: 0, spread_count: 0 },
                Person { id: 2, recruited_by: None, tier: FanTier::Fan, races_attended: 5, races_since_attended: 0, spread_count: 0 },
                Person { id: 3, recruited_by: None, tier: FanTier::Superfan, races_attended: 10, races_since_attended: 0, spread_count: 0 },
            ],
            next_id: 4,
        };

        let mut total_demand_free = 0u32;
        let mut total_demand_neutral = 0u32;

        for seed in 0..100 {
            let mut net_free = make_net();
            let mut net_neutral = make_net();

            total_demand_free += simulate_race(&mut net_free, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 0, seed,
            }).demand;
            total_demand_neutral += simulate_race(&mut net_neutral, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 3, seed,
            }).demand;
        }

        assert!(
            total_demand_free > total_demand_neutral,
            "Free entry should attract more than $3 neutral: free={}, neutral={}",
            total_demand_free, total_demand_neutral
        );
    }

    #[test]
    fn test_neutral_pricing_no_penalty() {
        // $0 and $1 are the only neutral prices; $1 should match $0 minus the free bonus
        // We test that $1 produces identical demand across seeds (no penalty applied)
        // and that $5 is noticeably lower (penalty kicks in at $2+)
        let make_net = || FanNetwork {
            people: vec![
                Person { id: 0, recruited_by: None, tier: FanTier::Aware, races_attended: 0, races_since_attended: 0, spread_count: 0 },
                Person { id: 1, recruited_by: None, tier: FanTier::Attendee, races_attended: 2, races_since_attended: 0, spread_count: 0 },
                Person { id: 2, recruited_by: None, tier: FanTier::Fan, races_attended: 5, races_since_attended: 0, spread_count: 0 },
            ],
            next_id: 3,
        };

        let mut total_demand_1 = 0u32;
        let mut total_demand_5 = 0u32;

        for seed in 0..100 {
            let mut net_1 = make_net();
            let mut net_5 = make_net();

            total_demand_1 += simulate_race(&mut net_1, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 1, seed,
            }).demand;
            total_demand_5 += simulate_race(&mut net_5, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 5, seed,
            }).demand;
        }

        assert!(
            total_demand_1 > total_demand_5,
            "$1 (neutral) should have more demand than $5 (penalized): $1={}, $5={}",
            total_demand_1, total_demand_5
        );
    }

    #[test]
    fn test_price_penalty_plateaus() {
        // $50 ticket should not be dramatically worse than $20 (soft cap)
        let make_net = || FanNetwork {
            people: vec![
                Person { id: 0, recruited_by: None, tier: FanTier::Aware, races_attended: 0, races_since_attended: 0, spread_count: 0 },
                Person { id: 1, recruited_by: None, tier: FanTier::Attendee, races_attended: 2, races_since_attended: 0, spread_count: 0 },
                Person { id: 2, recruited_by: None, tier: FanTier::Fan, races_attended: 5, races_since_attended: 0, spread_count: 0 },
                Person { id: 3, recruited_by: None, tier: FanTier::Superfan, races_attended: 10, races_since_attended: 0, spread_count: 0 },
            ],
            next_id: 4,
        };

        let mut total_demand_20 = 0u32;
        let mut total_demand_50 = 0u32;

        for seed in 0..100 {
            let mut net_20 = make_net();
            let mut net_50 = make_net();

            total_demand_20 += simulate_race(&mut net_20, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 20, seed,
            }).demand;
            total_demand_50 += simulate_race(&mut net_50, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 50, seed,
            }).demand;
        }

        // The gap between $20 and $50 should be small relative to total demand,
        // proving the penalty plateaus rather than growing linearly.
        let gap = total_demand_20.saturating_sub(total_demand_50);
        assert!(
            gap <= total_demand_20 / 10,
            "Penalty should plateau: $20 demand={}, $50 demand={}, gap={} (should be <={})",
            total_demand_20, total_demand_50, gap, total_demand_20 / 10
        );
    }

    #[test]
    fn test_free_entry_boosts_spread() {
        let make_net = || FanNetwork {
            people: vec![
                Person { id: 0, recruited_by: None, tier: FanTier::Attendee, races_attended: 2, races_since_attended: 0, spread_count: 0 },
                Person { id: 1, recruited_by: None, tier: FanTier::Fan, races_attended: 5, races_since_attended: 0, spread_count: 0 },
                Person { id: 2, recruited_by: None, tier: FanTier::Superfan, races_attended: 10, races_since_attended: 0, spread_count: 0 },
            ],
            next_id: 3,
        };

        let mut total_spread_free = 0u32;
        let mut total_spread_paid = 0u32;

        for seed in 0..200 {
            let mut net_free = make_net();
            let mut net_paid = make_net();

            total_spread_free += simulate_race(&mut net_free, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 0, seed,
            }).new_aware_from_spread;
            total_spread_paid += simulate_race(&mut net_paid, &RaceAttractionInputs {
                track_quality: 0.5, location_attractiveness: 0.5, capacity: 1000,
                marketing: default_marketing(), ticket_price: 3, seed,
            }).new_aware_from_spread;
        }

        assert!(
            total_spread_free > total_spread_paid,
            "Free events should generate more word-of-mouth: free={}, paid={}",
            total_spread_free, total_spread_paid
        );
    }
}
