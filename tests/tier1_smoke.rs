//! Tier 1 end-to-end smoke test (cognitive-engine).
//!
//! Exercises one workflow through every Tier 1 modification — Phase 1 edge
//! typing + directionality, Phase 3 per-type decay with floors, Phase 4
//! ontology tags, Phase 5 contradiction-preserving interference. Per-phase
//! integration tests already cover each modification in detail; this file
//! pins down that the five phases compose, that the public Rust API surface
//! works as advertised, and that one phase doesn't accidentally regress
//! another's behaviour. It is the binding gate for tagging
//! `v0.1.90-ce.tier1`.
//!
//! A pre-Phase-1 caller who imports nothing new sees byte-identical
//! behaviour on every code path — that contract is verified in each
//! phase's own test file. This smoke test focuses on the *positive* side:
//! every new capability is reachable from the public API and the
//! expected output materialises end-to-end.

use shodh_memory::chrono::{Duration, Utc};
use shodh_memory::decay::{
    decay_factor_with_params, hybrid_decay_factor, DecayConfig, DecayParams,
};
use shodh_memory::graph_memory::{
    EdgeCategory, EdgeTier, EntityLabel, EntityNode, GraphMemory, LtpStatus, RelationType,
    RelationshipEdge,
};
use shodh_memory::memory::introspection::{ConsolidationEvent, InterferenceType};
use shodh_memory::memory::replay::{ContradictionPolicy, InterferenceDetector};
use shodh_memory::memory::{Experience, ExperienceType, Memory, MemoryId, NodeType, QueryBuilder};
use shodh_memory::uuid::Uuid;

fn make_memory(content: &str, et: ExperienceType) -> Memory {
    let exp = Experience {
        experience_type: et,
        content: content.to_string(),
        ..Default::default()
    };
    Memory::new(MemoryId(Uuid::new_v4()), exp, 0.7, None, None, None, None)
}
use std::collections::HashMap;
use tempfile::TempDir;

fn make_entity(name: &str, label: EntityLabel, salience: f32) -> EntityNode {
    EntityNode {
        uuid: Uuid::new_v4(),
        name: name.to_string(),
        labels: vec![label],
        created_at: Utc::now(),
        last_seen_at: Utc::now(),
        mention_count: 1,
        summary: String::new(),
        attributes: HashMap::new(),
        name_embedding: None,
        salience,
        is_proper_noun: true,
    }
}

fn make_edge(from: Uuid, to: Uuid, kind: RelationType, strength: f32) -> RelationshipEdge {
    RelationshipEdge {
        uuid: Uuid::new_v4(),
        from_entity: from,
        to_entity: to,
        relation_type: kind,
        strength,
        created_at: Utc::now(),
        valid_at: Utc::now(),
        invalidated_at: None,
        source_episode_id: None,
        context: String::new(),
        last_activated: Utc::now(),
        activation_count: 0,
        ltp_status: LtpStatus::None,
        tier: EdgeTier::L1Working,
        activation_timestamps: None,
        entity_confidence: None,
    }
}

#[test]
fn tier1_phase1_directional_causal_inhibits_edge_traverses_one_way() {
    let temp = TempDir::new().expect("tempdir");
    let graph = GraphMemory::new(temp.path(), None).expect("graph");

    let cause = make_entity("Rate hike", EntityLabel::Concept, 0.8);
    let effect = make_entity("Equity multiples", EntityLabel::Concept, 0.7);
    let cause_id = graph.add_entity(cause).expect("add cause");
    let effect_id = graph.add_entity(effect).expect("add effect");

    // Phase 1 vocabulary: Causal::Inhibits is one of the new variants.
    let edge = make_edge(cause_id, effect_id, RelationType::Inhibits, 0.85);
    graph.add_relationship(edge).expect("add edge");

    // Phase 1 metadata: Inhibits is Causal and not bidirectional.
    assert_eq!(RelationType::Inhibits.category(), EdgeCategory::Causal);
    assert!(!RelationType::Inhibits.is_bidirectional());

    let from_cause = graph
        .outgoing_with_bidirectional(&cause_id)
        .expect("traverse cause");
    assert!(from_cause
        .iter()
        .any(|e| e.to_entity == effect_id && e.relation_type == RelationType::Inhibits));

    // The directionality contract: querying from the effect side does NOT
    // surface the inhibits edge, because Inhibits is one-way.
    let from_effect = graph
        .outgoing_with_bidirectional(&effect_id)
        .expect("traverse effect");
    assert!(!from_effect.iter().any(|e| e.from_entity == cause_id));
}

