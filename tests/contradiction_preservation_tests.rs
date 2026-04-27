//! Phase 5 (cognitive-engine): integration tests for the
//! contradiction-preservation policy that flips upstream's
//! similarity-suppression engine.
//!
//! The detector lives inside `shodh_memory::memory::replay` and is
//! reachable from a downstream crate via that path. These tests pin
//! down two things:
//!
//!   1. The default policy is byte-equivalent to upstream — every
//!      pre-Phase-5 caller sees identical behavior.
//!   2. Flipping `preserve_contradictions = true` averts decay /
//!      suppression at both write time (`check_interference`) and
//!      read time (`apply_retrieval_competition`), and an explicit
//!      `contradict_explicit()` call returns the configured salience
//!      boost plus a `ContradictionRegistered` event for the audit
//!      trail.

use chrono::{Duration, Utc};
use shodh_memory::memory::introspection::{ConsolidationEvent, InterferenceType};
use shodh_memory::memory::replay::{ContradictionPolicy, InterferenceDetector};

#[test]
fn default_policy_preserves_upstream_suppression_semantics() {
    let mut detector = InterferenceDetector::new();
    let now = Utc::now();
    let similar = vec![(
        "old-mem".to_string(),
        0.90,
        0.5,
        now - Duration::hours(12),
        "Old memory content".to_string(),
    )];
    let result = detector.check_interference("new-mem", 0.7, now, &similar);
    // Upstream behavior: retroactive suppression fires on a vulnerable
    // older memory of lower importance.
    assert!(!result.retroactive_targets.is_empty());
    assert!(!result.is_duplicate);
    assert!(result
        .events
        .iter()
        .any(|e| matches!(e, ConsolidationEvent::InterferenceDetected { .. })));
}

#[test]
fn flipped_policy_preserves_contradicting_evidence_at_write_time() {
    let mut detector = InterferenceDetector::with_policy(ContradictionPolicy {
        preserve_contradictions: true,
        salience_boost: 0.20,
    });
    let now = Utc::now();
    // Two memories about the same topic but contradicting each other —
    // upstream would weaken whichever is less important. Under Phase 5
    // both are kept and the would-be suppression is logged.
    let similar = vec![(
        "fed-pivot-confirmed".to_string(),
        0.90,
        0.6,
        now - Duration::hours(6),
        "Fed has signalled a pivot".to_string(),
    )];
    let result = detector.check_interference("fed-pivot-denied", 0.7, now, &similar);
    assert!(result.retroactive_targets.is_empty());
    assert_eq!(result.proactive_decay, 0.0);
    let averted = result.events.iter().find_map(|e| match e {
        ConsolidationEvent::SuppressionAverted {
            new_memory_id,
            old_memory_id,
            interference_type,
            ..
        } => Some((
            new_memory_id.clone(),
            old_memory_id.clone(),
            interference_type.clone(),
        )),
        _ => None,
    });
    let (new_id, old_id, kind) = averted.expect("expected SuppressionAverted event");
    assert_eq!(new_id, "fed-pivot-denied");
    assert_eq!(old_id, "fed-pivot-confirmed");
    assert!(matches!(kind, InterferenceType::Retroactive));
}

#[test]
fn flipped_policy_keeps_all_competitors_at_retrieval_time() {
    let mut detector = InterferenceDetector::with_policy(ContradictionPolicy {
        preserve_contradictions: true,
        ..Default::default()
    });
    // Three competing reads — under upstream, the close runner-up
    // would be suppressed. Under Phase 5 both close survivors stay
    // and a SuppressionAverted event captures the avoided loss.
    let candidates = vec![
        ("yields-up".to_string(), 0.90, 0.85),
        ("yields-down".to_string(), 0.88, 0.82),
        ("yields-flat".to_string(), 0.40, 0.60),
    ];
    let result = detector.apply_retrieval_competition(&candidates, "yields direction?");
    assert_eq!(result.winners.len(), 3);
    assert!(result.suppressed.is_empty());
}

#[test]
fn explicit_contradiction_emits_audit_event_and_boost() {
    let mut detector = InterferenceDetector::with_policy(ContradictionPolicy {
        preserve_contradictions: true,
        salience_boost: 0.25,
    });
    let result = detector.contradict_explicit("hypothesis-a", "hypothesis-b", Some("paper-2026"));
    assert!((result.salience_boost - 0.25).abs() < 1e-6);
    match result.event {
        ConsolidationEvent::ContradictionRegistered {
            node_a_id,
            node_b_id,
            evidence_id,
            salience_boost,
            ..
        } => {
            assert_eq!(node_a_id, "hypothesis-a");
            assert_eq!(node_b_id, "hypothesis-b");
            assert_eq!(evidence_id, Some("paper-2026".to_string()));
            assert!((salience_boost - 0.25).abs() < 1e-6);
        }
        other => panic!("expected ContradictionRegistered, got {:?}", other),
    }
}

#[test]
fn policy_can_be_swapped_on_an_existing_detector() {
    let mut detector = InterferenceDetector::new();
    let now = Utc::now();
    let similar = vec![(
        "old".to_string(),
        0.90,
        0.5,
        now - Duration::hours(12),
        String::new(),
    )];
    // Default policy → suppression fires.
    let r1 = detector.check_interference("new1", 0.7, now, &similar);
    assert!(!r1.retroactive_targets.is_empty());
    // Flip the policy on the same detector → next check averts.
    detector.set_policy(ContradictionPolicy {
        preserve_contradictions: true,
        salience_boost: 0.20,
    });
    let r2 = detector.check_interference("new2", 0.7, now, &similar);
    assert!(r2.retroactive_targets.is_empty());
    assert!(r2
        .events
        .iter()
        .any(|e| matches!(e, ConsolidationEvent::SuppressionAverted { .. })));
}
