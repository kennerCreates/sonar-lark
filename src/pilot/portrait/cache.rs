use std::collections::HashMap;

use bevy::prelude::*;

use crate::pilot::roster::PilotRoster;
use crate::pilot::{PilotId, SelectedPilots};

use super::rasterize::rasterize_portrait;

const PORTRAIT_SIZE: u32 = 48;

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
}

/// Rasterize portraits for all selected pilots and insert/update the cache.
/// Runs `OnEnter(Race)` after pilot selection, guarded by
/// `run_if(resource_exists::<SelectedPilots>)`.
pub fn setup_portrait_cache(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    selected: Res<SelectedPilots>,
    roster: Res<PilotRoster>,
    existing_cache: Option<Res<PortraitCache>>,
) {
    let mut cache = existing_cache
        .map(|c| PortraitCache {
            portraits: c.portraits.clone(),
        })
        .unwrap_or_default();

    for sel in &selected.pilots {
        if cache.portraits.contains_key(&sel.pilot_id) {
            continue;
        }

        let Some(pilot) = roster.get(sel.pilot_id) else {
            warn!(
                "Pilot {:?} not found in roster, skipping portrait",
                sel.pilot_id
            );
            continue;
        };

        let image = rasterize_portrait(&pilot.portrait, pilot.color_scheme.primary, PORTRAIT_SIZE);
        let handle = images.add(image);
        cache.portraits.insert(sel.pilot_id, handle);
    }

    commands.insert_resource(cache);
}
