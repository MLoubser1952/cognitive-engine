# Tier 1 Architecture Spike

**Status:** Draft (Phase 0.5)
**Date:** 2026-04-26
**Baseline:** Shodh-memory v0.1.90 (pinned)

This document maps each Tier 1 modification from the project blueprint
to the actual upstream code at v0.1.90. **Several blueprint assumptions
turned out to be stale** — they were written against an earlier Shodh
state. Findings below.

---

## Major finding — Tier 1 scope shrinks

The blueprint identified ten gaps in Shodh. After reading v0.1.90 source,
**at least three gaps are partially or wholly obsolete**:

### GAP 1 (no edge typing) — PARTIALLY OBSOLETE

The blueprint claims "all Shodh graph edges are undifferentiated 'related to'
connections." This is **not true at v0.1.90**.

`src/graph_memory.rs:285` defines `RelationshipEdge` with:
- `from_entity: Uuid` and `to_entity: Uuid` — explicit source/target ordering
- `relation_type: RelationType` — typed enum
- `strength`, `created_at`, `valid_at`, `invalidated_at` — full lifecycle

`RelationType` (`src/graph_memory.rs:850`) already includes:
- Work: `WorksWith`, `WorksAt`, `EmployedBy`
- Structural: `PartOf`, `Contains`, `OwnedBy`
- Location: `LocatedIn`, `LocatedAt`
- Usage: `Uses`, `CreatedBy`, `DevelopedBy`
- **Causal:** `Causes`, `ResultsIn`
- Learning: `Learned`, `Knows`, `Teaches`
- Generic: `RelatedTo`, `AssociatedWith`
- Hebbian: `CoRetrieved`

**What's still missing for the project's ontology:**
- The `Meta` category — no `Contradicts`, `Supports`, `IsInstanceOf`, `GeneralisesTo`
- `Causal::Inhibits`, `Causal::Enables`, `Causal::Amplifies` (only `Causes` and `ResultsIn` exist)
- `Temporal` category — no `Preceded`, `Triggered`, `CoincidedWith`
- A grouping layer — types are flat, not grouped under Structural/Causal/Temporal/Meta supercategories

**Implication for Tier 1:**
Mod #1 (edge typing) reduces to: **add the missing Meta and Temporal categories**, **add the missing Causal sub-tags**, and **decide whether to keep the flat enum or refactor into a grouped enum**. The fundamental capability is already there — this is now a vocabulary extension, not a structural rewrite.

### GAP 1 (also covers directionality) — OBSOLETE

Blueprint claims edges are undirected. **Not true** — `from_entity`/`to_entity`
ordering is explicit at v0.1.90. Mod #2 (edge directionality) is **already done**.

The remaining work for #2 is just policy: should certain `RelationType`s
be treated as semantically bidirectional even though they're physically
directional? (E.g. "competes with" — physically stored as A→B, but
queries from B should still surface A.) This is a query-layer concern,
not a schema change.

### GAP 9 (limited decay customisation) — STILL VALID

`src/decay.rs:74` `hybrid_decay_factor()` takes only `(days_elapsed, potentiated)`.
No per-type decay parameters, no decay floors. Mod #3 is genuine work.

Constants in `src/constants.rs`:
- `DECAY_CROSSOVER_DAYS`, `DECAY_LAMBDA_CONSOLIDATION`, `POWERLAW_BETA`, `POWERLAW_BETA_POTENTIATED`

These are global. Mod #3 must extend the decay function to look up
per-type params, with the existing globals as defaults.

### GAP 6 (no episodic/semantic/procedural separation at memory-type level) — PARTIALLY OBSOLETE

Shodh already has multiple memory types — see `src/memory/types.rs` and
related files (`facts.rs`, `temporal_facts.rs`, `learning_history.rs`,
`replay.rs`, `prospective.rs`). The blueprint's tag-only critique was
correct for an earlier version; v0.1.90 has structural separation already.

Mod #4 (ontology tags) is still useful for **adding the eight base node
types from the blueprint's Section 5.1** as a unifying layer over the
existing memory types, plus multi-domain tag arrays. But the underlying
storage separation it would have implied is already there.

### GAP 2 (no contradiction handling) — STILL VALID

