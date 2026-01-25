use tui::app::App;
use tui::state::AppState;
use tui::cli::CliArgs;
use tui::ui; // If used

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use crossterm::{
    event::{self, Event as CEvent},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ifcl_core::{
    learning::{BasicLearningManager, LearningManager},
    planner::{BasicPlanner, LLMPlanner, Planner},
    AiCliAgent,
    AiGenericWorker,
    CliExecutor,
    CliWorker,
    Event,
    EventStore,
    InMemoryEventBus,
    LoopStatus,
    MarketplaceLoader,
    SetupWizard,
    SqliteEventStore,
    TaskStatus,
    Worker, // Import Worker trait
    WorkerOutputPayload,
    WorkerRole,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn get_sid(state: &Arc<Mutex<AppState>>) -> Uuid {
    state.lock().unwrap().current_session_id.unwrap_or_default()
}

fn get_goal(state: &Arc<Mutex<AppState>>) -> String {
    state.lock().unwrap().wizard.goal.clone()
}

fn is_running(state: &Arc<Mutex<AppState>>) -> bool {
    state.lock().unwrap().status == LoopStatus::Running
}

fn are_missions_done(state: &Arc<Mutex<AppState>>) -> bool {
    let s = state.lock().unwrap();
    !s.missions.is_empty()
        && s.missions.iter().all(|m| {
            m.tasks
                .iter()
                .all(|t| t.status == TaskStatus::Success || t.status == TaskStatus::Failure)
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    let _args = CliArgs::parse();

    // 1. Initialize Infrastructure (Shared)
    let bus: Arc<dyn ifcl_core::EventBus> = Arc::new(InMemoryEventBus::new(200));
    let store: Arc<dyn EventStore> =
        Arc::new(SqliteEventStore::new("sqlite://ifcl.db?mode=rwc").await?);
    let learning_manager: Arc<dyn LearningManager> = Arc::new(BasicLearningManager::new());
    let _memory_store: Arc<dyn ifcl_core::MemoryStore> =
        Arc::new(ifcl_core::memory::InMemoryMemoryStore::new());
    let orchestrator: Arc<dyn ifcl_core::Orchestrator> =
        Arc::new(ifcl_core::BasicOrchestrator::new());

    let provider = _args.provider.clone();
    let planner: Arc<dyn Planner> = match provider.as_deref() {
        Some("gemini") => Arc::new(LLMPlanner {
            executor: CliExecutor::new(
                "gemini".to_string(),
                vec!["--yolo".to_string()],
            ),
        }),
        Some("claude") => Arc::new(LLMPlanner {
            executor: CliExecutor::new("claude".to_string(), vec![]),
        }),
        Some("opencode") => Arc::new(LLMPlanner {
            executor: CliExecutor::new("opencode".to_string(), vec![]),
        }),
        _ => Arc::new(BasicPlanner),
    };

    // 2. Check for Headless Mode
    if _args.is_headless() {
        println!("🚀 Starting Infinite Coding Loop in Headless Mode...");
        let goal = _args
            .goal
            .clone()
            .unwrap_or_else(|| "General Autonomy".to_string());
        println!("Objective: {}", goal);

        let workspace = _args.workspace.clone();
        if let Some(ws) = &workspace {
            let _ = std::fs::create_dir_all(ws);
            println!("Workspace: {}", ws);
        }

        let mut missions = planner.generate_initial_missions(&goal).await;
        let sid = Uuid::new_v4();
        for m in &mut missions {
            m.session_id = sid;
            m.workspace_path = workspace.clone();
            orchestrator.add_mission(m.clone()).await?;
            println!("Mission Created: {} (ID: {})", m.name, m.id);
        }

        let mut rx = bus.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if event.event_type == "WorkerOutput" {
                    if let Ok(payload) = serde_json::from_str::<WorkerOutputPayload>(&event.payload)
                    {
                        print!("{}", payload.content);
                    }
                } else if event.event_type == "Log" {
                    if let Ok(payload) =
                        serde_json::from_str::<ifcl_core::LogPayload>(&event.payload)
                    {
                        println!("LOG [{}]: {}", payload.level, payload.message);
                    }
                }
            }
        });

        loop {
            let missions = orchestrator.get_missions().await?;
            let mut pending_task = None;
            let mut all_done = true;

            for m in missions {
                for t in m.tasks {
                    if t.status == TaskStatus::Pending {
                        let w_name = t
                            .assigned_worker
                            .clone()
                            .unwrap_or_else(|| "Headless-Bot".to_string());
                        pending_task = Some((m.id, t.id, w_name));
                        all_done = false;
                        break;
                    }
                    if t.status == TaskStatus::Running {
                        all_done = false;
                    }
                    if t.status == TaskStatus::Failure {
                        let _ = bus
                            .publish(Event {
                                id: Uuid::new_v4(),
                                session_id: sid,
                                trace_id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                worker_id: "system".to_string(),
                                event_type: "Log".to_string(),
                                payload: serde_json::to_string(&ifcl_core::LogPayload {
                                    level: "ERROR".to_string(),
                                    message: format!("❌ Task Failed: {}", t.name),
                                })
                                .unwrap(),
                            })
                            .await;
                        return Ok(());
                    }
                }
                if pending_task.is_some() {
                    break;
                }
            }

            if let Some((mid, tid, worker_name)) = pending_task {
                let w_lower = worker_name.to_lowercase();
                let worker: Box<dyn ifcl_core::Worker> = if w_lower.contains("gemini") || w_lower.contains("planner") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new(
                            "gemini".to_string(),
                            None,
                            vec!["--yolo".to_string(), "--allowed-tools".to_string(), "run_shell_command".to_string()],
                        )),
                    ))
                } else if w_lower.contains("claude") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new("claude".to_string(), None, vec![])),
                    ))
                } else if w_lower.contains("opencode") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new("opencode".to_string(), None, vec![])),
                    ))
                } else {
                    Box::new(CliWorker::new(&worker_name, WorkerRole::Coder))
                };

                let result = orchestrator
                    .execute_task(Arc::clone(&bus), mid, tid, worker.as_ref())
                    .await;
                match result {
                    Ok(_out) => {
                        // Orchestrator already logs success
                    }
                    Err(_e) => {
                        // Orchestrator already logs error
                        if let Ok(count) = orchestrator.increment_retry_count(mid, tid).await {
                            if count <= 3 {
                                let _ = bus
                                    .publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: "system".to_string(),
                                        event_type: "Log".to_string(),
                                        payload: serde_json::to_string(
                                            &ifcl_core::LogPayload {
                                                level: "WARN".to_string(),
                                                message: format!(
                                                    "⚠️ Retrying Task (Attempt {}/3)",
                                                    count
                                                ),
                                            },
                                        )
                                        .unwrap(),
                                    })
                                    .await;
                                let _ = orchestrator
                                    .update_task_status(mid, tid, TaskStatus::Pending)
                                    .await;
                                tokio::time::sleep(tokio::time::Duration::from_millis(500))
                                    .await;
                            } else {
                                let _ = bus.publish(Event {
                                     id: Uuid::new_v4(),
                                     session_id: sid,
                                     trace_id: Uuid::new_v4(),
                                     timestamp: Utc::now(),
                                     worker_id: "system".to_string(),
                                     event_type: "Log".to_string(),
                                     payload: serde_json::to_string(&ifcl_core::LogPayload {
                                         level: "ERROR".to_string(),
                                         message: format!("❌ Retries Exhausted. Escalating to Planner for task '{}'", mid),
                                     }).unwrap(),
                                 }).await;
                                let missions =
                                    orchestrator.get_missions().await.unwrap_or_default();
                                if let Some(mission) = missions.iter().find(|m| m.id == mid) {
                                    // Planner Worker Pattern (Headless)
                                    let replan_context = ifcl_core::ReplanContext {
                                        goal: goal.clone(),
                                        mission: mission.clone(),
                                        failed_task_id: tid,
                                    };
                                    let context_json =
                                        serde_json::to_string(&replan_context).unwrap();

                                    let replan_task = ifcl_core::Task {
                                        id: Uuid::new_v4(),
                                        name: format!("Replan Task {}", tid),
                                        description: context_json,
                                        status: TaskStatus::Pending,
                                        assigned_worker: Some("Planner".to_string()),
                                        retry_count: 0,
                                    };

                                    let planner_worker =
                                        ifcl_core::PlannerWorker::new(planner.clone());
                                    let _ = bus
                                        .publish(Event {
                                            id: Uuid::new_v4(),
                                            session_id: mission.session_id,
                                            trace_id: Uuid::new_v4(),
                                            timestamp: Utc::now(),
                                            worker_id: "system".to_string(),
                                            event_type: "Log".to_string(),
                                            payload: serde_json::to_string(
                                                &ifcl_core::LogPayload {
                                                    level: "INFO".to_string(),
                                                    message:
                                                        "🧠 Delegating to Planner Worker..."
                                                            .to_string(),
                                                },
                                            )
                                            .unwrap(),
                                        })
                                        .await;

                                    match planner_worker
                                        .execute(
                                            Arc::clone(&bus),
                                            &replan_task,
                                            ".",
                                            mission.session_id,
                                        )
                                        .await
                                    {
                                        Ok(output) => {
                                            if let Ok(new_missions) =
                                                serde_json::from_str::<Vec<ifcl_core::Mission>>(
                                                    &output,
                                                )
                                            {
                                                let sid_r = mission.session_id;
                                                let ws = mission.workspace_path.clone();

                                                for mut m in new_missions {
                                                    m.session_id = sid_r;
                                                    m.workspace_path = ws.clone();
                                                    orchestrator
                                                        .add_mission(m.clone())
                                                        .await
                                                        .unwrap();

                                                    let _ = bus.publish(Event {
                                                         id: Uuid::new_v4(),
                                                         session_id: sid_r,
                                                         trace_id: Uuid::new_v4(),
                                                         timestamp: Utc::now(),
                                                         worker_id: "system".to_string(),
                                                         event_type: "Log".to_string(),
                                                         payload: serde_json::to_string(&ifcl_core::LogPayload {
                                                             level: "INFO".to_string(),
                                                             message: format!("🔄 Replanned: Created new mission '{}'", m.name),
                                                         }).unwrap(),
                                                     }).await;
                                                }
                                            }
                                        }
                                        Err(e_h) => {
                                            let _ = bus
                                                .publish(Event {
                                                    id: Uuid::new_v4(),
                                                    session_id: mission.session_id,
                                                    trace_id: Uuid::new_v4(),
                                                    timestamp: Utc::now(),
                                                    worker_id: "system".to_string(),
                                                    event_type: "Log".to_string(),
                                                    payload: serde_json::to_string(
                                                        &ifcl_core::LogPayload {
                                                            level: "ERROR".to_string(),
                                                            message: format!(
                                                                "Planner Worker failed: {}",
                                                                e_h
                                                            ),
                                                        },
                                                    )
                                                    .unwrap(),
                                                })
                                                .await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if all_done {
                let _ = bus
                    .publish(Event {
                        id: Uuid::new_v4(),
                        session_id: sid,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: "system".to_string(),
                        event_type: "Log".to_string(),
                        payload: serde_json::to_string(&ifcl_core::LogPayload {
                            level: "SUCCESS".to_string(),
                            message: "✨ All missions completed successfully.".to_string(),
                        })
                        .unwrap(),
                    })
                    .await;
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        return Ok(());
    }

    // 3. Setup Terminal (GUI Mode)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let wizard = {
        let mut w = SetupWizard::new();
        if let Some(g) = _args.goal {
            w.goal = g;
        }
        if let Some(c) = _args.max_coins {
            w.budget_coins = c;
        }
        if let Some(ws) = _args.workspace {
            w.workspace_path = ws;
        }
        w
    };

    let available_groups = MarketplaceLoader::load_groups("marketplace/groups");
    let state = Arc::new(Mutex::new(AppState::new(available_groups, wizard)));

    let app = Arc::new(App::new(
        Arc::clone(&state),
        Arc::clone(&bus),
        Arc::clone(&store),
        Arc::clone(&orchestrator),
        Arc::clone(&learning_manager),
    ));

    // 4. Load Marketplace items (Async)
    let m_bus: Arc<dyn ifcl_core::EventBus> = Arc::clone(&bus);
    let s_c_marketplace = Arc::clone(&state);
    tokio::spawn(async move {
        let _ = std::fs::create_dir_all("marketplace/workers");
        let _ = std::fs::create_dir_all("marketplace/missions");

        for worker in MarketplaceLoader::load_workers("marketplace/workers") {
            let m_bus_w = Arc::clone(&m_bus);
            let s_c = Arc::clone(&s_c_marketplace);
            tokio::spawn(async move {
                let sid = get_sid(&s_c);
                let _ = m_bus_w
                    .publish(Event {
                        id: Uuid::new_v4(),
                        session_id: sid,
                        trace_id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        worker_id: "system".to_string(),
                        event_type: "WorkerJoined".to_string(),
                        payload: serde_json::to_string(&worker).unwrap(),
                    })
                    .await;
            });
        }

        for mission in MarketplaceLoader::load_missions("marketplace/missions") {
            let m_bus_m: Arc<dyn ifcl_core::EventBus> = Arc::clone(&m_bus);
            let s_c = Arc::clone(&s_c_marketplace);
            tokio::spawn(async move {
                let sid = get_sid(&s_c);
                let _ = m_bus_m
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
            });
        }
    });

    let app_event = Arc::clone(&app);
    let bus_sub: Arc<dyn ifcl_core::EventBus> = Arc::clone(&bus);
    tokio::spawn(async move {
        let mut sub = bus_sub.subscribe();
        while let Ok(event) = sub.recv().await {
            app_event.process_event(event).await;
        }
    });

    // 5. Background Task Runner (The Real Loop Engine)
    let bus_runner: Arc<dyn ifcl_core::EventBus> = Arc::clone(&bus);
    let state_runner = Arc::clone(&state);
    let orch_runner = Arc::clone(&orchestrator);
    let planner_runner = Arc::clone(&planner);
    tokio::spawn(async move {
        loop {
            let target: Option<(Uuid, Uuid, String)> = {
                if !is_running(&state_runner) {
                    None
                } else {
                    let s = state_runner.lock().unwrap();
                    let mut found = None;
                    for mission in &s.missions {
                        for task in &mission.tasks {
                            if task.status == TaskStatus::Pending {
                                let worker_name = task
                                    .assigned_worker
                                    .clone()
                                    .unwrap_or_else(|| "Loop-Bot".to_string());
                                found = Some((mission.id, task.id, worker_name));
                                break;
                            }
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                    found
                }
            };

            if let Some((mid, tid, worker_name)) = target {
                let bus_exec = Arc::clone(&bus_runner);
                let orch_exec = Arc::clone(&orch_runner);
                let w_lower = worker_name.to_lowercase();

                let worker: Box<dyn ifcl_core::Worker> = if w_lower.contains("gemini") || w_lower.contains("planner") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new(
                            "gemini".to_string(),
                            None,
                            vec!["--yolo".to_string(), "--allowed-tools".to_string(), "run_shell_command".to_string()],
                        )),
                    ))
                } else if w_lower.contains("claude") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new("claude".to_string(), None, vec![])),
                    ))
                } else if w_lower.contains("opencode") {
                    Box::new(AiGenericWorker::new(
                        worker_name.clone(),
                        WorkerRole::Coder,
                        Box::new(AiCliAgent::new("opencode".to_string(), None, vec![])),
                    ))
                } else {
                    Box::new(CliWorker::new(&worker_name, WorkerRole::Coder))
                };

                let result = orch_exec
                    .execute_task(bus_exec.clone(), mid, tid, worker.as_ref())
                    .await;

                match result {
                    Ok(_out) => {
                        let ok_sid = get_sid(&state_runner);
                        let _ = bus_exec
                            .publish(Event {
                                id: Uuid::new_v4(),
                                session_id: ok_sid,
                                trace_id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                worker_id: "system".to_string(),
                                event_type: "Log".to_string(),
                                payload: serde_json::to_string(&ifcl_core::LogPayload {
                                    level: "INFO".to_string(),
                                    message: format!("✅ Task Success on mission '{}'", mid),
                                })
                                .unwrap(),
                            })
                            .await;
                    }
                    Err(e) => {
                        let err_sid = get_sid(&state_runner);
                        let _ = bus_exec
                            .publish(Event {
                                id: Uuid::new_v4(),
                                session_id: err_sid,
                                trace_id: Uuid::new_v4(),
                                timestamp: Utc::now(),
                                worker_id: "system".to_string(),
                                event_type: "Log".to_string(),
                                payload: serde_json::to_string(&ifcl_core::LogPayload {
                                    level: "ERROR".to_string(),
                                    message: format!("❌ Task Execution Failed: {}", e),
                                })
                                .unwrap(),
                            })
                            .await;

                        // Retry Logic
                        match orch_exec.increment_retry_count(mid, tid).await {
                            Ok(count) => {
                                if count <= 3 {
                                    let retry_sid = get_sid(&state_runner);
                                    let _ = bus_exec
                                        .publish(Event {
                                            id: Uuid::new_v4(),
                                            session_id: retry_sid,
                                            trace_id: Uuid::new_v4(),
                                            timestamp: Utc::now(),
                                            worker_id: "system".to_string(),
                                            event_type: "Log".to_string(),
                                            payload: serde_json::to_string(
                                                &ifcl_core::LogPayload {
                                                    level: "WARN".to_string(),
                                                    message: format!(
                                                        "⚠️ Retrying Task (Attempt {}/3)",
                                                        count
                                                    ),
                                                },
                                            )
                                            .unwrap(),
                                        })
                                        .await;

                                    let _ = orch_exec
                                        .update_task_status(mid, tid, TaskStatus::Pending)
                                        .await;
                                    tokio::time::sleep(tokio::time::Duration::from_secs(
                                        2u64.pow(count),
                                    ))
                                    .await; // Linear backoff: 2s, 4s, 8s
                                } else {
                                    let fatal_sid = get_sid(&state_runner);
                                    let _ = bus_exec.publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: fatal_sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: "system".to_string(),
                                        event_type: "Log".to_string(),
                                        payload: serde_json::to_string(&ifcl_core::LogPayload {
                                            level: "ERROR".to_string(),
                                            message: format!("❌ Retries Exhausted for task '{}'. Escalating to Planner...", tid),
                                        }).unwrap(),
                                    }).await;

                                    let missions =
                                        orch_exec.get_missions().await.unwrap_or_default();
                                    if let Some(mission) = missions.iter().find(|m| m.id == mid) {
                                        let goal = get_goal(&state_runner);

                                        // Planner Worker Pattern

                                        // Planner Worker Pattern
                                        // 1. Create a logical "Replan Task"
                                        let replan_context = ifcl_core::ReplanContext {
                                            goal: goal.clone(),
                                            mission: mission.clone(),
                                            failed_task_id: tid,
                                        };
                                        let context_json =
                                            serde_json::to_string(&replan_context).unwrap();

                                        let replan_task = ifcl_core::Task {
                                            id: Uuid::new_v4(),
                                            name: format!("Replan Task {}", tid),
                                            description: context_json,
                                            status: TaskStatus::Pending,
                                            assigned_worker: Some("Planner".to_string()),
                                            retry_count: 0,
                                        };

                                        // 2. Instantiate Planner Worker (Ephemeral for now, or use roster)
                                        let planner_worker =
                                            ifcl_core::PlannerWorker::new(planner_runner.clone());

                                        let _ = bus_exec
                                            .publish(Event {
                                                id: Uuid::new_v4(),
                                                session_id: mission.session_id,
                                                trace_id: Uuid::new_v4(),
                                                timestamp: Utc::now(),
                                                worker_id: "system".to_string(),
                                                event_type: "Log".to_string(),
                                                payload: serde_json::to_string(
                                                    &ifcl_core::LogPayload {
                                                        level: "INFO".to_string(),
                                                        message:
                                                            "🧠 Delegating to Planner Worker..."
                                                                .to_string(),
                                                    },
                                                )
                                                .unwrap(),
                                            })
                                            .await;

                                        // 3. Execute
                                        match planner_worker
                                            .execute(
                                                bus_exec.clone(),
                                                &replan_task,
                                                ".",
                                                mission.session_id,
                                            )
                                            .await
                                        {
                                            Ok(output) => {
                                                if let Ok(new_missions) =
                                                    serde_json::from_str::<Vec<ifcl_core::Mission>>(
                                                        &output,
                                                    )
                                                {
                                                    let sid = mission.session_id;
                                                    let ws = mission.workspace_path.clone();

                                                    for mut m in new_missions {
                                                        m.session_id = sid;
                                                        m.workspace_path = ws.clone();
                                                        let _ =
                                                            orch_exec.add_mission(m.clone()).await;

                                                        // Log New Mission
                                                        let new_mission_sid =
                                                            get_sid(&state_runner);
                                                        let _ = bus_exec.publish(Event {
                                                             id: Uuid::new_v4(),
                                                             session_id: new_mission_sid,
                                                             trace_id: Uuid::new_v4(),
                                                             timestamp: Utc::now(),
                                                             worker_id: "system".to_string(),
                                                             event_type: "Log".to_string(),
                                                             payload: serde_json::to_string(&ifcl_core::LogPayload {
                                                                 level: "INFO".to_string(),
                                                                 message: format!("🔄 Replanned: Created new mission '{}'", m.name),
                                                             }).unwrap(),
                                                         }).await;
                                                    }
                                                }
                                            }
                                            Err(e_inner) => {
                                                let err_mission_sid = mission.session_id;
                                                let _ = bus_exec
                                                    .publish(Event {
                                                        id: Uuid::new_v4(),
                                                        session_id: err_mission_sid,
                                                        trace_id: Uuid::new_v4(),
                                                        timestamp: Utc::now(),
                                                        worker_id: "system".to_string(),
                                                        event_type: "Log".to_string(),
                                                        payload: serde_json::to_string(
                                                            &ifcl_core::LogPayload {
                                                                level: "ERROR".to_string(),
                                                                message: format!(
                                                                    "Planner Worker failed: {}",
                                                                    e_inner
                                                                ),
                                                            },
                                                        )
                                                        .unwrap(),
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e_retry) => {
                                let retry_err_sid = get_sid(&state_runner);
                                let _ = bus_exec
                                    .publish(Event {
                                        id: Uuid::new_v4(),
                                        session_id: retry_err_sid,
                                        trace_id: Uuid::new_v4(),
                                        timestamp: Utc::now(),
                                        worker_id: "system".to_string(),
                                        event_type: "Log".to_string(),
                                        payload: serde_json::to_string(&ifcl_core::LogPayload {
                                            level: "ERROR".to_string(),
                                            message: format!(
                                                "Failed to update retry count: {}",
                                                e_retry
                                            ),
                                        })
                                        .unwrap(),
                                    })
                                    .await;
                            }
                        }
                    }
                }

                let all_done = are_missions_done(&state_runner);

                if all_done {
                    let done_sid = get_sid(&state_runner);
                    let _ = bus_exec
                        .publish(Event {
                            id: Uuid::new_v4(),
                            session_id: done_sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: "system".to_string(),
                            event_type: "Log".to_string(),
                            payload: serde_json::to_string(&ifcl_core::LogPayload {
                                level: "SUCCESS".to_string(),
                                message: "✨ All missions completed successfully.".to_string(),
                            })
                            .unwrap(),
                        })
                        .await;

                    let _ = bus_exec
                        .publish(Event {
                            id: Uuid::new_v4(),
                            session_id: done_sid,
                            trace_id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            worker_id: "system".to_string(),
                            event_type: "GoalCompleted".to_string(),
                            payload: "All missions completed".to_string(),
                        })
                        .await;
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
    });

    // 6. TUI Rendering Loop
    loop {
        terminal.draw(|f| {
            if let Ok(mut s) = state.lock() {
                ui::draw(f, &mut s);
            }
        })?;

        if let Ok(mut s) = state.lock() {
            s.frame_count = s.frame_count.wrapping_add(1);
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                if app.handle_input(key.code).await {
                    break;
                }
            }
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::style::ResetColor
    )?;
    terminal.show_cursor()?;

    Ok(())
}
