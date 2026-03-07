use serde::{Deserialize, Serialize};

use super::fan_network::{FanNetwork, FanTier, RaceAttractionResult};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum BountyId {
    Attendees5,
    Attendees10,
    Attendees25,
    FirstFan,
    FirstSuperfan,
}

pub struct BountyDef {
    pub id: BountyId,
    pub label: &'static str,
    pub description: &'static str,
    pub reward: f32,
}

pub const BOUNTIES: &[BountyDef] = &[
    BountyDef {
        id: BountyId::Attendees5,
        label: "5 Attendees",
        description: "Have at least 5 people attend a race",
        reward: 50.0,
    },
    BountyDef {
        id: BountyId::Attendees10,
        label: "10 Attendees",
        description: "Have at least 10 people attend a race",
        reward: 100.0,
    },
    BountyDef {
        id: BountyId::Attendees25,
        label: "25 Attendees",
        description: "Have at least 25 people attend a race",
        reward: 200.0,
    },
    BountyDef {
        id: BountyId::FirstFan,
        label: "First Fan",
        description: "Have someone become a Fan",
        reward: 100.0,
    },
    BountyDef {
        id: BountyId::FirstSuperfan,
        label: "First Superfan",
        description: "Have someone become a Superfan",
        reward: 500.0,
    },
];

pub fn is_bounty_met(id: BountyId, attraction: &RaceAttractionResult, network: &FanNetwork) -> bool {
    match id {
        BountyId::Attendees5 => attraction.actual_attendance >= 5,
        BountyId::Attendees10 => attraction.actual_attendance >= 10,
        BountyId::Attendees25 => attraction.actual_attendance >= 25,
        BountyId::FirstFan => network
            .people
            .iter()
            .any(|p| matches!(p.tier, FanTier::Fan | FanTier::Superfan)),
        BountyId::FirstSuperfan => network.people.iter().any(|p| p.tier == FanTier::Superfan),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::fan_network::{FanNetwork, FanTier, Person, RaceAttractionResult};

    fn make_result(attendance: u32) -> RaceAttractionResult {
        RaceAttractionResult {
            demand: attendance,
            actual_attendance: attendance,
            turned_away: 0,
            new_aware_from_spread: 0,
            promotions: 0,
            demotions: 0,
            removed: 0,
            fan_count: 0,
            network_size: attendance,
        }
    }

    fn empty_network() -> FanNetwork {
        FanNetwork::default()
    }

    fn network_with_tier(tier: FanTier) -> FanNetwork {
        let mut net = FanNetwork::default();
        net.people.push(Person {
            id: 0,
            recruited_by: None,
            tier,
            races_attended: 5,
            races_since_attended: 0,
            spread_count: 0,
        });
        net
    }

    #[test]
    fn attendance_bounties() {
        let net = empty_network();
        assert!(!is_bounty_met(BountyId::Attendees5, &make_result(4), &net));
        assert!(is_bounty_met(BountyId::Attendees5, &make_result(5), &net));
        assert!(!is_bounty_met(BountyId::Attendees10, &make_result(9), &net));
        assert!(is_bounty_met(BountyId::Attendees10, &make_result(10), &net));
        assert!(!is_bounty_met(BountyId::Attendees25, &make_result(24), &net));
        assert!(is_bounty_met(BountyId::Attendees25, &make_result(25), &net));
    }

    #[test]
    fn first_fan_bounty() {
        let result = make_result(0);
        assert!(!is_bounty_met(BountyId::FirstFan, &result, &empty_network()));
        assert!(is_bounty_met(BountyId::FirstFan, &result, &network_with_tier(FanTier::Fan)));
        assert!(is_bounty_met(BountyId::FirstFan, &result, &network_with_tier(FanTier::Superfan)));
    }

    #[test]
    fn first_superfan_bounty() {
        let result = make_result(0);
        assert!(!is_bounty_met(BountyId::FirstSuperfan, &result, &empty_network()));
        assert!(!is_bounty_met(BountyId::FirstSuperfan, &result, &network_with_tier(FanTier::Fan)));
        assert!(is_bounty_met(BountyId::FirstSuperfan, &result, &network_with_tier(FanTier::Superfan)));
    }
}
