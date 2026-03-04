use std::collections::HashMap;

use bevy::prelude::*;

use crate::pilot::roster::PilotRoster;
use crate::pilot::{PilotId, SelectedPilots};

use super::loader::PortraitParts;
use super::rasterize::rasterize_portrait;

const PORTRAIT_SIZE: u32 = 512;

/// Cached portrait textures keyed by `PilotId`. Persists across races so
/// portraits only need to be rasterized once per pilot per session.
#[derive(Resource, Default)]
pub struct PortraitCache {
    portraits: HashMap<PilotId, Handle<Image>>,
}

impl PortraitCache {
    pub fn get(&self, pilot_id: PilotId) -> Option<Handle<Image>> {
        self.portraits.get(&pilot_id).cloned()
    }

    /// Clear all cached portraits, forcing re-rasterization on next race entry.
    pub fn invalidate(&mut self) {
        self.portraits.clear();
    }
}

/// Rasterize portraits for all selected pilots and insert/update the cache.
/// Runs `OnEnter(Race)` after pilot selection, guarded by
/// `run_if(resource_exists::<PortraitParts>)`.
pub fn setup_portrait_cache(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    selected: Res<SelectedPilots>,
    roster: Res<PilotRoster>,
    parts: Res<PortraitParts>,
    existing_cache: Option<Res<PortraitCache>>,
) {
    let mut cache = existing_cache
        .map(|c| PortraitCache {
            portraits: c.portraits.clone(),
        })
        .unwrap_or_default();

    // Invalidate all cached portraits since drone colors change each race.
    cache.invalidate();

    for sel in &selected.pilots {
        let Some(pilot) = roster.get(sel.pilot_id) else {
            warn!(
                "Pilot {:?} not found in roster, skipping portrait",
                sel.pilot_id
            );
            continue;
        };

        // Use the race-assigned color (not the pilot's stored color_scheme)
        let srgba = sel.color.to_srgba();
        let drone_color = [srgba.red, srgba.green, srgba.blue];

        let image = rasterize_portrait(
            &pilot.portrait,
            drone_color,
            PORTRAIT_SIZE,
            &parts,
        );
        let handle = images.add(image);
        cache.portraits.insert(sel.pilot_id, handle);
    }

    commands.insert_resource(cache);
}
