# TODO

## Backlog (Quality / Polish)
- [x] Display `gates_passed` in Results UI (data already in `RaceResultEntry.gates_passed`)
- [x] Directional gate validation — already implemented: `plane_crossing_check` rejects backward passes via signed-distance directionality, tested by `back_to_front_rejected` and `flipped_gate_forward`
- [x] Deferred `RaceProgress` insertion timing — not a real risk: AI is gated by `drones_are_active()` (false during countdown), gate detection early-exits if `RaceProgress` missing, and system `.chain()` ordering guarantees insertion before detection in the same frame

## Future (Post-MVP)
- [ ] Player-controlled drone (same throttle/pitch/roll/yaw interface as AI)
- [ ] Per-drone customization (motor thrust, weight, drag, frame size)
- [ ] Multiple obstacle types beyond gates
- [ ] Multi-lap races
- [ ] Terrain elevation
- [ ] Gamepad support
