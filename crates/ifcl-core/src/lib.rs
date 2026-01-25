pub mod domain;
pub mod execution;
pub mod knowledge;
pub mod planning;
pub mod infrastructure;
pub mod identity;
pub mod interface;


// Top-level re-exports for convenience and backward compatibility
pub use domain::*;
pub use execution::orchestrator::{BasicOrchestrator, Orchestrator, WorkerRequest};
pub use execution::workers::base::Worker;
pub use execution::workers::agent::{Agent, AiCliAgent};
pub use execution::workers::ai::AiGenericWorker;
pub use execution::workers::cli::CliWorker;
pub use execution::workers::planner::{PlannerWorker, ReplanContext};
pub use execution::progress::{BasicProgressManager, ProgressManager, ProgressStats};
pub use planning::planner::{BasicPlanner, LLMPlanner, Planner};
pub use knowledge::memory::{InMemoryMemoryStore, MemoryStore, MemoryEntry};
pub use knowledge::learning::{BasicLearningManager, LearningManager};
pub use knowledge::enricher::ContextEnricher;
// Module Compatibility Exports
pub use planning::planner;
pub use knowledge::learning;
pub use knowledge::memory;
pub use execution::orchestrator;
pub use identity::groups;

pub use infrastructure::events::bus::{EventBus, InMemoryEventBus};
pub use infrastructure::events::store::{EventStore, InMemoryEventStore, SqliteEventStore};
pub use infrastructure::cli::{CliExecutor, CliResult};
pub use infrastructure::workspace::{WorkspaceStatus, validate_workspace_path};
pub use identity::groups::WorkerGroup;
pub use identity::marketplace::MarketplaceLoader;
pub use interface::ui_state::{AppMode, MenuAction, MenuState};
pub use interface::wizard::{AiProvider, SetupWizard, WizardStep};