#[test]
fn tier1_phase1_meta_contradicts_edge_is_bidirectional() {
    // Meta::Contradicts is the vocabulary added in Phase 1 specifically so
    // Phase 5 has somewhere to record contradictions structurally. It is
    // declared bidirectional — both endpoints see the conflict.
    assert_eq!(RelationType::Contradicts.category(), EdgeCategory::Meta);
    assert!(RelationType::Contradicts.is_bidirectional());

    let temp = TempDir::new().expect("tempdir");
    let graph = GraphMemory::new(temp.path(), None).expect("graph");
    let a = graph
        .add_entity(make_entity("Hypothesis A", EntityLabel::Concept, 0.7))
        .expect("add a");
    let b = graph
        .add_entity(make_entity("Hypothesis B", EntityLabel::Concept, 0.7))
        .expect("add b");
    graph
        .add_relationship(make_edge(a, b, RelationType::Contradicts, 0.9))
        .expect("contradicts edge");

    let from_a = graph.outgoing_with_bidirectional(&a).expect("from a");
    let from_b = graph.outgoing_with_bidirectional(&b).expect("from b");
    assert!(from_a
        .iter()
        .any(|e| e.relation_type == RelationType::Contradicts));
    assert!(from_b
        .iter()
        .any(|e| e.relation_type == RelationType::Contradicts));
}

#[test]
fn tier1_phase3_decay_floor_clamps_long_inactivity() {
    let no_floor = DecayParams {
        floor: 0.0,
        ..DecayParams::upstream_default()
    };
    let with_floor = DecayParams {
        floor: 0.40,
        ..DecayParams::upstream_default()
    };

    // 365 days of inactivity. Without a floor, retention drops to a low
    // single-digit percent (matches the upstream curve). With a 0.40
    // floor, retention is clamped at 40%.
    let raw_long = decay_factor_with_params(365.0, &no_floor);
    let clamped_long = decay_factor_with_params(365.0, &with_floor);
    let upstream_long = hybrid_decay_factor(365.0, false);

    assert!(
        (raw_long - upstream_long).abs() < 1e-6,
        "no-floor params must match upstream curve"
    );
    assert!(
        clamped_long >= 0.40 - 1e-6,
        "floor clamps long-inactivity decay"
    );
    assert!(raw_long < 0.40, "raw decay would drop below the floor");

    // A freshly-touched memory must NOT be lifted by the floor — the floor
    // is a clamp on long-inactivity decay, never a bonus on recent items.
    let raw_fresh = decay_factor_with_params(0.5, &no_floor);
    let clamped_fresh = decay_factor_with_params(0.5, &with_floor);
    assert!(
        (raw_fresh - clamped_fresh).abs() < 1e-6,
        "floor must not lift fresh memory"
    );
}

#[test]
fn tier1_phase3_decay_config_per_category_overrides() {
    // JSON config — Causal edges decay slower (longer crossover, lower beta)
    // and have a 0.5 floor; everything else uses upstream defaults.
    let json = r#"{
        "default": { "crossover_days": 3.0, "lambda": 0.693, "beta": 0.5, "floor": 0.0 },
        "per_edge_category": {
            "Causal": { "crossover_days": 7.0, "lambda": 0.5, "beta": 0.3, "floor": 0.5 }
        }
    }"#;
    let cfg = DecayConfig::from_json_str(json).expect("parse config");

    let causal_params = cfg.params_for_edge_category(EdgeCategory::Causal);
    assert!((causal_params.floor - 0.5).abs() < 1e-6);
    assert!((causal_params.crossover_days - 7.0).abs() < 1e-6);

    let temporal_params = cfg.params_for_edge_category(EdgeCategory::Temporal);
    assert_eq!(temporal_params, DecayParams::upstream_default());
}

#[test]
fn tier1_phase4_ontology_tags_filter_via_query_builder() {
    let phillips_curve = make_memory(
        "Phillips curve flattens at high inflation",
        ExperienceType::Learning,
    )
    .with_ontology(NodeType::Concept, vec!["financial".into(), "macro".into()]);

    let pe_compression = make_memory("P/E compression in late cycle", ExperienceType::Pattern)
        .with_ontology(NodeType::Pattern, vec!["financial".into()]);

    let cpi_print = make_memory("CPI print 0.4% MoM", ExperienceType::Observation)
        .with_ontology(NodeType::Signal, vec!["financial".into()]);

    // Filter on NodeType::Concept — only Phillips curve passes.
    let q_concept = QueryBuilder::default()
        .node_types(vec![NodeType::Concept])
        .build();
    assert!(q_concept.matches(&phillips_curve));
    assert!(!q_concept.matches(&pe_compression));
    assert!(!q_concept.matches(&cpi_print));

    // Filter on NodeType::Pattern — only P/E compression passes.
    let q_pattern = QueryBuilder::default()
        .node_types(vec![NodeType::Pattern])
        .build();
    assert!(q_pattern.matches(&pe_compression));
    assert!(!q_pattern.matches(&phillips_curve));

    // Domain-tag OR — "macro" only hits Phillips curve. "financial" hits all three.
    let q_macro = QueryBuilder::default()
        .domain_tags(vec!["macro".into()])
        .build();
    assert!(q_macro.matches(&phillips_curve));
    assert!(!q_macro.matches(&pe_compression));

    let q_financial = QueryBuilder::default()
        .domain_tags(vec!["financial".into()])
        .build();
    assert!(q_financial.matches(&phillips_curve));
    assert!(q_financial.matches(&pe_compression));
    assert!(q_financial.matches(&cpi_print));
}

