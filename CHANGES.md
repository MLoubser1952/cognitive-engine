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

### 2026-04-26 — Phase 1: Edge-typing vocabulary extension

**Files touched:**
- `src/graph_memory.rs` — extended `RelationType` enum, added `EdgeCategory`, added `category()` / `is_bidirectional()` / `opposite()` helpers, added `outgoing_edges` / `incoming_edges` / `outgoing_with_bidirectional` query helpers, added 5 unit tests.
- `src/mif/export.rs` — extended exhaustive `relation_type_to_string()` match for the 10 new variants.
- `src/mif/import.rs` — added explicit string parses for the 10 new variants (also fall through to `Custom` when unrecognised, preserving backward compat).
- `src/handlers/mif.rs` — added explicit MCP-string-to-variant parses for the 10 new variants (handles both en-GB and en-US `Generalises/Generalizes`).
- `tests/graph_memory_tests.rs` — added 4 integration tests covering directed and bidirectional traversal.

**Summary:** The Phase 0 spike showed that upstream `RelationshipEdge` already has explicit `from_entity`/`to_entity` ordering and a typed `RelationType` enum with 21 variants. Phase 1 reduces to a vocabulary extension rather than the structural rewrite that the original Tier 1 plan envisaged. Added 10 new variants — `Inhibits`, `Enables`, `Amplifies` (Causal); `Contradicts`, `Supports`, `IsInstanceOf`, `GeneralisesTo` (Meta); `Preceded`, `Triggered`, `CoincidedWith` (Temporal). The flat-vs-grouped enum decision was resolved in favour of FLAT to keep the public API (Python HTTP, MCP server, RocksDB serde, all existing tests) stable; grouping is provided as metadata via `RelationType::category() -> EdgeCategory`.

**Why:** Phase 5 (contradiction preservation) depends on `Meta::Contradicts` edges existing in the vocabulary; Tier 2 hypothesis tracking depends on `Meta::Supports`; the broader Causal sub-tags (Inhibits/Enables/Amplifies) are needed to express financial / operations causal models in the application layers without falling back to `Custom(String)`.

**Compatibility:** No RocksDB schema migration required — serde-encoded `RelationType` round-trips because we only added variants. Existing stores keep working. Phase 2 (directionality) was absorbed into this PR because the spike confirmed it was already done at v0.1.90; the only remaining directionality work was the `outgoing_with_bidirectional()` helper, which now lives here.

---

## Provenance

Every entry in this log should record:

- **Date** of the change
- **Phase** (0 setup, 1 edge typing, 2 directionality, 3 decay, 4 ontology, 5 contradiction)
- **Files touched** (relative paths)
- **Summary** of what changed and why
- **Compatibility notes** if behaviour diverges from upstream

This file is updated continuously, never retrofitted.
