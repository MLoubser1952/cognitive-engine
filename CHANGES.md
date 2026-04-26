# Cognitive Engine — Changes from upstream Shodh-memory

This file records significant changes made to the upstream Shodh-memory
codebase, as required by Apache License 2.0 §4(b).

**Upstream:** https://github.com/varun29ankuS/shodh-memory
**Pinned baseline:** tag `v0.1.90` (April 2026)
**Derivative branch base:** `cognitive-engine-tier1` (forked from `v0.1.90`)

---

## Tier 1 — Rust core modifications

The Tier 1 work plan defines five modifications to the upstream Rust core:

1. **Edge typing** — typed graph edges (Structural / Causal / Temporal / Meta with sub-tags)
2. **Edge directionality** — directed edges with an explicit source/target ordering
3. **Per-type decay with floors** — configurable decay parameters per node and edge type, with structural-importance floors
4. **Ontology tags** — eight base node types (Entity / Event / Concept / Pattern / Heuristic / Signal / Forecast / Context) plus multi-domain tag arrays
5. **Contradiction-preserving interference** — flips upstream's similarity-suppression engine into a contradiction-preservation engine that boosts conflicting evidence and inserts `Meta::Contradicts` edges

See [docs/design/tier1-architecture.md](docs/design/tier1-architecture.md)
for the architecture spike that maps each modification to the relevant
upstream files.

---

## Change log

### 2026-04-26 — Phase 0 setup

- Forked `varun29ankuS/shodh-memory` to `MLoubser1952/cognitive-engine`.
- Pinned to upstream tag `v0.1.90`.
- Created branch `cognitive-engine-tier1` for Tier 1 work.
- Added `NOTICE` (Apache 2.0 §4(d) attribution).
- Added this `CHANGES.md` (Apache 2.0 §4(b) modification record).

No source code modifications yet.

---

## Provenance

Every entry in this log should record:

- **Date** of the change
- **Phase** (0 setup, 1 edge typing, 2 directionality, 3 decay, 4 ontology, 5 contradiction)
- **Files touched** (relative paths)
- **Summary** of what changed and why
- **Compatibility notes** if behaviour diverges from upstream

This file is updated continuously, never retrofitted.
