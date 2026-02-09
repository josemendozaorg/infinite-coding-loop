use crate::state::{AiOutput, AppState, FocusMode};
use chrono::Utc;
use crossterm::event::KeyCode;
use ifcl_core::{
    learning::{LearningManager, MissionOutcome},
    orchestrator::Orchestrator,
    planner::{BasicPlanner, LLMPlanner, Planner},
    AiProvider, AppMode, CliExecutor, Event, EventBus, EventStore, LogPayload, LoopStatus,
    MenuAction, Mission, SetupWizard, TaskStatus, WizardStep, WorkerOutputPayload, WorkerProfile,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct App {
    pub state: Arc<Mutex<AppState>>,
    pub bus: Arc<dyn EventBus>,
    pub store: Arc<dyn EventStore>,
    pub orchestrator: Arc<dyn Orchestrator>,
    pub learning_manager: Arc<dyn LearningManager>,
}

impl App {
    pub fn new(
        state: Arc<Mutex<AppState>>,
        bus: Arc<dyn EventBus>,
        store: Arc<dyn EventStore>,
        orchestrator: Arc<dyn Orchestrator>,
        learning_manager: Arc<dyn LearningManager>,
    ) -> Self {
        Self {
            state,
            bus,
            store,
            orchestrator,
            learning_manager,
        }
    }

    pub async fn process_event(&self, event: Event) {
        let _ = self.store.append(event.clone()).await;

        let bus_c = Arc::clone(&self.bus);
        let learning_c = Arc::clone(&self.learning_manager);
        let state_c = Arc::clone(&self.state);

        if let Ok(mut s) = self.state.lock() {
            s.last_event_at = Utc::now();
            s.last_event_type = event.event_type.clone();
            s.pulse = !s.pulse;

            if Some(event.session_id) != s.current_session_id && event.session_id != Uuid::nil() {
                return;
            }

            let event_cloned = event.clone();
            match event_cloned.event_type.as_str() {
                "WorkerJoined" => {
                    if let Ok(profile) = serde_json::from_str::<WorkerProfile>(&event.payload) {
                        s.workers.push(profile);
                    }
                }
                "MissionCreated" => {
                    if let Ok(mission) = serde_json::from_str::<Mission>(&event.payload) {
                        s.mental_map.add_mission(mission.id, &mission.name);
                        for task in &mission.tasks {
                            s.mental_map.add_task(mission.id, task.id, &task.name);
                            if let Some(assigned) = &task.assigned_worker {
                                s.mental_map.assign_worker(task.id, assigned);
                            }
                        }
                        s.missions.push(mission);
                    }
                }
                "TaskUpdated" => {
                    #[derive(serde::Deserialize)]
                    struct TaskUpdate {
                        mission_id: Uuid,
                        task_id: Uuid,
                        status: TaskStatus,
                    }
                    if let Ok(update) = serde_json::from_str::<TaskUpdate>(&event.payload) {
                        if let Some(m) = s.missions.iter_mut().find(|m| m.id == update.mission_id) {
                            if let Some(t) = m.tasks.iter_mut().find(|t| t.id == update.task_id) {
                                t.status = update.status;
                                if update.status == TaskStatus::Success {
                                    let m_bus: Arc<dyn ifcl_core::EventBus> = Arc::clone(&self.bus);
                                    let sid = event.session_id;
                                    let tid = event.trace_id;
                                    tokio::spawn(async move {
                                        let _ = m_bus
                                            .publish(Event {
                                                id: Uuid::new_v4(),
                                                session_id: sid,
                                                trace_id: tid,
                                                timestamp: Utc::now(),
                                                worker_id: "system".to_string(),
                                                event_type: "RewardEarned".to_string(),
                                                payload: r#"{"xp":25,"coins":10}"#.to_string(),
                                            })
                                            .await;
                                    });
                                }
                            }
                        }
                    }
                }
                "RewardEarned" => {
                    if let Ok(reward) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                        let xp = reward["xp"].as_u64().unwrap_or(0);
                        let coins = reward["coins"].as_u64().unwrap_or(0);
                        s.bank.deposit(xp, coins);
                    }
                }
                "AiResponse" => {
                    s.ai_outputs.push(AiOutput {
                        timestamp: event.timestamp,
                        worker_id: event.worker_id.clone(),
                        content: event.payload.clone(),
                    });
                    if s.ai_outputs.len() > 50 {
                        s.ai_outputs.remove(0);
                    }
                }
                "WorkerOutput" => {
                    if let Ok(payload) = serde_json::from_str::<WorkerOutputPayload>(&event.payload)
                    {
                        if let Some(latest) = s.ai_outputs.last_mut() {
                            if latest.worker_id == event.worker_id {
                                latest.content.push_str(&payload.content);
                                latest.content.push('\n');
                            } else {
                                s.ai_outputs.push(AiOutput {
                                    timestamp: event.timestamp,
                                    worker_id: event.worker_id.clone(),
                                    content: payload.content,
                                });
                            }
                        } else {
                            s.ai_outputs.push(AiOutput {
                                timestamp: event.timestamp,
                                worker_id: event.worker_id.clone(),
                                content: payload.content,
                            });
                        }
                        if s.ai_outputs.len() > 50 {
                            s.ai_outputs.remove(0);
                        }
                    }
                }
                "LoopStatusChanged" => {
                    if let Ok(status) = serde_json::from_str::<LoopStatus>(&event.payload) {
                        s.status = status;
                    }
                }
                "ManualCommandInjected" => {
                    let cmd = event.payload.to_lowercase();
                    if cmd == "force success" {
                        for m in &mut s.missions {
                            for t in &mut m.tasks {
                                if t.status == TaskStatus::Running
                                    || t.status == TaskStatus::Pending
                                {
                                    t.status = TaskStatus::Success;
                                    let b_rew = Arc::clone(&bus_c);
                                    let sid = event.session_id;
                                    let tid = event.trace_id;
                                    tokio::spawn(async move {
                                        let _ = b_rew
                                            .publish(Event {
                                                id: Uuid::new_v4(),
                                                session_id: sid,
                                                trace_id: tid,
                                                timestamp: Utc::now(),
                                                worker_id: "system".to_string(),
                                                event_type: "RewardEarned".to_string(),
                                                payload: r#"{"xp":50,"coins":20}"#.to_string(),
                                            })
                                            .await;
                                    });
                                    break;
                                }
                            }
                        }
                    } else if cmd == "force failure" {
                        for m in &mut s.missions {
                            for t in &mut m.tasks {
                                if t.status == TaskStatus::Running
                                    || t.status == TaskStatus::Pending
                                {
                                    t.status = TaskStatus::Failure;
                                    let bus_f = Arc::clone(&bus_c);
                                    let sid = event.session_id;
                                    let tid = event.trace_id;
                                    let name_cloned = t.name.clone();
                                    tokio::spawn(async move {
                                        let _ = bus_f
                                            .publish(Event {
                                                id: Uuid::new_v4(),
                                                session_id: sid,
                                                trace_id: tid,
                                                timestamp: Utc::now(),
                                                worker_id: "system".to_string(),
                                                event_type: "Log".to_string(),
                                                payload: serde_json::to_string(&LogPayload {
                                                    level: "ERROR".to_string(),
                                                    message: format!(
                                                        "GOD MODE: Forced failure on task '{}'",
                                                        name_cloned
                                                    ),
                                                })
                                                .unwrap(),
                                            })
                                            .await;
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
                "Log" => {
                    if let Ok(payload) = serde_json::from_str::<LogPayload>(&event.payload) {
                        s.last_event_type = format!("{}: {}", payload.level, payload.message)
                            .chars()
                            .take(40)
                            .collect();
                        s.ai_outputs.push(AiOutput {
                            timestamp: event.timestamp,
                            worker_id: event.worker_id.clone(),
                            content: format!("LOG [{}]: {}", payload.level, payload.message),
                        });
                        if s.ai_outputs.len() > 50 {
                            s.ai_outputs.remove(0);
                        }
                    }
                }
                _ => {}
            }

            if s.events.len() > 100 {
                s.events.remove(0);
            }
            s.events.push(event);

            let mut just_completed = Vec::new();
            for m in &s.missions {
                if !s.recorded_missions.contains(&m.id)
                    && !m.tasks.is_empty()
                    && m.tasks
                        .iter()
                        .all(|t| t.status == TaskStatus::Success || t.status == TaskStatus::Failure)
                {
                    just_completed.push(m.clone());
                }
            }

            if !just_completed.is_empty() {
                let l_c = Arc::clone(&learning_c);
                let s_cc = Arc::clone(&state_c);

                for m in &just_completed {
                    s.recorded_missions.insert(m.id);
                }

                tokio::spawn(async move {
                    for m in just_completed {
                        let success = m.tasks.iter().all(|t| t.status == TaskStatus::Success);
                        let errors: Vec<String> = m
                            .tasks
                            .iter()
                            .filter(|t| t.status == TaskStatus::Failure)
                            .map(|t| format!("{}: General Failure", t.name))
                            .collect();

                        let outcome = MissionOutcome {
                            mission_id: m.id,
                            success,
                            duration_seconds: 0,
                            metadata: if success {
                                serde_json::json!({"name": m.name})
                            } else {
                                serde_json::json!({
                                    "name": m.name,
                                    "error": if errors.is_empty() { "Unknown error".to_string() } else { errors.join("; ") }
                                })
                            },
                        };
                        let _ = l_c.record_outcome(outcome).await;
                    }
                    if let Ok(bits) = l_c.analyze_history().await {
                        if let Ok(mut state) = s_cc.lock() {
                            state.insights = bits;
                        }
                    }
                    if let Ok(opts) = l_c.propose_optimizations().await {
                        if let Ok(mut state) = s_cc.lock() {
                            state.optimizations = opts;
                        }
                    }
                });
            }
        }
    }

    pub async fn handle_input(&self, key: KeyCode) -> bool {
        let mut s = self.state.lock().unwrap();
        let current_mode = s.mode.clone();

        match current_mode {
            AppMode::MainMenu => {
                match key {
                    KeyCode::Char('q') => return true,
                    KeyCode::Down | KeyCode::Char('j') => s.menu.next(),
                    KeyCode::Up | KeyCode::Char('k') => s.menu.previous(),
                    KeyCode::Enter => {
                        match s.menu.current_action() {
                            MenuAction::NewGame => {
                                let _ = s.wizard.clone(); // Just to use it if needed, or:
                                *s = AppState::new(s.available_groups.clone(), SetupWizard::new());
                                s.mode = AppMode::Setup;
                            }
                            MenuAction::LoadGame => {
                                let store_lp = Arc::clone(&self.store);
                                let s_lp = Arc::clone(&self.state);
                                tokio::spawn(async move {
                                    if let Ok(sessions) = store_lp.list_all_sessions().await {
                                        let mut state = s_lp.lock().unwrap();
                                        state.available_sessions = sessions;
                                        state.selected_session_index = 0;
                                        state.mode = AppMode::SessionPicker;
                                    }
                                });
                            }
                            MenuAction::Quit => return true,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            AppMode::Setup => match key {
                KeyCode::Esc => s.mode = AppMode::MainMenu,
                KeyCode::Enter => {
                    if s.wizard.current_step == WizardStep::Summary {
                        let sid = Uuid::new_v4();
                        s.current_session_id = Some(sid);
                        s.mode = AppMode::Running;
                        s.status = LoopStatus::Running;

                        let bus_m = Arc::clone(&self.bus);
                        let orch_m = Arc::clone(&self.orchestrator);
                        let goal = s.wizard.goal.clone();
                        let workspace = if s.wizard.workspace_path.is_empty() {
                            None
                        } else {
                            Some(s.wizard.workspace_path.clone())
                        };
                        let provider = s.wizard.provider.clone();

                        tokio::spawn(async move {
                            let planner: Arc<dyn Planner> = match provider {
                                AiProvider::Basic => Arc::new(BasicPlanner),
                                AiProvider::Gemini => Arc::new(LLMPlanner {
                                    executor: CliExecutor::new(
                                        "gemini".to_string(),
                                        vec![
                                            "--approval-mode".to_string(),
                                            "yolo".to_string(),
                                            "--allowed-tools".to_string(),
                                            "run_shell_command".to_string(),
                                        ],
                                    ),
                                }),
                                AiProvider::Claude => Arc::new(LLMPlanner {
                                    executor: CliExecutor::new("claude".to_string(), vec![]),
                                }),
                                AiProvider::OpenCode => Arc::new(LLMPlanner {
                                    executor: CliExecutor::new("opencode".to_string(), vec![]),
                                }),
                            };
                            let mut missions = planner.generate_initial_missions(&goal).await;

                            for mission in &mut missions {
                                mission.session_id = sid;
                                mission.workspace_path = workspace.clone();
                            }

                            for mission in missions {
                                let _ = orch_m.add_mission(mission.clone()).await;
                                let _ = bus_m
                                    .publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: "system".to_string(),
                                        event_type: "MissionCreated".to_string(),
                                        payload: serde_json::to_string(&mission).unwrap(),
                                    })
                                    .await;
                            }
                        });
                    } else {
                        let _ = s.wizard.next();
                    }
                }
                KeyCode::Backspace => {
                    if s.wizard.current_step == WizardStep::Goal {
                        s.wizard.goal.pop();
                    } else if s.wizard.current_step == WizardStep::Workspace {
                        s.wizard.workspace_path.pop();
                    } else {
                        s.wizard.prev();
                    }
                }
                KeyCode::Up => {
                    if s.wizard.current_step == WizardStep::Team
                        && s.wizard.selected_group_index > 0
                    {
                        s.wizard.selected_group_index -= 1;
                    } else if s.wizard.current_step == WizardStep::Provider {
                        s.wizard.provider = match s.wizard.provider {
                            AiProvider::Gemini => AiProvider::Basic,
                            AiProvider::Claude => AiProvider::Gemini,
                            AiProvider::OpenCode => AiProvider::Claude,
                            AiProvider::Basic => AiProvider::OpenCode,
                        };
                    }
                }
                KeyCode::Down => {
                    if s.wizard.current_step == WizardStep::Team
                        && s.wizard.selected_group_index
                            < s.available_groups.len().saturating_sub(1)
                    {
                        s.wizard.selected_group_index += 1;
                    } else if s.wizard.current_step == WizardStep::Provider {
                        s.wizard.provider = match s.wizard.provider {
                            AiProvider::Gemini => AiProvider::Claude,
                            AiProvider::Claude => AiProvider::OpenCode,
                            AiProvider::OpenCode => AiProvider::Basic,
                            AiProvider::Basic => AiProvider::Gemini,
                        };
                    }
                }
                KeyCode::Left => {
                    s.wizard.prev();
                }
                KeyCode::Char(c) => {
                    if s.wizard.current_step == WizardStep::Goal {
                        s.wizard.goal.push(c);
                    } else if s.wizard.current_step == WizardStep::Workspace {
                        s.wizard.workspace_path.push(c);
                    }
                }
                _ => {}
            },
            AppMode::Running => {
                if s.show_event_details {
                    match key {
                        KeyCode::Esc | KeyCode::Enter => {
                            s.show_event_details = false;
                        }
                        _ => {}
                    }
                } else if s.is_intervening {
                    match key {
                        KeyCode::Esc => {
                            s.is_intervening = false;
                            s.input_buffer.clear();
                        }
                        KeyCode::Enter => {
                            let cmd = s.input_buffer.clone();
                            s.is_intervening = false;
                            s.input_buffer.clear();
                            let bus_g = Arc::clone(&self.bus);
                            let sid = s.current_session_id.unwrap_or_default();
                            tokio::spawn(async move {
                                let _ = bus_g
                                    .publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: "god".to_string(),
                                        event_type: "ManualCommandInjected".to_string(),
                                        payload: cmd,
                                    })
                                    .await;
                            });
                        }
                        KeyCode::Char(c) => {
                            s.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            s.input_buffer.pop();
                        }
                        _ => {}
                    }
                } else {
                    match key {
                        KeyCode::Char('q') => {
                            s.mode = AppMode::MainMenu;
                        }
                        KeyCode::Char('i') => {
                            s.is_intervening = true;
                            s.input_buffer.clear();
                        }
                        KeyCode::Char(' ') => {
                            let new_status = match s.status {
                                LoopStatus::Running => LoopStatus::Paused,
                                LoopStatus::Paused => LoopStatus::Running,
                            };
                            let bus_p = Arc::clone(&self.bus);
                            let sid = s.current_session_id.unwrap_or_default();
                            tokio::spawn(async move {
                                let _ = bus_p
                                    .publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: "user".to_string(),
                                        event_type: "LoopStatusChanged".to_string(),
                                        payload: serde_json::to_string(&new_status).unwrap(),
                                    })
                                    .await;
                            });
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            let count = s.events.len();
                            if count > 0 {
                                let i = match s.feed_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            count - 1
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => count - 1,
                                };
                                s.feed_state.select(Some(i));
                                s.selected_event_index = Some(i);
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let count = s.events.len();
                            if count > 0 {
                                let i = match s.feed_state.selected() {
                                    Some(i) => {
                                        if i >= count - 1 {
                                            0
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                s.feed_state.select(Some(i));
                                s.selected_event_index = Some(i);
                            }
                        }
                        KeyCode::Enter => {
                            if s.selected_event_index.is_some() {
                                s.show_event_details = true;
                            }
                        }
                        KeyCode::Esc => {
                            s.selected_event_index = None;
                            s.feed_state.select(None);
                        }
                        KeyCode::Char('1') => {
                            s.focus_mode = if s.focus_mode == FocusMode::Roster {
                                FocusMode::None
                            } else {
                                FocusMode::Roster
                            };
                        }
                        KeyCode::Char('2') => {
                            s.focus_mode = if s.focus_mode == FocusMode::MissionControl {
                                FocusMode::None
                            } else {
                                FocusMode::MissionControl
                            };
                        }
                        KeyCode::Char('3') => {
                            s.focus_mode = if s.focus_mode == FocusMode::MentalMap {
                                FocusMode::None
                            } else {
                                FocusMode::MentalMap
                            };
                        }
                        KeyCode::Char('4') => {
                            s.focus_mode = if s.focus_mode == FocusMode::Feed {
                                FocusMode::None
                            } else {
                                FocusMode::Feed
                            };
                        }
                        KeyCode::Char('5') => {
                            s.focus_mode = if s.focus_mode == FocusMode::Terminal {
                                FocusMode::None
                            } else {
                                FocusMode::Terminal
                            };
                        }
                        KeyCode::Char('6') => {
                            s.focus_mode = if s.focus_mode == FocusMode::Learnings {
                                FocusMode::None
                            } else {
                                FocusMode::Learnings
                            };
                        }
                        KeyCode::Char('0') => {
                            s.focus_mode = FocusMode::None;
                        }
                        _ => {}
                    }
                }
            }
            AppMode::SessionPicker => {
                match key {
                    KeyCode::Esc => s.mode = AppMode::MainMenu,
                    KeyCode::Down | KeyCode::Char('j') => {
                        if s.selected_session_index < s.available_sessions.len().saturating_sub(1) {
                            s.selected_session_index += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if s.selected_session_index > 0 {
                            s.selected_session_index -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(sid) =
                            s.available_sessions.get(s.selected_session_index).cloned()
                        {
                            s.current_session_id = Some(sid);
                            // Session loading logic would go here
                        }
                    }
                    _ => {}
                }
            }
            AppMode::Marketplace => {
                // TODO: Marketplace input handling
            }
        }
        false
    }
}
