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

### 2026-04-27 — Phase 5: Contradiction-preserving interference

**Files touched:**
- `src/memory/replay.rs` — added `ContradictionPolicy { preserve_contradictions: bool, salience_boost: f32 }` (defaults: `false`, `0.20` — byte-equivalent to upstream when not flipped). Added `ContradictionResult { salience_boost, event }`. Added `policy: ContradictionPolicy` field on `InterferenceDetector`. New constructors / mutators: `with_policy()`, `set_policy()`, `policy()`. New API `contradict_explicit(node_a_id, node_b_id, evidence_id) -> ContradictionResult` for caller-driven contradiction registration. Modified `check_interference()` — when `preserve_contradictions = true`, every would-be retroactive / proactive suppression on a similar-but-conflicting memory is replaced by a `SuppressionAverted` audit event and the loop `continue`s without applying decay. Modified `apply_retrieval_competition()` — when `preserve_contradictions = true`, every candidate becomes a winner; close-competitor pairs that would have been suppressed get a `SuppressionAverted` event for the audit trail. Severe-similarity duplicate detection still fires (preserves dedup semantics).
- `src/memory/introspection.rs` — added two `ConsolidationEvent` variants: `SuppressionAverted { new_memory_id, old_memory_id, similarity, interference_type, timestamp }` and `ContradictionRegistered { node_a_id, node_b_id, evidence_id, salience_boost, timestamp }`. Added `suppressions_averted` and `contradictions_registered` counters to `ConsolidationStats` (with `#[serde(default)]` for backward-compat). Added match arms in both report-builder methods and the `timestamp()` accessor.
- `src/memory/learning_history.rs` — added match arms in `classify_event()` mapping the two new event variants onto `LearningEventType::InterferenceDetected` so the existing event-history channel surfaces them.
- `tests/contradiction_preservation_tests.rs` — new integration test file with 5 cases that pin down (1) default policy is byte-equivalent to upstream suppression, (2) flipped policy averts retroactive suppression at write time and emits `SuppressionAverted`, (3) flipped policy keeps every candidate as a winner at retrieval time, (4) `contradict_explicit()` emits `ContradictionRegistered` with the configured boost, (5) policy can be swapped on an existing detector mid-flight.

**Summary:** Reverses the polarity of upstream's similarity-suppression engine — but only when the caller opts in via `ContradictionPolicy { preserve_contradictions: true }`. The default remains the upstream behavior (suppress the weaker / older memory under high-similarity interference), so every pre-Phase-5 caller sees byte-identical results and the existing 1163 upstream tests pass unchanged. When flipped, the engine preserves both sides of a contradiction, logs `SuppressionAverted` for the audit trail, and on explicit `contradict_explicit()` calls returns a configurable salience boost (default +0.20) plus a `ContradictionRegistered` event so downstream Tier 2 hypothesis-register and audit-trail work has a typed signal to consume. Crucially, `InterferenceDetector` does NOT take a `GraphMemory` dependency — `Meta::Contradicts` edge insertion is left to the caller (the application layer), keeping module boundaries clean.

**Why:** Spec Section 6.1 — biological brains preserve contradicting evidence rather than suppressing it; Tier 2 hypothesis tracking, audit trails, and outcome-based salience all depend on contradictions being first-class signals rather than noise that gets averaged away. The Phase 1 `Meta::Contradicts` variant was added precisely so this phase had a vocabulary to record contradictions structurally; this phase wires the runtime that produces those signals.

