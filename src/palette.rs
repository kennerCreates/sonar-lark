#![allow(dead_code, clippy::approx_constant)]

use bevy::prelude::Color;

// Full 64-color palette — source of truth: assets/color/color_palette.hex
// All colors are fully opaque. Names match the palette file exactly (in SCREAMING_SNAKE_CASE).

// --- Neutrals & darks ---
pub const BLACK: Color = Color::srgb(0.000, 0.000, 0.000);           // #000000
pub const SMOKY_BLACK: Color = Color::srgb(0.020, 0.055, 0.102);     // #050e1a
pub const INDIGO: Color = Color::srgb(0.051, 0.129, 0.251);          // #0d2140
pub const STEEL: Color = Color::srgb(0.286, 0.318, 0.412);           // #495169
pub const STONE: Color = Color::srgb(0.412, 0.396, 0.439);           // #696570
pub const CHAINMAIL: Color = Color::srgb(0.502, 0.475, 0.502);       // #807980
pub const SIDEWALK: Color = Color::srgb(0.651, 0.604, 0.612);        // #a69a9c
pub const SAND: Color = Color::srgb(0.769, 0.733, 0.702);            // #c4bbb3
pub const TAN: Color = Color::srgb(0.851, 0.655, 0.596);             // #d9a798
pub const VANILLA: Color = Color::srgb(0.949, 0.949, 0.855);         // #f2f2da

// --- Blues ---
pub const SAPPHIRE: Color = Color::srgb(0.141, 0.224, 0.400);        // #243966
pub const TEAL: Color = Color::srgb(0.098, 0.357, 0.651);            // #195ba6
pub const HOMEWORLD: Color = Color::srgb(0.110, 0.459, 0.741);       // #1c75bd
pub const CAROLINA: Color = Color::srgb(0.090, 0.576, 0.902);        // #1793e6
pub const CERULEAN: Color = Color::srgb(0.145, 0.675, 0.961);        // #25acf5
pub const SKY: Color = Color::srgb(0.286, 0.761, 0.949);             // #49c2f2

// --- Purples ---
pub const VIOLET: Color = Color::srgb(0.306, 0.153, 0.549);          // #4e278c
pub const ROYAL: Color = Color::srgb(0.467, 0.231, 0.749);           // #773bbf
pub const ULTRAMARINE: Color = Color::srgb(0.208, 0.325, 0.651);     // #3553a6
pub const SLATE: Color = Color::srgb(0.345, 0.416, 0.769);           // #586ac4
pub const PERIWINKLE: Color = Color::srgb(0.494, 0.494, 0.949);      // #7e7ef2
pub const AMETHYST: Color = Color::srgb(0.639, 0.365, 0.851);        // #a35dd9
pub const ORCHID: Color = Color::srgb(0.792, 0.494, 0.949);          // #ca7ef2
pub const LILAC: Color = Color::srgb(0.886, 0.608, 0.980);           // #e29bfa
pub const LAVENDER: Color = Color::srgb(0.682, 0.533, 0.890);        // #ae88e3

// --- Pinks & reds ---
pub const BURGUNDY: Color = Color::srgb(0.278, 0.180, 0.243);        // #472e3e
pub const EGGPLANT: Color = Color::srgb(0.431, 0.259, 0.314);        // #6e4250
pub const GRAPE: Color = Color::srgb(0.522, 0.133, 0.392);           // #852264
pub const MAGENTA: Color = Color::srgb(0.702, 0.176, 0.490);         // #b32d7d
pub const PINK: Color = Color::srgb(0.851, 0.298, 0.557);            // #d94c8e
pub const BUBBLEGUM: Color = Color::srgb(0.922, 0.459, 0.561);       // #eb758f
pub const PALE_PINK: Color = Color::srgb(0.980, 0.733, 0.686);       // #fabbaf
pub const CHERRY: Color = Color::srgb(0.769, 0.047, 0.180);          // #c40c2e
pub const NEON_RED: Color = Color::srgb(0.961, 0.192, 0.255);        // #f53141
pub const SALMON: Color = Color::srgb(1.000, 0.439, 0.439);          // #ff7070
pub const PEACH: Color = Color::srgb(0.980, 0.596, 0.569);           // #fa9891
pub const MAROON: Color = Color::srgb(0.620, 0.298, 0.298);          // #9e4c4c

// --- Oranges ---
pub const TANGERINE: Color = Color::srgb(0.949, 0.384, 0.122);       // #f2621f
pub const PUMPKIN: Color = Color::srgb(0.859, 0.294, 0.086);         // #db4b16
pub const SUNFLOWER: Color = Color::srgb(0.961, 0.506, 0.133);       // #f58122
pub const DANDELION: Color = Color::srgb(0.980, 0.627, 0.196);       // #faa032

// --- Yellows ---
pub const LIMON: Color = Color::srgb(0.980, 0.851, 0.216);           // #fad937
pub const SUNSHINE: Color = Color::srgb(1.000, 0.725, 0.220);        // #ffb938
pub const GOLDENROD: Color = Color::srgb(0.902, 0.608, 0.133);       // #e69b22
pub const BRONZE: Color = Color::srgb(0.800, 0.502, 0.161);          // #cc8029
pub const LIME: Color = Color::srgb(0.800, 0.780, 0.239);            // #ccc73d
pub const ACID: Color = Color::srgb(0.702, 0.690, 0.176);            // #b3b02d
pub const AVOCADO: Color = Color::srgb(0.596, 0.612, 0.153);         // #989c27
pub const OLIVE: Color = Color::srgb(0.549, 0.502, 0.141);           // #8c8024

// --- Greens ---
pub const GRASS: Color = Color::srgb(0.580, 0.749, 0.188);           // #94bf30
pub const GREEN: Color = Color::srgb(0.333, 0.702, 0.231);           // #55b33b
pub const FROG: Color = Color::srgb(0.090, 0.612, 0.263);            // #179c43
pub const JUNGLE: Color = Color::srgb(0.024, 0.502, 0.318);          // #068051
pub const SPRUCE: Color = Color::srgb(0.067, 0.376, 0.380);          // #116061
pub const SEA_FOAM: Color = Color::srgb(0.627, 0.922, 0.659);        // #a0eba8
pub const MINT: Color = Color::srgb(0.486, 0.812, 0.604);            // #7ccf9a
pub const GRANNY_SMITH: Color = Color::srgb(0.361, 0.722, 0.533);    // #5cb888
pub const JADE: Color = Color::srgb(0.239, 0.631, 0.494);            // #3da17e
pub const SEAGREEN: Color = Color::srgb(0.126, 0.502, 0.424);        // #20806c

// --- Browns ---
pub const CLAY: Color = Color::srgb(0.678, 0.416, 0.271);            // #ad6a45
pub const DIRT: Color = Color::srgb(0.478, 0.369, 0.216);            // #7a5e37
pub const HAZELNUT: Color = Color::srgb(0.710, 0.549, 0.498);        // #b58c7f
pub const TOAST: Color = Color::srgb(0.620, 0.467, 0.404);           // #9e7767
pub const CLOVE: Color = Color::srgb(0.529, 0.365, 0.345);           // #875d58
