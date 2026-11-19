#![forbid(unsafe_code)]
//! Pure Cloud AI subsystem: Zero-token static intent router, SSE multi-provider client, embeddings, security auditing, and generators.

pub mod client;
pub mod embeddings;
pub mod generators;
pub mod merge_assistant;
pub mod orchestrator;
pub mod router;
pub mod security_evaluator;

pub use client::{ChatMessage, CloudAiProvider, CloudAiService};
pub use embeddings::EmbeddingsClient;
pub use generators::AiGenerators;
pub use merge_assistant::{strip_markdown_fences, MergeAssistant};
pub use orchestrator::CloudAIOrchestrator;
pub use router::{AgentAction, IntentClassifier, StaticIntent};
pub use security_evaluator::CloudAiSecurityEvaluator;
