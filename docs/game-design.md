# Sonar Lark — Game Design Document

## Concept

**Drone racing league organizer.** You design courses, host races, and build hype to grow your league from nothing into a premier racing destination. The core creative acts are course design and event promotion — the races themselves are the payoff you watch unfold.

**Genre touchstones:** Motorsport Manager (watch simulated races), Game Dev Tycoon (build a thing → release it → see how it performs → improve), Townscaper (satisfying creative building).

---

## Core Loop (~20 minutes per cycle)

```
 BUILD COURSE ──► RACE DAY ──► HYPE PHASE ──► GROW LEAGUE
      │                │              │              │
 Place obstacles   Choreographed  Select key     Reputation ↑
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

**Architecture:** Choreographed spline racing with scripted outcomes.

The race outcome (finish order, crashes, acrobatics) is predetermined from pilot skill, personality, course difficulty, and randomness. Drones follow their per-drone racing line splines at curvature-based speeds with pacing adjustments to hit scripted finish times. Acrobatic maneuvers are procedural rotation keyframes + position offsets at tight turns. The player watches the scripted race play out in real-time with full camera controls.

This is "WWE, not true wrestling" — the spectacle is authored, not emergent. The drama comes from the script generator, which translates pilot skill and course design into exciting race narratives.

**How it works:**
1. Race script generated from pilot data + course geometry + randomness (instant, on race start)
2. Each drone follows its unique spline at a paced speed to hit its scripted finish time
3. Acrobatics trigger at tight turns for skilled pilots (rotation keyframes + altitude dip/climb)
4. Crashes trigger at scripted moments (obstacle collisions, drone-on-drone collisions)
5. All events (gate passes, finishes, crashes) fire from spline progress thresholds

**What the player sees:**
- Drones banking into turns with curvature-derived tilt angles
- Acrobatic maneuvers at tight turns — Split-S (altitude dip), power loops (climb), aggressive banks
- Overtakes where faster drones catch slower ones on straights
- Crashes at tight turns (obstacle clips, mid-air collisions) with ballistic arcs and explosions
- Attitude jitter, dirty air wobble, position micro-drift for visual realism
- The consequences of their course design: tight turns create acrobatics AND crashes

**What the player controls:**
- Camera angle/mode (Chase, FPV, Spectator, placed course cameras)
- Nothing that affects the outcome — this is watching, not playing
- (Future: slow-mo / speed up / pause)

**Skill expression through acrobatics:** Higher-skill pilots execute more acrobatic maneuvers and do them more cleanly (smooth rotation, confident entry). Lower-skill pilots take wide banking turns. Reckless pilots attempt flips and sometimes crash. The visual difference between a talented and mediocre pilot is immediately obvious — and the player attracted/collected those pilots.

**Course design feedback loop:** Tight turns (>100° direction change between gates) trigger acrobatics for skilled pilots but also increase crash probability. The player learns that dramatic courses create exciting races but risk losing pilots to DNFs. Elevation changes between gates determine maneuver type (Split-S vs power loop). This gives the course editor mechanical depth — gate placement directly shapes the race spectacle.

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
| Per-drone spline generation | Racing line variation from pilot skill/personality — unchanged |
| Choreographed playback | NEW — spline-following with scripted pacing, replaces physics during races |
| Race script generator | NEW — predetermines outcome from pilot data + course + randomness |
| Acrobatic maneuvers | NEW — rotation keyframes + position offsets at tight turns |
| Pilot roster | Pilot attraction and collection — lightweight management |
| Pilot personality/skill | Drives finish order, crash probability, acrobatic frequency, visual smoothness |
| Race gate system | Gate passes detected from spline progress thresholds (not live physics) |
| Leaderboard/results | Feeds into highlight reel moment detection |
| Drone physics | Used for post-race wandering and editor preview — not active during races |

---

## Technical Architecture Implications

### Choreographed Race Pipeline
```
Course + Pilots + Seed
        │
   Race Script Generator (instant — pure math)
   ├── Estimate lap times from spline curvature + pilot skill
   ├── Assign finish order (skill-based + random perturbation)
   ├── Assign crashes (tight turns × low skill × personality)
   ├── Assign acrobatics (tight turns × high skill × personality)
   └── Compute pace factors (speed multiplier to hit target finish time)
        │
   RaceScript resource (per-drone: pace_factor, crashes, acrobatic gates)
        │
   Real-time Choreographed Playback (FixedUpdate each tick)
   ├── Advance spline_t at curvature-based speed × pace_factor
   ├── Position from spline + acrobatic offset (altitude dip/climb)
   ├── Rotation from curvature (bank) or keyframes (acrobatics)
   ├── Visual noise (attitude jitter, dirty air wobble, micro-drift)
   ├── Events from spline_t thresholds (gate pass, finish, crash)
   └── Cameras, leaderboard, explosions, fireworks — all unchanged
```

### Key Data
- **Race script:** ~1 KB per race (12 DroneScripts with pace factors + crash/acrobatic metadata).
- **Race seed:** Single u32 for deterministic script generation.
- **Highlight candidates:** Scored events (overtakes, acrobatics, crashes, close finishes) for the hype phase UI.
- No trajectory buffer needed — the race plays out in real-time from the script.

### Replay Support (future)
Deterministic from `(RaceScript, CourseData, per-drone splines)`. A replay re-runs the choreography with different camera settings. Store the RaceScript alongside the course save file.

---

## Open Questions

1. **Audience scoring model** — How exactly does the game evaluate "how entertaining was that race"? Needs design (number of overtakes, acrobatic maneuvers, close finishes, DNFs, etc.)
2. ~~**Pilot card collection**~~ — Resolved: "Gotta catch 'em all" collection as progress tracking. Cards are trophies, not strategic resources.
3. **Course rating** — Does the game give feedback on course design before the race (predicted difficulty, spectacle score) or only after?
4. **Hype tool scope** — How much creative control in posters/merch before it becomes a distraction from the core loop?
5. **Difficulty curve** — How does the game teach course design? Tutorial races? Example courses? Feedback systems?
