use std::collections::HashMap;

use bevy::prelude::*;

const MASTER_SVG_PATH: &str = "assets/portraits/pilot-portraits.svg";

/// SVG fragment content extracted from the master portrait SVG, keyed by
/// Inkscape layer label (e.g. `"face_oval"`, `"hair_back_beanie"`).
///
/// Each entry stores the inner SVG content of a layer group, ready to be
/// composed into a full portrait SVG by `assemble_svg`.
#[derive(Resource, Default)]
pub struct PortraitParts {
    fragments: HashMap<String, String>,
}

impl PortraitParts {
    /// Look up a fragment by slot prefix and variant, e.g. `get("face", "oval")`.
    pub fn get(&self, slot: &str, variant: &str) -> Option<&str> {
        let key = format!("{slot}_{variant}");
        self.fragments.get(&key).map(|s| s.as_str())
    }

    /// Look up a fragment by its exact layer label (e.g. `"background"`).
    pub fn get_by_label(&self, label: &str) -> Option<&str> {
        self.fragments.get(label).map(|s| s.as_str())
    }

    /// Insert a fragment directly (used for testing).
    #[cfg(test)]
    pub fn insert(&mut self, key: impl Into<String>, content: impl Into<String>) {
        self.fragments.insert(key.into(), content.into());
    }
}

/// Startup system that parses the master portrait SVG and populates
/// the `PortraitParts` resource.
pub fn load_portrait_parts(mut commands: Commands) {
    let mut parts = PortraitParts::default();
    match std::fs::read_to_string(MASTER_SVG_PATH) {
        Ok(svg_text) => {
            parts.fragments = parse_master_svg(&svg_text);
            info!(
                "Loaded {} portrait layers from {}",
                parts.fragments.len(),
                MASTER_SVG_PATH
            );
        }
        Err(e) => {
            warn!("Could not read master portrait SVG {MASTER_SVG_PATH}: {e}");
        }
    }
    commands.insert_resource(parts);
}

/// Re-read the master SVG from disk. Used by the hot-reload system.
pub fn reload_portrait_parts(parts: &mut PortraitParts) {
    parts.fragments.clear();
    match std::fs::read_to_string(MASTER_SVG_PATH) {
        Ok(svg_text) => {
            parts.fragments = parse_master_svg(&svg_text);
            info!(
                "Reloaded {} portrait layers from disk",
                parts.fragments.len()
            );
        }
        Err(e) => {
            warn!("Could not reload master portrait SVG: {e}");
        }
    }
}

/// Hot-reload system: press F6 to re-read the master SVG and invalidate
/// the portrait cache so portraits get re-rasterized.
#[cfg(debug_assertions)]
pub fn hot_reload_portraits(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut parts: ResMut<PortraitParts>,
    mut cache: Option<ResMut<super::cache::PortraitCache>>,
) {
    if keyboard.just_pressed(KeyCode::F6) {
        reload_portrait_parts(&mut parts);
        if let Some(ref mut cache) = cache {
            cache.invalidate();
        }
        info!("Portrait parts reloaded from disk");
    }
}

// ---------------------------------------------------------------------------
// Master SVG layer parser
// ---------------------------------------------------------------------------

/// Parse a master Inkscape SVG and extract each layer's inner content.
///
/// Looks for `<g` tags with `inkscape:groupmode="layer"` and extracts the
/// `inkscape:label` as the key and all child content as the value.
fn parse_master_svg(svg_text: &str) -> HashMap<String, String> {
    let mut parts = HashMap::new();
    let mut search_from = 0;

    while let Some(layer_start) = find_layer_group(svg_text, search_from) {
        let Some(label) = extract_inkscape_label(svg_text, layer_start) else {
            search_from = layer_start + 1;
            continue;
        };

        let Some(tag_close) = svg_text[layer_start..].find('>') else {
            search_from = layer_start + 1;
            continue;
        };
        let content_start = layer_start + tag_close + 1;

        let Some(content_end) = find_matching_close_g(svg_text, content_start) else {
            search_from = content_start;
            continue;
        };

        let content = svg_text[content_start..content_end].trim();
        if !content.is_empty() {
            parts.insert(label, content.to_string());
        }

        search_from = content_end;
    }

    parts
}

/// Find the start position of the next `<g` tag that has `inkscape:groupmode="layer"`.
fn find_layer_group(svg_text: &str, from: usize) -> Option<usize> {
    let mut pos = from;
    while let Some(g_offset) = svg_text[pos..].find("<g") {
        let g_start = pos + g_offset;
        // Find the end of this opening tag
        let tag_end = svg_text[g_start..].find('>')?;
        let tag_text = &svg_text[g_start..g_start + tag_end + 1];

        if tag_text.contains("inkscape:groupmode=\"layer\"") {
            return Some(g_start);
        }
        pos = g_start + 2; // skip past "<g"
    }
    None
}

/// Extract the `inkscape:label="..."` value from a tag starting at `tag_start`.
fn extract_inkscape_label(svg_text: &str, tag_start: usize) -> Option<String> {
    let tag_end = svg_text[tag_start..].find('>')?;
    let tag_text = &svg_text[tag_start..tag_start + tag_end + 1];

    let label_attr = "inkscape:label=\"";
    let label_start = tag_text.find(label_attr)?;
    let value_start = label_start + label_attr.len();
    let value_end = tag_text[value_start..].find('"')?;
    Some(tag_text[value_start..value_start + value_end].to_string())
}