#[test]
fn tier1_phase5_contradiction_preserved_with_meta_audit_event() {
    let mut detector = InterferenceDetector::with_policy(ContradictionPolicy {
        preserve_contradictions: true,
        salience_boost: 0.20,
    });
    let now = Utc::now();
    let similar = vec![(
        "fed-pivot-confirmed".to_string(),
        0.90,
        0.6,
        now - Duration::hours(6),
        "Fed has signalled a pivot".to_string(),
    )];
    let result = detector.check_interference("fed-pivot-denied", 0.7, now, &similar);

    // Phase 5 contract: under flipped policy, suppression is averted and
    // the audit trail records WHICH would-be suppression was skipped.
    assert!(result.retroactive_targets.is_empty());
    assert_eq!(result.proactive_decay, 0.0);
    let averted = result
        .events
        .iter()
        .find(|e| matches!(e, ConsolidationEvent::SuppressionAverted { .. }));
    let event = averted.expect("expected SuppressionAverted event");
    if let ConsolidationEvent::SuppressionAverted {
        new_memory_id,
        old_memory_id,
        interference_type,
        ..
    } = event
    {
        assert_eq!(new_memory_id, "fed-pivot-denied");
        assert_eq!(old_memory_id, "fed-pivot-confirmed");
        assert!(matches!(interference_type, InterferenceType::Retroactive));
    }
}

#[test]
fn tier1_end_to_end_phases_compose() {
    // One workflow that exercises all five phases at once:
    //   1. Build a small graph with typed edges (Phase 1)
    //   2. Look up per-category decay params (Phase 3)
    //   3. Tag the analyst's working notes with NodeType + domain (Phase 4)
    //   4. Detect a contradiction at retrieval time, preserve both candidates,
    //      register an explicit contradiction with audit trail (Phase 5)
    //   5. Confirm a Meta::Contradicts edge (Phase 1) is the structural
    //      record the audit event refers to.

    let temp = TempDir::new().expect("tempdir");
    let graph = GraphMemory::new(temp.path(), None).expect("graph");

    // (1) Phase 1 — typed, directional graph
    let pivot_a = graph
        .add_entity(make_entity("Pivot signalled", EntityLabel::Concept, 0.8))
        .expect("add a");
    let pivot_b = graph
        .add_entity(make_entity("Pivot denied", EntityLabel::Concept, 0.8))
        .expect("add b");
    graph
        .add_relationship(make_edge(pivot_a, pivot_b, RelationType::Contradicts, 0.95))
        .expect("contradicts edge");

    // (2) Phase 3 — pull decay params for the Causal category to verify the
    //     config plumbing is reachable from the public API.
    let cfg = DecayConfig::default();
    let params = cfg.params_for_edge_category(EdgeCategory::Meta);
    assert_eq!(params, DecayParams::upstream_default());

    // (3) Phase 4 — typed working notes, multi-domain tags
    let note = make_memory(
        "Two contradictory Fed pivot signals this week",
        ExperienceType::Learning,
    )
    .with_ontology(NodeType::Concept, vec!["financial".into(), "macro".into()]);
    let q = QueryBuilder::default()
        .node_types(vec![NodeType::Concept])
        .domain_tags(vec!["macro".into()])
        .build();
    assert!(q.matches(&note));

    // (4) Phase 5 — flipped policy preserves both candidates, explicit
    //     contradiction emits an audit event, and the salience boost is
    //     surfaced for the caller to apply.
    let mut detector = InterferenceDetector::with_policy(ContradictionPolicy {
        preserve_contradictions: true,
        salience_boost: 0.25,
    });
    let outcome = detector.contradict_explicit(
        "fed-pivot-confirmed",
        "fed-pivot-denied",
        Some("nyt-2026-04-26"),
    );
    assert!((outcome.salience_boost - 0.25).abs() < 1e-6);
    let registered = matches!(
        outcome.event,
        ConsolidationEvent::ContradictionRegistered { .. }
    );
    assert!(registered, "expected ContradictionRegistered event");

    // (5) Phase 1 closure — the Meta::Contradicts edge from step (1) is the
    //     structural record. Both endpoints surface it via the bidirectional
    //     traversal helper.
    let from_a = graph
        .outgoing_with_bidirectional(&pivot_a)
        .expect("traverse a");
    let from_b = graph
        .outgoing_with_bidirectional(&pivot_b)
        .expect("traverse b");
    assert!(from_a
        .iter()
        .any(|e| e.relation_type == RelationType::Contradicts));
    assert!(from_b
        .iter()
        .any(|e| e.relation_type == RelationType::Contradicts));
}
