use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use ratatui::widgets::ListState;
use ifcl_core::{
    Event, WorkerProfile, Mission, Bank, LoopStatus, AppMode, MenuState, SetupWizard,
    groups::WorkerGroup,
    learning::{Insight, Optimization},
};
use crate::relationship::MentalMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOutput {
    pub timestamp: DateTime<Utc>,
    pub worker_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMode {
    None,
    Roster,
    MissionControl,
    MentalMap,
    Feed,
    Terminal,
    Learnings,
}

pub struct AppState {
    pub events: Vec<Event>,
    pub workers: Vec<WorkerProfile>,
    pub missions: Vec<Mission>,
    pub bank: Bank,
    pub status: LoopStatus,
    pub is_intervening: bool,
    pub input_buffer: String,
    pub mental_map: MentalMap,
    pub mode: AppMode,
    pub menu: MenuState,
    pub wizard: SetupWizard,
    pub current_session_id: Option<Uuid>,
    pub available_sessions: Vec<Uuid>,
    pub selected_session_index: usize,
    pub ai_outputs: Vec<AiOutput>,
    pub available_groups: Vec<WorkerGroup>,
    pub insights: Vec<Insight>,
    pub optimizations: Vec<Optimization>,
    pub managed_context_stats: Option<(usize, usize)>, // (tokens, pruned)
    pub recorded_missions: std::collections::HashSet<Uuid>,
    pub progress_stats: Option<ifcl_core::ProgressStats>,
    pub last_event_at: DateTime<Utc>,
    pub last_event_type: String,
    pub pulse: bool,
    pub feed_state: ListState,
    pub selected_event_index: Option<usize>,
    pub show_event_details: bool,
    pub focus_mode: FocusMode,
    pub frame_count: u64,
}

impl AppState {
    pub fn new(available_groups: Vec<WorkerGroup>, wizard: SetupWizard) -> Self {
        Self {
            events: Vec::new(),
            workers: Vec::new(),
            missions: Vec::new(),
            bank: Bank::default(),
            status: LoopStatus::Paused,
            is_intervening: false,
            input_buffer: String::new(),
            mental_map: MentalMap::new(),
            mode: AppMode::MainMenu,
            menu: MenuState::new(),
            wizard,
            current_session_id: None,
            available_sessions: Vec::new(),
            selected_session_index: 0,
            ai_outputs: Vec::new(),
            available_groups,
            insights: Vec::new(),
            optimizations: Vec::new(),
            managed_context_stats: None,
            recorded_missions: std::collections::HashSet::new(),
            progress_stats: None,
            last_event_at: Utc::now(),
            last_event_type: "Waiting...".to_string(),
            pulse: false,
            feed_state: ListState::default(),
            selected_event_index: None,
            show_event_details: false,
            focus_mode: FocusMode::None,
            frame_count: 0,
        }
    }
}
