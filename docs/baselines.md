# Phase 0 Baselines — Cognitive Engine

Captures the upstream Shodh-memory v0.1.90 baseline performance and test
state on the team's primary hardware. These numbers are the **regression
budget** for all Tier 1 work — no Tier 1 phase may regress any baseline
metric by more than 10%.

---

## Capture environment

- **Date:** 2026-04-26
- **Hardware:** Apple Silicon (aarch64-apple-darwin), macOS 24.6.0
- **Rust toolchain:** 1.95.0 (stable)
- **Build profile:** `release` (`cargo build --release`)
- **Upstream commit:** v0.1.90 → `1de172e` (`docs: update TUI screenshots — recall view and projects/todos view`)

## Build baseline

- `cargo build --release`: **4 min 13 sec** clean build (cold cache, after `cargo clean`)
- One-time setup blocker resolved: librocksdb-sys requires libclang at runtime; on macOS we symlinked `/Library/Developer/CommandLineTools/usr/lib/libclang.dylib` into `~/.rustup/toolchains/stable-aarch64-apple-darwin/lib/`.
- Models + ONNX Runtime (~70MB) auto-cache at `~/Library/Caches/shodh-memory/` on macOS (via `dirs::cache_dir()`); pre-populated for tests via direct download from HuggingFace (MiniLM model_quint8_avx2.onnx, TinyBERT-NER model_quantized.onnx) and GitHub (onnxruntime-osx-arm64-1.23.2.tgz).

## Test baseline

`cargo test --release --no-fail-fast`:

| Metric | Upstream baseline |
|---|---|
| Total test cases | **1135** (across 24 integration test binaries + lib unit tests + doc tests) |
| Passed | 1135 |
| Failed | 0 |
| Ignored | 7 (doc tests gated on examples) |
| Wall time | ~6 min on Apple Silicon (slowest suites: brutal_stress 105s, cognitive_stress 121s, timing_sla 49s) |

## Benchmark baseline

`cargo bench --bench graph_benchmarks --bench memory_benchmarks` (criterion).

`relevance_benchmarks` excluded — broken at upstream v0.1.90 (six call sites pass 4 args to `surface_relevant`, which takes 5; tracked as upstream issue, will be fixed when we touch relevance during Phase 5).

Spec-quoted upstream targets (from project blueprint Section 3.1) versus measured numbers on this hardware:

| Bench | Spec target | Measured (this hardware) | Tier 1 budget (= measured × 1.10) |
|---|---|---|---|
| graph_entity_get / 100-entity store | 763 ns | **622 ns** | 684 ns |
| graph_entity_get / 1000-entity store | — | 699 ns | 769 ns |
| graph_traversal / 3-hop | 30 µs | **141 µs** | 155 µs |
| graph_traversal / 1-hop | — | 60 µs | 66 µs |
| graph_traversal / 2-hop | — | 107 µs | 117 µs |
| graph_hebbian_traversal / 200e_400r_d3 | — | 58 µs | 64 µs |
| vector_search / 25-result top-k | 34–58 ms | **2.31 ms** | 2.54 ms |
| retrieve_memories / 10 | — | 2.38 ms | 2.62 ms |
| record_experience / 100-memory store | 55–60 ms | **2.18 µs** | 2.40 µs |
| end_to_end_ner_record_retrieve | — | 2.38 ms | 2.62 ms |
| embedding_generation / 50 words | — | 24.00 ms | 26.40 ms |
| ner_only | — | 573 ns | 631 ns |

### Variance from spec — read carefully

- **Entity lookup (763 ns → 622 ns):** Apple Silicon outperforms the spec target by ~18%. Use 622 ns as the regression anchor, not 763 ns.
- **3-hop traversal (30 µs → 141 µs):** ~4.7× slower than the spec figure. The spec number likely came from a smaller fixture or non-Hebbian path; the Hebbian variant lands at 58 µs which is closer. Treat 141 µs as authoritative for this hardware. Investigate whether Phase 1 (edge typing) lookups can be made cheaper as part of Tier 1 stretch goals.
- **Semantic search (34–58 ms → 2.31 ms):** ~15–25× faster than the spec figure. The spec range probably included the embedding-generation hop (~24 ms) plus search; isolated `vector_search` runs at 2.31 ms. End-to-end `end_to_end_ner_record_retrieve` lands at 2.38 ms.
- **Memory storage (55–60 ms → 2.18 µs):** Spec figure must have included full pipeline (embedding generation + RocksDB write); core `record_experience` is microseconds. The end-to-end number is captured by `end_to_end_ner_record_retrieve` (2.38 ms).

### Full bench detail

```
graph_entity_get
  10           539.75 ns
  100          622.20 ns
  1000         699.04 ns
graph_traversal
  1             59.85 µs
  2            106.50 µs
  3            140.98 µs
graph_hebbian_traversal
  100e_200r_d2  42.24 µs
  200e_400r_d3  58.48 µs
  50e_100r_d2   54.76 µs
vector_search
  5              2.40 ms
  10             2.31 ms
  25             2.31 ms
  50             2.34 ms
record_experience
  10             1.78 µs
  50             1.93 µs
  100            2.18 µs
  500            3.03 µs
retrieve_memories
  1              2.36 ms
  5              2.30 ms
  10             2.38 ms
  25             2.54 ms
embedding_generation
  10_words       23.94 ms
  50_words       24.00 ms
  100_words      24.06 ms
ner_record_combined
  ner_only      573.32 ns
  ner_experience_creation   1.17 µs
  record_no_ner             2.61 µs
  ner_record_full           3.02 µs
end_to_end_ner_record_retrieve   2.38 ms
```

Raw Criterion estimates persist in `target/criterion/<bench>/<size>/new/estimates.json` and are gitignored. Re-run any single bench with `cargo bench --bench <name>` to refresh.

## Regression policy

- Every Tier 1 PR must run `cargo bench` and compare against this file.
- A regression > 10% on any single metric blocks merge.
- A < 10% regression that recurs across phases (death by a thousand cuts) requires a budget exception decision logged in [04 Decisions.md](../../3.%20Active%20Projects/Cognitive%20Engine/04%20Decisions.md).
- Spec figures for entity-lookup and 3-hop traversal are retained only as ballpark sanity checks; the **measured numbers above** are the binding regression budget.