/// Find the position of the matching `</g>` for content starting at `content_start`.
/// Tracks nesting depth to handle child `<g>` elements.
fn find_matching_close_g(svg_text: &str, content_start: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut pos = content_start;

    while pos < svg_text.len() {
        if svg_text[pos..].starts_with("</g>") || svg_text[pos..].starts_with("</g ") {
            if depth == 0 {
                return Some(pos);
            }
            depth -= 1;
            pos += 4;
        } else if svg_text[pos..].starts_with("<g ") || svg_text[pos..].starts_with("<g>") {
            depth += 1;
            pos += 2;
        } else {
            pos += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_master_svg_extracts_layers() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
<g inkscape:groupmode="layer" id="layer1" inkscape:label="face_oval" style="display:inline">
<path d="M10 10" fill="#000000"/>
</g>
<g inkscape:groupmode="layer" id="layer2" inkscape:label="eyes_normal" style="display:inline">
<circle cx="8" cy="8" r="2" fill="#808080"/>
</g>
</svg>"##;

        let parts = parse_master_svg(svg);
        assert_eq!(parts.len(), 2);
        assert!(parts.get("face_oval").unwrap().contains("path"));
        assert!(parts.get("eyes_normal").unwrap().contains("circle"));
    }

    #[test]
    fn parse_master_svg_ignores_non_layer_groups() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
<g id="not-a-layer"><rect width="10" height="10"/></g>
<g inkscape:groupmode="layer" id="layer1" inkscape:label="face_round" style="display:inline">
<ellipse cx="10" cy="10" rx="5" ry="6" fill="#000000"/>
</g>
</svg>"##;

        let parts = parse_master_svg(svg);
        assert_eq!(parts.len(), 1);
        assert!(parts.contains_key("face_round"));
        assert!(!parts.contains_key("not-a-layer"));
    }

    #[test]
    fn parse_master_svg_handles_nested_children() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
<g inkscape:groupmode="layer" id="layer1" inkscape:label="face_angular" style="display:inline">
<g transform="translate(-9,-11)">
<path d="M20 20" fill="#000000"/>
<path d="M21 21" stroke="#ffffff"/>
</g>
</g>
</svg>"##;

        let parts = parse_master_svg(svg);
        assert_eq!(parts.len(), 1);
        let content = parts.get("face_angular").unwrap();
        assert!(content.contains("translate(-9,-11)"));
        assert!(content.contains("M20 20"));
        assert!(content.contains("M21 21"));
    }

    #[test]
    fn parse_master_svg_empty_layer_skipped() {
        let svg = r#"<svg viewBox="0 0 20 20">
<g inkscape:groupmode="layer" id="layer1" inkscape:label="empty_layer" style="display:none"></g>
<g inkscape:groupmode="layer" id="layer2" inkscape:label="has_content" style="display:inline">
<rect width="5" height="5"/>
</g>
</svg>"#;

        let parts = parse_master_svg(svg);
        assert_eq!(parts.len(), 1);
        assert!(parts.contains_key("has_content"));
    }

    #[test]
    fn portrait_parts_get() {
        let mut parts = PortraitParts::default();
        parts.insert("face_oval", "<ellipse/>");
        assert_eq!(parts.get("face", "oval"), Some("<ellipse/>"));
        assert_eq!(parts.get("face", "round"), None);
    }

    #[test]
    fn portrait_parts_get_by_label() {
        let mut parts = PortraitParts::default();
        parts.insert("background", "<rect/>");
        assert_eq!(parts.get_by_label("background"), Some("<rect/>"));
        assert_eq!(parts.get_by_label("nonexistent"), None);
    }

    #[test]
    fn load_from_real_svg_if_available() {
        let path = std::path::Path::new(MASTER_SVG_PATH);
        if !path.exists() {
            return; // skip if running from a different working directory
        }
        let svg_text = std::fs::read_to_string(path).unwrap();
        let parts = parse_master_svg(&svg_text);
        // Should have at least the core layers
        assert!(parts.len() >= 20, "Expected >= 20 layers, got {}", parts.len());
        assert!(parts.contains_key("background"));
        assert!(parts.contains_key("face_round"));
        assert!(parts.contains_key("face_oval"));
        assert!(parts.contains_key("eyes_normal"));
        assert!(parts.contains_key("hair_front_mohawk"));
        assert!(parts.contains_key("shirt_crew"));
    }

    #[test]
    fn find_layer_group_finds_first() {
        let svg = r#"<g id="no"><g inkscape:groupmode="layer" inkscape:label="yes">"#;
        let pos = find_layer_group(svg, 0);
        assert!(pos.is_some());
        assert!(svg[pos.unwrap()..].contains("inkscape:label=\"yes\""));
    }

    #[test]
    fn extract_inkscape_label_extracts_value() {
        let tag = r#"<g inkscape:groupmode="layer" id="l1" inkscape:label="face_square" style="display:inline">"#;
        let label = extract_inkscape_label(tag, 0);
        assert_eq!(label, Some("face_square".to_string()));
    }
}
