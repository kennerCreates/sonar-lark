# Sonar Lark — Game Design Document

## Concept

**Drone racing league organizer.** You design courses, host races, and build hype to grow your league from nothing into a premier racing destination. The core creative acts are course design and event promotion — the races themselves are the payoff you watch unfold.

**Genre touchstones:** Motorsport Manager (watch simulated races), Game Dev Tycoon (build a thing → release it → see how it performs → improve), Townscaper (satisfying creative building).

---

## Core Loop (~20 minutes per cycle)

```
 BUILD COURSE ──► RACE DAY ──► HYPE PHASE ──► GROW LEAGUE
      │                │              │              │
 Place obstacles   Pre-simulated   Select key     Reputation ↑
 Design layout     Watch races     moments for    Fans ↑
 Balance difficulty Acrobatics!    highlight reel Money ↑
 Test spectacle    Camera control  Compose poster Attract pilots
                   Overtakes       Design merch   Unlock obstacles
                   Crashes                        Better courses
                   Photo finishes                      │
                                                       ▼
                                              NEXT CYCLE (harder,
                                              more options, better
                                              pilots, higher stakes)
```

**Minimum full playthrough:** ~12 courses, ~4 hours.

---

## Phase 1: Build Course

The course editor is the primary creative tool — not a side feature.

**Player learns across 12+ courses:**
- How gate placement creates tight turns (which trigger acrobatic maneuvers — visual spectacle)
- How difficulty balancing works — too easy is boring, too hard causes mass DNFs
- How obstacle variety and elevation changes create dramatic racing
- How to design for the camera — sight lines, dramatic reveals, close passes

**Progression unlocks:**
- New obstacle types (earned through reputation/money)
- Larger venue sizes
- More complex gate types
- Aesthetic props (crowds, lighting, decoration) that affect hype

**Key design constraint:** The course editor already exists and works well. The management layer wraps around it, giving purpose and progression to what's currently a sandbox.

---

## Phase 2: Race Day

**Architecture:** Pre-simulated + acrobatic splice + playback.

1. Simulation runs the full race using current physics engine (sub-second compute time)
2. Post-processing detects tight turns, splices in acrobatic maneuvers for skilled pilots
3. Player watches the race with camera controls (chase, FPV, spectator, cinematic)

**What the player sees:**
- Realistic flight physics (banking, drafting, speed variation, dirty air wobble)
- Acrobatic maneuvers on tight turns (Split-S, power loops) — frequency tied to pilot skill
- Overtakes, crashes, close calls, photo finishes
- The consequences of their course design playing out in real time

**What the player controls during playback:**
- Camera angle/mode
- Slow-mo / speed up / pause
- Nothing that affects the outcome — this is watching, not playing

**Skill expression through acrobatics:** Higher-skill pilots execute more acrobatic maneuvers and do them more cleanly. Lower-skill pilots take wide banking turns, lose speed in corners. The visual difference between a talented and mediocre pilot is immediately obvious — and the player attracted/collected those pilots.

---

## Phase 3: Hype Phase

Post-race creative curation. The player takes raw race footage and turns it into promotional material.

### Highlight Reel
- Player quickly selects key moments from the race (overtakes, flips, close finishes, crashes)
- Game auto-edits them into a highlight reel with transitions and music
- Light creative control — pick moments, order, maybe camera angles per moment
- Quality of the reel affects hype generation

### Poster / Promo Art
- Library of stamps, fonts, and actual photos from the last race
- Player composes promotional posters (drag-and-drop creative tool)
- Used to promote the next race event

### Merch
- Similar compose-from-library approach
- Merch generates revenue and fan engagement

**Design goal:** This phase should feel quick and satisfying, not tedious. ~3-5 minutes. The creative tools should be fun to use with a low skill floor and high expression ceiling.

---

## Phase 4: League Growth

Races and hype generate two intertwined currencies:

### Reputation
- Grows from exciting races, good highlight reels, course quality
- Attracts better pilots (higher skill, more crowd appeal)
- Unlocks bigger venues, new obstacle types, new creative tools
- Determines league tier and progression milestones

### Money
- Generated from ticket sales (fan count), sponsorships (reputation-dependent), merch sales
- Spent on: new obstacles, venue upgrades, pilot attraction bonuses, aesthetic props
- Economic pressure: spending on spectacle should pay off in fans and reputation

### Pilot Attraction
- Pilots are drawn to leagues with high reputation
- Each pilot has: skill level (affects race performance + acrobatic frequency), crowd appeal (affects fan engagement), visual identity (portrait, gamertag)
- Collecting pilot cards is a lightweight but satisfying progression mechanic
- Pilots are not deeply managed — they show up, they race, they have personality

---

## Endgame

### Primary: Season Finale
- 12+ race season building toward a championship event
- The finale is the culmination — your best course, your best pilots, maximum stakes
- Success measured by: fan count, reputation tier, revenue, championship drama

### Stretch: Endless Sandbox
- After the season, keep going — new seasons with higher expectations
- Architecture should not preclude this (no hard "game over" state)
- Higher tiers with new mechanics, pilots, obstacles

---

## Existing Systems → Management Role

| Current System | Management Role |
|---|---|
| Course editor | Core creative gameplay — now with progression unlocks and purpose |
| Obstacle workshop | Feeds into course editor — new obstacles unlocked through reputation |
| Drone physics | Powers pre-simulation — unchanged, runs headless |
| Acrobatic splice | NEW — post-processing on simulated trajectories |
| Pilot roster | Pilot attraction and collection — lightweight management |
| Pilot personality/skill | Drives drone behavior variation + acrobatic frequency |
| Race gate system | Unchanged — drives race scoring and drama detection |
| Leaderboard/results | Feeds into highlight reel moment detection |

---

## Technical Architecture Implications

### Pre-simulation Pipeline
```
Course + Pilots + Seed
        │
   Physics Simulation (headless, current engine)
        │
   Raw Trajectory Data (positions, rotations, velocities per drone per tick)
        │
   Event Detection (gate passes, overtakes, near-misses, crashes)
        │
   Acrobatic Splice (replace tight-turn segments with flip animations)
        │
   Final Trajectory + Event Timeline
        │
   Playback Engine (interpolate transforms, fire events, drive cameras)
```

### Key Data
- **Trajectory buffer:** ~2.7 MB per race (12 drones, 90s, 64Hz). Trivial.
- **Event timeline:** Timestamped list of gate passes, overtakes, crashes, acrobatic maneuvers.
- **Race seed:** Single u32 that reproduces the simulation deterministically.
- **Highlight candidates:** Scored events for the hype phase UI to present.

### Replay Support (future)
Falls out naturally — a replay is just re-playing the trajectory buffer with different camera settings. No re-simulation needed. Store trajectory + events + seed alongside the course save file.

---

## Open Questions

1. **Audience scoring model** — How exactly does the game evaluate "how entertaining was that race"? Needs design (number of overtakes, acrobatic maneuvers, close finishes, DNFs, etc.)
2. ~~**Pilot card collection**~~ — Resolved: "Gotta catch 'em all" collection as progress tracking. Cards are trophies, not strategic resources.
3. **Course rating** — Does the game give feedback on course design before the race (predicted difficulty, spectacle score) or only after?
4. **Hype tool scope** — How much creative control in posters/merch before it becomes a distraction from the core loop?
5. **Difficulty curve** — How does the game teach course design? Tutorial races? Example courses? Feedback systems?
