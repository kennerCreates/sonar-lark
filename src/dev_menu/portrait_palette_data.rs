use bevy::prelude::Color;

use crate::palette;

/// Extract sRGB components from a `Color::srgb()` constant at compile time.
const fn color_to_rgb(c: Color) -> [f32; 3] {
    match c {
        Color::Srgba(srgba) => [srgba.red, srgba.green, srgba.blue],
        _ => [0.0, 0.0, 0.0],
    }
}

// 64 palette colors: (name, sRGB [0..1])
pub const PALETTE_COLORS: [(&str, [f32; 3]); 64] = [
    ("Black", color_to_rgb(palette::BLACK)),
    ("Smoky Black", color_to_rgb(palette::SMOKY_BLACK)),
    ("Indigo", color_to_rgb(palette::INDIGO)),
    ("Steel", color_to_rgb(palette::STEEL)),
    ("Stone", color_to_rgb(palette::STONE)),
    ("Chainmail", color_to_rgb(palette::CHAINMAIL)),
    ("Sidewalk", color_to_rgb(palette::SIDEWALK)),
    ("Sand", color_to_rgb(palette::SAND)),
    ("Tan", color_to_rgb(palette::TAN)),
    ("Vanilla", color_to_rgb(palette::VANILLA)),
    // Blues
    ("Sapphire", color_to_rgb(palette::SAPPHIRE)),
    ("Teal", color_to_rgb(palette::TEAL)),
    ("Homeworld", color_to_rgb(palette::HOMEWORLD)),
    ("Carolina", color_to_rgb(palette::CAROLINA)),
    ("Cerulean", color_to_rgb(palette::CERULEAN)),
    ("Sky", color_to_rgb(palette::SKY)),
    // Purples
    ("Violet", color_to_rgb(palette::VIOLET)),
    ("Royal", color_to_rgb(palette::ROYAL)),
    ("Ultramarine", color_to_rgb(palette::ULTRAMARINE)),
    ("Slate", color_to_rgb(palette::SLATE)),
    ("Periwinkle", color_to_rgb(palette::PERIWINKLE)),
    ("Amethyst", color_to_rgb(palette::AMETHYST)),
    ("Orchid", color_to_rgb(palette::ORCHID)),
    ("Lilac", color_to_rgb(palette::LILAC)),
    ("Lavender", color_to_rgb(palette::LAVENDER)),
    // Pinks & reds
    ("Burgundy", color_to_rgb(palette::BURGUNDY)),
    ("Eggplant", color_to_rgb(palette::EGGPLANT)),
    ("Grape", color_to_rgb(palette::GRAPE)),
    ("Magenta", color_to_rgb(palette::MAGENTA)),
    ("Pink", color_to_rgb(palette::PINK)),
    ("Bubblegum", color_to_rgb(palette::BUBBLEGUM)),
    ("Pale Pink", color_to_rgb(palette::PALE_PINK)),
    ("Cherry", color_to_rgb(palette::CHERRY)),
    ("Neon Red", color_to_rgb(palette::NEON_RED)),
    ("Salmon", color_to_rgb(palette::SALMON)),
    ("Peach", color_to_rgb(palette::PEACH)),
    ("Maroon", color_to_rgb(palette::MAROON)),
    // Oranges
    ("Tangerine", color_to_rgb(palette::TANGERINE)),
    ("Pumpkin", color_to_rgb(palette::PUMPKIN)),
    ("Sunflower", color_to_rgb(palette::SUNFLOWER)),
    ("Dandelion", color_to_rgb(palette::DANDELION)),
    // Yellows
    ("Limon", color_to_rgb(palette::LIMON)),
    ("Sunshine", color_to_rgb(palette::SUNSHINE)),
    ("Goldenrod", color_to_rgb(palette::GOLDENROD)),
    ("Bronze", color_to_rgb(palette::BRONZE)),
    ("Lime", color_to_rgb(palette::LIME)),
    ("Acid", color_to_rgb(palette::ACID)),
    ("Avocado", color_to_rgb(palette::AVOCADO)),
    ("Olive", color_to_rgb(palette::OLIVE)),
    // Greens
    ("Grass", color_to_rgb(palette::GRASS)),
    ("Green", color_to_rgb(palette::GREEN)),
    ("Frog", color_to_rgb(palette::FROG)),
    ("Jungle", color_to_rgb(palette::JUNGLE)),
    ("Spruce", color_to_rgb(palette::SPRUCE)),
    ("Sea Foam", color_to_rgb(palette::SEA_FOAM)),
    ("Mint", color_to_rgb(palette::MINT)),
    ("Granny Smith", color_to_rgb(palette::GRANNY_SMITH)),
    ("Jade", color_to_rgb(palette::JADE)),
    ("Seagreen", color_to_rgb(palette::SEAGREEN)),
    // Browns
    ("Clay", color_to_rgb(palette::CLAY)),
    ("Dirt", color_to_rgb(palette::DIRT)),
    ("Hazelnut", color_to_rgb(palette::HAZELNUT)),
    ("Toast", color_to_rgb(palette::TOAST)),
    ("Clove", color_to_rgb(palette::CLOVE)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_colors_count() {
        assert_eq!(PALETTE_COLORS.len(), 64);
    }

    #[test]
    fn palette_colors_in_range() {
        for (name, rgb) in &PALETTE_COLORS {
            for (i, ch) in rgb.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(ch),
                    "{name} channel {i} out of range: {ch}"
                );
            }
        }
    }
}