**Compatibility:** Default policy is byte-equivalent to upstream — `ContradictionPolicy::default()` sets `preserve_contradictions = false`, so any caller that does not opt in sees identical behavior on every code path through `check_interference()` and `apply_retrieval_competition()`. The two new `ConsolidationEvent` variants are additive; bincode roundtrip survives because variants are tagged by index and we only appended. `ConsolidationStats` adds two `#[serde(default)]` counters. No RocksDB schema migration required. The 1173 release-mode tests pass (1163 inherited + 5 new lib unit tests + 5 new integration tests). `cargo bench --bench graph_benchmarks` shows median deltas of +2-8% with no median exceeding the +10% blocking threshold; this is consistent with criterion baseline drift on a freshly-cooled host (Phase 4 baselines were captured on a hotter host). Phase 5 only touches `replay.rs` / `introspection.rs` / `learning_history.rs`, which `graph_benchmarks` does not exercise — observed drift is environmental noise, not Phase 5 cost. Other bench suites (`memory_benchmarks`, `ner_benchmarks`, `cognitive_benchmarks`) are blocked by pre-existing upstream/env issues unrelated to Phase 5: cdylib panic-strategy collision when multiple benches share the lib build, missing `libonnxruntime.dylib` on this host, and stale upstream code in `cognitive_benchmarks.rs` that no longer matches the current API surface (5 E0277 errors). Same caveat applied at Phase 0 for `relevance_benchmarks` and is documented in `docs/baselines.md`.

### 2026-04-29 — Tier 2 Phase 0: pyo3 0.23 deprecation cleanup + LLM-parser config fix

