# Rendering

## CelMaterial

All visible geometry (ground, obstacles, drones) uses `CelMaterial` — a custom material with cel-shading, halftone dot transitions, and hue-shifted highlights/shadows.

- `CelLightDir` resource stores the world-space light direction, computed once at startup.
- Use `cel_material_from_color(base_color, light_dir)` to create materials.
- Custom WGSL shaders live in `assets/shaders/`.

## Skybox

`SkyboxMaterial` on an inverted sphere (front-face culled), procedural TRON-style night sky with stars, moon, and neon horizon glow.

## Exceptions

- Explosion particles use `StandardMaterial` (unlit emissive), not `CelMaterial`.

## Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `CelMaterial` | `rendering/cel_material` | Cel-shading material asset |
| `SkyboxMaterial` | `rendering/skybox` | Procedural night sky material |
| `CelLightDir` | `rendering/mod` | Shared world-space light direction resource |
| `SkyboxEntity` | `rendering/skybox` | Marker on the skybox sphere entity |
