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

### 2026-04-26 — Phase 3: Per-type decay with floors

**Files touched:**
- `src/decay.rs` — added `DecayParams` struct (`crossover_days`, `lambda`, `beta`, `floor`), `DecayConfig` struct (`default` + `per_edge_category: HashMap<EdgeCategory, DecayParams>`), `decay_factor_with_params()` function that applies the configured floor as a clamp, JSON `from_json_str`/`from_file` loaders, plus 7 unit tests covering default-matches-upstream, floor-clamping under long inactivity, fresh-memory-not-lifted, lookup fallback, per-category divergence, JSON roundtrip, and JSON partial override.
- `src/graph_memory.rs` — added `Hash` derive to `EdgeCategory` so it can be used as a `HashMap` key.

**Summary:** Adds an opt-in API for per-edge-category decay parameters with structural-importance floors. The floor mechanic is the load-bearing addition: once a node/edge has been classified as structurally important (e.g. a Causal chokepoint), its decayed score cannot drop below the configured floor regardless of access pattern. The default `DecayConfig` is byte-equivalent to upstream's `hybrid_decay_factor` curve (no floor, λ=0.693, β=0.5, crossover=3 days), so callers that don't opt in see no behaviour change. Config loads via JSON; `serde_json` was already a dependency, so no new crate needed.

**Why:** Spec Section 3.3 mod #3 calls for per-type decay so that — for example — Causal evidence can decay slower than transient Co-Retrieved Hebbian links, and Structural backbone nodes never disappear. Floors exist because biological brains protect chokepoints structurally; the cognitive engine needs the same so that Tier 2 audit trail and hypothesis register cannot lose their anchor edges.

**Compatibility:** Pure addition. Upstream `hybrid_decay_factor` and `tier_decay_factor` are untouched; new entry points are opt-in. No RocksDB schema change. JSON config files are optional — when absent, behaviour is identical to upstream.

### 2026-04-27 — Phase 4: Ontology tags

**Files touched:**
- `src/memory/types.rs` — added `NodeType` enum (eight base variants `Entity` / `Event` / `Concept` / `Pattern` / `Heuristic` / `Signal` / `Forecast` / `Context` plus `Legacy` migration default), `default_for_experience_type()` mapping for the migration tool, `is_legacy()` predicate. Added `node_type: NodeType` and `domain_tags: Vec<String>` fields to `Memory`. Updated `MemoryFlat` (the bincode wire format) with `#[serde(default)]` on the new fields so pre-Phase-4 stores deserialize cleanly. Updated `impl Serialize`, `impl Deserialize`, `impl Clone`, `Memory::new`, and `Memory::from_legacy` to thread the new fields. Added `with_ontology(self, NodeType, Vec<String>) -> Self` builder. Added `node_types: Option<Vec<NodeType>>` and `domain_tags: Option<Vec<String>>` filter fields to `Query` plus matching filter logic in `Query::matches()`. Added `QueryBuilder::node_types()` and `QueryBuilder::domain_tags()` helpers. Added 8 unit tests covering the new types, defaults, serde roundtrip, and query filtering.
- `src/memory/mod.rs` — propagated `node_types` and `domain_tags` through the inline `Query { ... }` construction in `vector_query` so semantic-search hits respect the ontology filter.
- `tests/ontology_tags_tests.rs` — new integration test file with 3 cases that exercise the public API (`Memory::with_ontology`, `Query::matches`, `QueryBuilder` chaining, `default_for_experience_type` totality).

**Summary:** Adds an ontology layer on top of upstream's `ExperienceType`. `ExperienceType` describes how a memory was *captured* (Conversation, Decision, CodeEdit, …); `NodeType` describes what it *is* ontologically (an Entity, an Event, a Concept, …). The two are orthogonal — a `Decision` experience usually maps to a `Concept` node, but the application layer is free to override. `domain_tags` is a free-form `Vec<String>` so a single node can belong to multiple application domains (`["financial", "macro"]`, `["operations", "fleet"]`, …) without bloating the type system. Filters on `Query` are AND-composed across fields and OR-composed within `domain_tags` (any match), matching the existing Query semantics.

**Why:** Spec Section 5.1 calls out the eight base node types as the substrate Tier 2 needs for hypothesis tracking, audit trails, and pattern extraction. Without typed nodes, downstream code can't tell a `Concept` ("the Phillips curve flattens at high inflation") from a `Signal` ("CPI print 0.4% MoM") — they're both `ExperienceType::Learning`. Domain tags exist so the same engine can serve multiple application brains (Financial / CEO / Research / Operations) without each brain needing its own database.

**Compatibility:** Pre-Phase-4 RocksDB stores deserialize because `MemoryFlat`'s new fields carry `#[serde(default)]` — old payloads decode with `node_type = Legacy` and `domain_tags = []`. Existing call sites that don't opt in are byte-equivalent on the read path. Bincode roundtrip is covered by the lib unit test `memory_serde_roundtrip_preserves_ontology`. The integration test deliberately *omits* a bincode roundtrip case because including one triggers a Cargo dep-resolution edge case caused by upstream's `crate-type = ["rlib", "cdylib"]` setting on the lib target — coverage is preserved at the lib-internal layer, where the rlib/cdylib distinction doesn't apply.

---

## Provenance

Every entry in this log should record:

- **Date** of the change
- **Phase** (0 setup, 1 edge typing, 2 directionality, 3 decay, 4 ontology, 5 contradiction)
- **Files touched** (relative paths)
- **Summary** of what changed and why
- **Compatibility notes** if behaviour diverges from upstream

This file is updated continuously, never retrofitted.
