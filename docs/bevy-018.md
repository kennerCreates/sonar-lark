# Bevy 0.18 API Specifics

Consult this before writing any Bevy code. These APIs changed from previous versions and training data may be stale.

## Component & Bundle Changes
- `Mesh3d` / `MeshMaterial3d` components (not `PbrBundle` or `MaterialMeshBundle`)
- `SceneRoot` (not `SceneBundle`)
- `set_parent_in_place()` (not `set_parent()`)

## Spawning
- `ChildSpawnerCommands` (not `ChildBuilder`) for `with_children` closures in commands context

## Input
- `AccumulatedMouseMotion` / `AccumulatedMouseScroll` from `bevy::input::mouse` (not in prelude)
- `KeyboardInput` from `bevy::input::keyboard`

## Events
- `MessageReader<T>` (not `EventReader<T>`) for reading events

## Assets
- `Gltf::named_nodes` / `named_scenes` use `Box<str>` keys (not `String`)

## System Registration
- System tuples max ~12 elements for `run_if`; split larger groups into multiple `add_systems` calls