Confirmed via grep: interference logic exists in:
- `src/constants.rs:1528+` — `INTERFERENCE_SIMILARITY_THRESHOLD = 0.85`, `INTERFERENCE_SEVERE_THRESHOLD = 0.95`, plus strength-reduction constants
- `src/memory/storage.rs`, `src/memory/retrieval.rs`, `src/memory/replay.rs`, `src/memory/mod.rs`, `src/memory/learning_history.rs`, `src/memory/introspection.rs`, `src/relevance.rs` — all reference interference

The upstream model: when similarity > 0.85, similar memories compete; when > 0.95, one dominates and the other is weakened. This is exactly the
suppression-instead-of-preservation behaviour the blueprint identifies as
the cognitive bias problem. Mod #5 is real Tier 1 work.

---

## Revised Tier 1 work inventory

| # | Original mod | Revised scope at v0.1.90 |
|---|---|---|
| 1 | Edge typing | Extend `RelationType` enum with missing Meta/Temporal categories and missing Causal sub-tags. Decide flat vs. grouped enum. |
| 2 | Edge directionality | **Already done.** Add bidirectional-query policy at query layer for symmetric semantics like `WorksWith`. |
| 3 | Per-type decay with floors | Full work — decay function and constants are global today. |
| 4 | Ontology tags | Add eight base `NodeType` enum + multi-domain `tag_array: Vec<String>` field over existing entity types. |
| 5 | Contradiction-preserving interference | Full work — flip suppression to preservation across `src/memory/*` and `src/relevance.rs`. |

Phase 0 finding: **Tier 1 is roughly half the size the blueprint estimated.**
The vocabulary-expansion work of #1 plus #2-as-policy is days, not weeks.
The decay refactor (#3) and contradiction flip (#5) remain the main work.

---

## File-level impact map

### `src/graph_memory.rs` (6495 lines)
- Phase 1: extend `RelationType` enum (lines ~850 onward).
- Phase 2: bidirectional-query helper functions; no schema change.

### `src/decay.rs` (366 lines)
- Phase 3: extend `hybrid_decay_factor()` signature to accept per-type params + floor; load from a config struct.

### `src/constants.rs`
- Phase 3: keep current globals as defaults; new per-type overrides live in config files, not constants.

### `src/memory/types.rs` and friends
- Phase 4: add `NodeType` enum and `domain_tags: Vec<String>`. Bridge old types to new node types.

### `src/memory/storage.rs`, `src/memory/retrieval.rs`, `src/memory/replay.rs`, `src/memory/learning_history.rs`, `src/memory/introspection.rs`, `src/relevance.rs`
- Phase 5: every site that suppresses similar memories must be replaced with preservation + contradiction-edge insertion + salience boost on the new evidence.

### `src/lib.rs`
- Module re-exports for any new public types added in Phases 1, 3, 4.

### `python/`
- Phase 1, 3, 4, 5: PyO3 bindings updates for new public API surface.

### `mcp-server/`
- Phase 1, 5: new MCP tool definitions (`contradict`, edge-type-filter on `traverse`, etc.).

---

## Open questions for the team

1. **Flat vs grouped enum for `RelationType`?** Keep upstream's flat enum (preserves API compat) or refactor into `EdgeType::Causal(CausalType)` etc. (cleaner, breaks API)?
2. **Default `EdgeType` for Hebbian co-activation?** Upstream uses `RelationType::CoRetrieved` — keep this, or rename to fit the new ontology?
3. **Bidirectional semantics:** which `RelationType`s should default to bidirectional queries? Proposal: `WorksWith`, `RelatedTo`, `AssociatedWith` (symmetric); rest stay strictly directional.
4. **Decay config format:** TOML, JSON, or YAML? Upstream uses JSON for `shodh_config.example.json`.
5. **Contradiction detection mechanism in Tier 1:** explicit-only via a `contradict()` API call (proposed), or also a heuristic where similar-but-disagreeing memories auto-contradict? Heuristic agreement check probably needs LLM, which is Tier 3.

---

## Next steps after this spike

1. Capture baseline build/test/bench numbers in `docs/baselines.md`.
2. Resolve open questions above with the team.
3. Open issues in the fork's GitHub for each Tier 1 mod with this spike as context.
4. Begin Phase 1 (edge typing vocabulary extension).