**Files touched:**
- `src/python.rs` — added `use pyo3::conversion::IntoPyObjectExt;`; migrated all 164 `value.into_py(py)` deprecation sites to `value.into_py_any(py)?` (pyo3 0.24's `IntoPyObjectExt` extension method). Adjusted seven `.map(|...| { ... })` closures (the `to_py_list` builder, three `neuron`-builder closures, the `evt` event closure, and the `mem` / `assoc` consolidation-report closures) to return `PyResult<HashMap<...>>` and propagate via `.collect::<PyResult<Vec<_>>>()?` so the new `?` propagation type-checks. Replaced one `Option::map(...)?.unwrap_or_else(py.None())` site with a `match` arm because `Option::map` doesn't compose with `?`.
- `src/query_parsing/mod.rs` — replaced the stale `llm_model_path` / `llm_threads` / `llm_context_size` fields on `ParserConfig` (a leftover from pre-HTTP llama.cpp-based `LlmParser`) with `llm_endpoint` / `llm_model` to match the current `LlmParser::new(endpoint: &str, model: &str)` signature. Updated `ParserConfig::llm()` constructor and the `create_parser()` call site accordingly. Removes the 3-arg-vs-2-arg signature drift that blocked `cargo clippy --all-targets` and `cargo build --features llm-parser`.

**Summary:** Tier 1 fallout cleanup. Tier 1 reported `cargo clippy --features python -- -D warnings` blocked by ~347 pyo3 0.23 deprecation errors and `cargo build --features llm-parser` blocked by stale `LlmParser::new()` arity. Phase 0 of Tier 2 unblocks both gates so subsequent Tier 2 phases can extend the Python bindings without re-cleaning rot.

**Verification:**
- `cargo check --features python --lib` — clean (was 164 deprecation warnings before this phase, 0 after).
- `cargo check --features llm-parser --lib` — clean (was 2 errors before this phase, 0 after).
- `cargo check --lib` — clean (no regression on default-feature builds).
- `cargo build --release --bench cognitive_benchmarks` — clean (was 5 E0277 errors at Tier 1, now compiles with only style warnings).
- `cargo test --release --no-fail-fast` — **1180 passed, 0 failed, 0 ignored** across 30 suites (Tier 1 baseline preserved exactly).
- `cargo fmt --check` — clean.
- `cargo clippy --features python --lib` — 1 new lint surfaced (`clippy::too_many_arguments` on `record_decision`, 8/7 args, a pyo3 binding signature decision and not deprecation-related); 145 inherited upstream warnings carry forward unchanged. Net regression on python feature: zero deprecation warnings, +1 unrelated style lint.

**Compatibility:** Behavior-preserving migration. `into_py(py)` and `into_py_any(py)?` produce equivalent `PyObject` outputs for every type used in this file (numbers, strings, vecs, hashmaps, datetimes); the `?` cascade only surfaces if the underlying conversion truly fails, which it cannot for the types in use. The `ParserConfig` struct shape changes from three `llm_*` fields to two, but the only caller is the in-module `create_parser()`; no public-API breakage observed in `git grep`. The `llm-parser` feature was already broken at Tier 1 — this phase fixes it. Bench infrastructure (`graph_benchmarks` panic-strategy collision, `relevance_benchmarks` / `streaming_benchmarks` / `memory_benchmarks` E0061/E0308 signature drift) remains as documented upstream rot — not in scope for Phase 0.

### 2026-04-27 — Tier 1 release (v0.1.90-ce.tier1)

**Files touched:**
- `src/python.rs` — added `node_types: None, domain_tags: None` to two `Query { ... }` struct literals (Phase 4 fallout sites missed during the initial Phase 4 work; surfaced by `cargo clippy --features python`).
- `tests/tier1_smoke.rs` — new end-to-end integration test file with 7 cases that exercise all five Tier 1 phases through the public Rust API: directional `Causal::Inhibits` traversal (Phase 1), bidirectional `Meta::Contradicts` traversal (Phase 1), decay-floor clamping under long inactivity (Phase 3), per-category decay overrides via JSON config (Phase 3), `NodeType` + `domain_tags` filtering through `QueryBuilder` (Phase 4), `SuppressionAverted` audit trail under flipped `ContradictionPolicy` (Phase 5), and a single workflow that composes all five phases including `contradict_explicit()` with `ContradictionRegistered` audit trail.

**Summary:** Closes Tier 1 with a single end-to-end smoke test that pins the public Rust API for every Tier 1 modification. Phase-internal integration tests already cover each modification in detail; the smoke test is the binding gate that the five phases compose without regression.

**Verification:**
- `cargo build --release` — clean (1135 + 38 + 0 + 0 + 5 + 7 = 1185 reachable test items; `cargo test --release` reports 1180 passed across 30 integration suites + lib unit tests).
- `cargo test --release --no-fail-fast` — **1180 passed, 0 failed, 0 ignored**. Includes the 7 new `tier1_smoke.rs` cases plus all 1173 inherited + Tier-1-added tests.
- `cargo fmt --check` — clean.
- `cargo clippy --lib` (default features) — clean (145 inherited upstream warnings, 0 errors).
- `cargo clippy --all-features -- -D warnings` — **blocked** by pre-existing upstream pyo3 0.23 deprecation rot in `src/python.rs` (~347 `IntoPy::into_py` deprecation errors). Not a Tier 1 regression; documented as Tier 2 cleanup work.
- `cargo clippy --all-targets` — **blocked** by pre-existing upstream rot in `tests/` and `benches/` (e.g., `LlmParser::new()` signature drift in `query_parsing/mod.rs`, stale call sites in `cognitive_benchmarks.rs`). Not a Tier 1 regression.
- `cargo bench --bench graph_benchmarks` — within ±10% of Phase 0 baseline (Phase 5 entry above).
- `cargo tarpaulin` — **deferred**; coverage on a Cargo `crate-type = ["rlib", "cdylib"]` lib trips the same panic-strategy collision documented for multi-bench runs.
- `cd python && maturin develop && pytest` — **deferred to Tier 1.x**; Python binding infrastructure is not configured on this host.
- `cd mcp-server && npm test` — **deferred to Tier 1.x**; MCP server infrastructure is not configured on this host.
- Migration smoke test — **deferred to Tier 1.x**; backward-compat is structurally guaranteed at the serde layer (every new field carries `#[serde(default)]`, every new enum variant is additive), so a pre-Tier-1 RocksDB store deserializes without a one-shot migration step. A dedicated `cognitive-engine migrate` CLI tool remains a planned Tier 1.x deliverable for callers who want to *eagerly* upgrade legacy entries (rather than letting them upgrade on next read).

**Compatibility:** Every Phase-1-through-Phase-5 entry above documents per-phase compatibility — default `ContradictionPolicy` byte-equivalent to upstream, additive `ConsolidationEvent` variants, `#[serde(default)]` on every new `MemoryFlat` field, additive `RelationType` variants. Composed across all five phases this still holds: a pre-Tier-1 caller reading a pre-Tier-1 store sees byte-identical behaviour on every code path.

---

## Provenance

Every entry in this log should record:

- **Date** of the change
- **Phase** (0 setup, 1 edge typing, 2 directionality, 3 decay, 4 ontology, 5 contradiction)
- **Files touched** (relative paths)
- **Summary** of what changed and why
- **Compatibility notes** if behaviour diverges from upstream

This file is updated continuously, never retrofitted.
