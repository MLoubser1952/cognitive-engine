//! Modular Query Parsing System
//!
//! Provides a trait-based abstraction for query parsing, allowing easy swapping
//! between rule-based and LLM-based implementations.
//!
//! # Architecture
//! ```text
//! Query → QueryParser (trait) → ParsedQuery
//!              ↓
//!     ┌────────┴────────┐
//!     │                 │
//! RuleBasedParser   LlmParser
//! (YAKE/regex)      (Qwen 1.5B)
//! ```
//!
//! # Usage
//! ```rust,ignore
//! let parser = create_parser(ParserConfig::default());
//! let parsed = parser.parse("When did Melanie paint a sunrise?", Some(conv_date))?;
//! ```

mod llm_parser;
mod parser_trait;
mod rule_based;

pub use llm_parser::{ApiType, LlmParser};
pub use parser_trait::*;
pub use rule_based::RuleBasedParser;

use std::sync::Arc;

/// Parser implementation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParserType {
    /// Rule-based parsing using YAKE, regex, and heuristics (default)
    #[default]
    RuleBased,
    /// LLM-based parsing using Qwen 1.5B or similar
    Llm,
}

/// Configuration for the query parser
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Which parser implementation to use
    pub parser_type: ParserType,
    /// HTTP endpoint for the LLM server (Ollama / LM Studio / vLLM).
    /// Only used when `parser_type == Llm`.
    pub llm_endpoint: Option<String>,
    /// Model name to request from the LLM server (e.g. "qwen2.5:1.5b").
    /// Only used when `parser_type == Llm`.
    pub llm_model: Option<String>,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            parser_type: ParserType::RuleBased,
            llm_endpoint: None,
            llm_model: None,
        }
    }
}

impl ParserConfig {
    /// Create config for rule-based parser
    pub fn rule_based() -> Self {
        Self::default()
    }

    /// Create config for LLM parser pointing at an HTTP-served model.
    pub fn llm(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            parser_type: ParserType::Llm,
            llm_endpoint: Some(endpoint.into()),
            llm_model: Some(model.into()),
        }
    }
}

/// Create a parser based on configuration
pub fn create_parser(config: ParserConfig) -> Arc<dyn QueryParser> {
    match config.parser_type {
        ParserType::RuleBased => Arc::new(RuleBasedParser::new()),
        #[cfg(feature = "llm-parser")]
        ParserType::Llm => {
            let endpoint = config
                .llm_endpoint
                .expect("LLM endpoint required for LLM parser");
            let model = config
                .llm_model
                .expect("LLM model name required for LLM parser");
            Arc::new(LlmParser::new(&endpoint, &model))
        }
        #[cfg(not(feature = "llm-parser"))]
        ParserType::Llm => {
            tracing::warn!("LLM parser requested but 'llm-parser' feature not enabled, falling back to rule-based");
            Arc::new(RuleBasedParser::new())
        }
    }
}
