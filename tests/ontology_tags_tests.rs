//! Phase 4 (cognitive-engine): integration tests for ontology tagging.
//!
//! Validates the public API surface — `NodeType`, `Memory::with_ontology`,
//! `Query.node_types`, `Query.domain_tags`, and `QueryBuilder` — is
//! reachable from a downstream crate and behaves correctly end-to-end.

use shodh_memory::memory::{
    Experience, ExperienceType, Memory, MemoryId, NodeType, Query, QueryBuilder,
};
use shodh_memory::uuid::Uuid;

fn make(content: &str, et: ExperienceType, node_type: NodeType, tags: Vec<&str>) -> Memory {
    let exp = Experience {
        experience_type: et,
        content: content.to_string(),
        ..Default::default()
    };
    Memory::new(MemoryId(Uuid::new_v4()), exp, 0.5, None, None, None, None)
        .with_ontology(node_type, tags.into_iter().map(String::from).collect())
}

// Bincode round-trip is covered by the lib unit test
// `memory_serde_roundtrip_preserves_ontology` in src/memory/types.rs.
// Including it here as well triggers a Cargo dep-resolution edge case
// caused by the upstream `crate-type = ["rlib", "cdylib"]` setting on the
// shodh-memory lib target — when an integration test re-links the rlib
// while bincode pulls a second serde rmeta, the test crate fails with
// E0277 "Memory: Serialize not satisfied" even though Memory implements
// Serialize. Keeping the bincode check at the lib-internal layer avoids
// this without losing coverage.

#[test]
fn legacy_memory_defaults_to_legacy_node_type() {
    let exp = Experience {
        experience_type: ExperienceType::Observation,
        content: "untyped".to_string(),
        ..Default::default()
    };
    let m = Memory::new(MemoryId(Uuid::new_v4()), exp, 0.5, None, None, None, None);
    assert!(m.node_type.is_legacy());
    assert!(m.domain_tags.is_empty());
}

#[test]
fn query_filters_by_node_type_and_domain_tag_combined() {
    let memories = vec![
        make(
            "a",
            ExperienceType::Learning,
            NodeType::Concept,
            vec!["financial"],
        ),
        make(
            "b",
            ExperienceType::Learning,
            NodeType::Concept,
            vec!["operations"],
        ),
        make(
            "c",
            ExperienceType::Discovery,
            NodeType::Concept,
            vec!["financial"],
        ),
        make(
            "d",
            ExperienceType::Pattern,
            NodeType::Pattern,
            vec!["financial"],
        ),
        make(
            "e",
            ExperienceType::Observation,
            NodeType::Entity,
            vec!["financial"],
        ),
    ];

    // Concept ∩ financial: a + c only.
    let q: Query = QueryBuilder::default()
        .node_types(vec![NodeType::Concept])
        .domain_tags(vec!["financial".to_string()])
        .build();

    let pass: Vec<&Memory> = memories.iter().filter(|m| q.matches(m)).collect();
    let pass_contents: Vec<&str> = pass.iter().map(|m| m.experience.content.as_str()).collect();
    assert_eq!(
        pass_contents.len(),
        2,
        "expected 2 hits, got {pass_contents:?}"
    );
    assert!(pass_contents.contains(&"a"));
    assert!(pass_contents.contains(&"c"));
}

#[test]
fn nodetype_default_for_experience_type_is_total() {
    // Sanity-check that the suggestion mapping is reachable via the
    // public API and covers every variant — used by the migration tool.
    let variants = [
        ExperienceType::Conversation,
        ExperienceType::Decision,
        ExperienceType::Error,
        ExperienceType::Learning,
        ExperienceType::Discovery,
        ExperienceType::Pattern,
        ExperienceType::Context,
        ExperienceType::Task,
        ExperienceType::CodeEdit,
        ExperienceType::FileAccess,
        ExperienceType::Search,
        ExperienceType::Command,
        ExperienceType::Observation,
        ExperienceType::Intention,
    ];
    for et in variants.iter() {
        // Every variant must map to a non-Legacy NodeType.
        let suggested = NodeType::default_for_experience_type(et);
        assert!(!suggested.is_legacy(), "{:?} should not map to Legacy", et);
    }
}
