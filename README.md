# Termhold

A simple CLI colony management game written in Rust.

Game can work with different UI systems. Now it includes only terminal UI.

Manage resources, expand your settlement and survive as long as possible.

---

# Implemented Mechanics

## Resources

- **Wood** — primary construction resource.
- **Stone** — primary construction resource.
- **Food** — required for survival and population growth.
- **Population** — provides workforce and determines colony expansion.

Population growth depends on available food and a random birth chance.
Building additional **Huts** increases the maximum population.

## Buildings

- [x] **Hut** — increases maximum population
- [x] **Lumber Yard** — passive wood production
- [x] **Stone Quarry** — passive stone production
- [x] **Farm** — passive food production
- [x] **Barn** — increases food storage capacity

---

# Mechanics Under Review

- [ ] Starvation (currently settlers die instantly one by one)

---

# Roadmap

## Gameplay

### Core Gameplay

- [x] Population growth
- [x] Worker management
- [x] Active gathering
  - [x] Wood
  - [x] Stone
  - [x] Food
- [x] Passive production

### Next Features

- [ ] Random events
- [ ] Endgame conditions
- [ ] Improved starvation mechanics
- [ ] Unique villagers
- [ ] Research system
- [ ] Happiness
- [ ] Combat
- [ ] Storyline

### World Simulation

- [ ] Seasons
- [ ] Weather
- [ ] Difficulty presets
- [ ] World map

### Economy

- [ ] Trading system
- [ ] Advanced resources
  - [ ] Stone tools
  - [ ] Processed food
  - [ ] Production chains
- [ ] Dynamic market prices

---

# Architecture Roadmap

## Core Data Model

- [ ] Replace `WorkerSite` with a shared `BuildingKind`
- [x] Introduce `ResourceKind`
- [ ] Replace building counters with `Building`
- [ ] Replace `Vec<String>` with structured `LogEntry`
- [ ] Expand `World`
  - [ ] Seasons
  - [ ] Weather
  - [ ] Difficulty
  - [ ] Game speed

## Configuration

- [x] Split `Balance` into nested structures

```text
Balance
├── GatherBalance
├── PopulationBalance
├── BuildingBalance
└── StorageBalance
```

## Commands & Actions

- [x] Replace specialized commands

```rust
BuildFarm
BuildBarn
BuildQuarry
...
```

with generic actions

```rust
Actions::Build(BuildingKind)
Actions::Demolish(BuildingKind)
Actions::Gather(ResourceKind)
```

## Game Systems

Split game logic into independent systems.

- [ ] Production
- [ ] Population
- [ ] Workers
- [ ] Economy
- [ ] Storage
- [ ] Events

### Target architecture

```text
Game::tick()
    ↓
Production
    ↓
Population
    ↓
Storage
    ↓
Events
    ↓
World
```

## Engine

- [ ] Keep the game logic completely UI-independent
- [ ] Support multiple frontends
  - CLI
  - Ratatui
  - GUI (future)
- [ ] Replace direct UI interaction with `GameEvent`
- [ ] Separate input handling from game simulation

---

# Development Principles

- Keep game logic independent from the user interface.
- Prefer data-driven balancing through `Balance`.
- Avoid God Objects by splitting logic into independent systems.
- Prefer generic enums (`BuildingKind`, `ResourceKind`) over duplicated types.
- Keep simulation deterministic where possible.
- Separate **input → simulation → rendering**.

---

# Design Philosophy

**Termhold** focuses on **strategic colony management**, not individual settler simulation.

If a world map is introduced, it will primarily provide **spatial strategy** rather than micromanagement.

The map is intended to answer questions such as:

- Where should buildings be placed?
- Which terrain is best for production?
- How should the colony expand?

rather than simulate every settler's movement or daily routine.

**Termhold** focuses on strategic colony management with narrative-driven progression.

The colony evolves not only through economic decisions, but also through story events, difficult choices and long-term consequences.

# Long-Term Ideas

- [ ] AI-controlled colonies
- [ ] Diplomacy
- [ ] Multiple races
- [ ] Procedural world generation
- [ ] Save / Load system
- [ ] Mod support
- [ ] Ratatui interface
- [ ] Graphical (2D/3D) client
- [ ] Audio
- [ ] Multiplayer
