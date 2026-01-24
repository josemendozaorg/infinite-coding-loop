use crate::relationship::NodeType;
use crate::state::{AppState, FocusMode};
use chrono::Utc;
use ifcl_core::{
    AiProvider, AppMode, LogPayload, LoopStatus, MenuAction, TaskStatus, ThoughtPayload,
    WizardStep, WorkerRole,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

#[allow(clippy::manual_is_multiple_of)]
pub fn draw(f: &mut Frame, s: &mut AppState) {
    let mode = s.mode.clone();

    match mode {
        AppMode::MainMenu => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(40),
                        Constraint::Percentage(30),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let title = Paragraph::new(
                r#"
  _____ _   _  _____ _____ _   _ _____ _____ _____ 
 |_   _| \ | ||  ___|_   _| \ | |_   _|_   _|  ___|
   | | |  \| || |_    | | |  \| | | |   | | | |__  
   | | | . ` ||  _|   | | | . ` | | |   | | |  __| 
  _| |_| |\  || |    _| |_| |\  |_| |_  | | | |___ 
  \___/\_| \_/\_|    \___/\_| \_/\___/  \_/ \____/ 
                                                   
   A U T O N O M O U S   C O D I N G   L O O P
                        "#,
            )
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
            f.render_widget(title, chunks[0]);

            let mut menu_items = Vec::new();
            for (i, item) in s.menu.items.iter().enumerate() {
                let label = match item {
                    MenuAction::NewGame => "NEW GAME",
                    MenuAction::LoadGame => "LOAD GAME",
                    MenuAction::OpenMarketplace => "MARKETPLACE",
                    MenuAction::Quit => "QUIT",
                };
                let style = if i == s.menu.selected_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                menu_items.push(ListItem::new(format!("  {}  ", label)).style(style));
            }

            let menu_list = List::new(menu_items)
                .block(Block::default().borders(Borders::ALL).title(" MAIN MENU "))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            let menu_area = centered_rect(40, 50, chunks[1]);
            f.render_widget(menu_list, menu_area);
        }
        AppMode::SessionPicker => {
            let mut session_items = Vec::new();
            for (i, sid) in s.available_sessions.iter().enumerate() {
                let style = if i == s.selected_session_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                session_items
                    .push(ListItem::new(format!("  Loop Session: {}  ", sid)).style(style));
            }
            if session_items.is_empty() {
                session_items.push(
                    ListItem::new("  No sessions found. Press ESC to Go Back.  ")
                        .style(Style::default().fg(Color::DarkGray)),
                );
            }

            let session_list = List::new(session_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" LOAD PREVIOUS LOOP "),
                )
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            let area = centered_rect(60, 50, f.size());
            f.render_widget(session_list, area);
        }
        AppMode::Setup => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let step = s.wizard.current_step.clone();
            let goal = s.wizard.goal.clone();
            let stack = s.wizard.stack.clone();
            let workspace = s.wizard.workspace_path.clone();
            let provider = s.wizard.provider.clone();
            let team = s.wizard.team_size;
            let budget = s.wizard.budget_coins;
            let avail_groups = s.available_groups.clone();
            let sel_grp_idx = s.wizard.selected_group_index;

            let step_text = match step {
                WizardStep::Goal => "Step 1/7: Define Objective",
                WizardStep::Stack => "Step 2/7: Technology Stack",
                WizardStep::Workspace => "Step 3/7: Project Workspace",
                WizardStep::Provider => "Step 4/7: AI Intelligence",
                WizardStep::Team => "Step 5/7: Squad Assembly",
                WizardStep::Budget => "Step 6/7: Resource Credits",
                WizardStep::Summary => "Step 7/7: Final Review",
            };

            f.render_widget(
                Paragraph::new(step_text)
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" NEW LOOP SETUP "),
                    ),
                chunks[0],
            );

            let content = match step {
                WizardStep::Goal => format!("Define your mission goal:\n\n> {}", goal),
                WizardStep::Stack => format!("Selected Technology:\n\n [ {} ]", stack),
                WizardStep::Workspace => format!("Project output directory:\n (Where files will be built)\n\n> {}", workspace),
                WizardStep::Provider => {
                    let mut s = String::from("Select Primary Architect (AI Provider):\n(Use UP/DOWN keys)\n\n");
                    let providers = vec![
                        (AiProvider::Gemini, "Gemini (Google) - Uses 'gemini' CLI"),
                        (AiProvider::Claude, "Claude (Anthropic) - Uses 'claude' CLI"),
                        (AiProvider::OpenCode, "OpenCode (OpenAI) - Uses 'opencode' CLI"),
                        (AiProvider::Basic, "Basic (Rule-based) - No AI, just templates"),
                    ];
                    for (p, label) in providers {
                        let checkbox = if provider == p { "[x]" } else { "[ ]" };
                        s.push_str(&format!(" {} {}\n", checkbox, label));
                    }
                    s
                },
                WizardStep::Team => {
                    let mut str_content = String::from("Select a Worker Team:\n\n");
                    for (i, grp) in avail_groups.iter().enumerate() {
                        let cursor = if i == sel_grp_idx { ">" } else { " " };
                        let checkbox = if i == sel_grp_idx { "[x]" } else { "[ ]" };
                        str_content.push_str(&format!("{} {} {}\n", cursor, checkbox, grp.name));
                    }
                    if avail_groups.is_empty() {
                        str_content.push_str("  (No teams found in marketplace/groups)\n");
                    } else if let Some(grp) = avail_groups.get(sel_grp_idx) {
                        str_content.push_str(&format!("\nDescription: {}\n", grp.description));
                        str_content.push_str("Members:\n");
                        for w in &grp.workers {
                            str_content.push_str(&format!(" - {} ({:?})\n", w.name, w.role));
                        }
                    }
                    str_content
                },
                WizardStep::Budget => format!("Initial Credit Allotment:\n\n [ {} ] Coins", budget),
                WizardStep::Summary => format!(
                    "Mission: {}\nStack: {}\nWorkspace: {}\nArchitect: {:?}\nTeam: {} Workers\nBudget: {} Coins\n\n[ PRESS ENTER TO START ]",
                    goal, stack, workspace, provider, team, budget
                ),
            };

            f.render_widget(
                Paragraph::new(content).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" CONFIGURATION "),
                ),
                chunks[1],
            );

            f.render_widget(
                Paragraph::new(" [ENTER] Next | [BACKSPACE] Prev | [ESC] Cancel ")
                    .style(Style::default().fg(Color::DarkGray)),
                chunks[2],
            );
        }
        AppMode::Running => {
            let focus_mode = s.focus_mode;

            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3), // Header
                        Constraint::Length(3), // Progress Bar
                        Constraint::Min(0),    // Main content
                        Constraint::Length(1), // Activity Bar
                        Constraint::Length(1), // Debug/Footer
                    ]
                    .as_ref(),
                )
                .split(f.size());

            // --- Header ---
            let header_color = if s.is_intervening {
                Color::Magenta
            } else {
                match s.status {
                    LoopStatus::Running => Color::Cyan,
                    LoopStatus::Paused => Color::Yellow,
                }
            };
            let ctx_info = if let Some((tokens, pruned)) = s.managed_context_stats {
                format!(" | CTX: {}tk ({}p)", tokens, pruned)
            } else {
                String::new()
            };

            let pulse_indicator = if s.status == LoopStatus::Running {
                const SPINNER_FRAMES: [&str; 10] =
                    ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                SPINNER_FRAMES[(s.frame_count as usize) % SPINNER_FRAMES.len()]
            } else {
                "⏸"
            };
            let last_activity_secs = (Utc::now() - s.last_event_at).num_seconds();
            let activity_timer = format!(" ({}s ago)", last_activity_secs);

            let header = Paragraph::new(format!(
                " {} OBJ: {:<20} | XP: {} | $: {} | ST: {:?}{}{}",
                pulse_indicator,
                if s.wizard.goal.len() > 20 {
                    format!("{}...", &s.wizard.goal[..17])
                } else {
                    s.wizard.goal.clone()
                },
                s.bank.xp,
                s.bank.coins,
                s.status,
                ctx_info,
                activity_timer
            ))
            .style(
                Style::default()
                    .fg(header_color)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .title(" INFINITE CODING LOOP [v0.1.0] ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(header, main_layout[0]);

            // --- Progress Bar ---
            if let Some(stats) = &s.progress_stats {
                let gauge = Gauge::default()
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" MISSION PROGRESS ")
                            .border_style(Style::default().fg(Color::DarkGray)),
                    )
                    .gauge_style(
                        Style::default()
                            .fg(if stats.is_stalled {
                                Color::Red
                            } else {
                                Color::Cyan
                            })
                            .bg(Color::Black),
                    )
                    .percent(stats.progress_percentage as u16)
                    .label(format!(
                        "{:.1}% ({} / {}){}",
                        stats.progress_percentage,
                        stats.completed_tasks,
                        stats.total_tasks,
                        if stats.is_stalled { " [STALLED!]" } else { "" }
                    ));
                f.render_widget(gauge, main_layout[1]);
            } else {
                f.render_widget(
                    Block::default()
                        .title(" MISSION PROGRESS ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                    main_layout[1],
                );
            }

            // Middle Layout
            let middle_chunks = if focus_mode == FocusMode::None {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(12),
                            Constraint::Percentage(18),
                            Constraint::Percentage(18),
                            Constraint::Percentage(18),
                            Constraint::Percentage(18),
                            Constraint::Percentage(18),
                        ]
                        .as_ref(),
                    )
                    .split(main_layout[2])
                    .to_vec()
            } else {
                vec![main_layout[2]]
            };

            // 1. ROSTER
            if focus_mode == FocusMode::None || focus_mode == FocusMode::Roster {
                let area = middle_chunks[0];
                let active_workers: std::collections::HashSet<String> = s
                    .missions
                    .iter()
                    .flat_map(|m| m.tasks.iter())
                    .filter(|t| t.status == TaskStatus::Running)
                    .filter_map(|t| t.assigned_worker.clone())
                    .collect();

                const WORKER_SPINNER: [&str; 4] = ["⣾", "⣽", "⣻", "⢿"];
                let worker_spinner =
                    WORKER_SPINNER[(s.frame_count as usize) % WORKER_SPINNER.len()];

                let worker_items: Vec<_> = s
                    .workers
                    .iter()
                    .map(|w| {
                        let symbol = match w.role {
                            WorkerRole::Git => "󰊢",
                            WorkerRole::Coder => "󰅩",
                            WorkerRole::Architect => "󰉪",
                            _ => "󰚩",
                        };
                        let is_active = active_workers.contains(&w.name);
                        let activity_indicator = if is_active {
                            format!(" {}", worker_spinner)
                        } else {
                            String::new()
                        };
                        let style = if is_active {
                            let blink_phase = ((s.frame_count / 5) % 2) == 0;
                            if blink_phase {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::LightCyan)
                                    .add_modifier(Modifier::BOLD)
                            }
                        } else {
                            Style::default().fg(Color::Yellow)
                        };
                        ListItem::new(format!(" {} {}{}", symbol, w.name, activity_indicator))
                            .style(style)
                    })
                    .collect();
                f.render_widget(
                    List::new(worker_items).block(
                        Block::default()
                            .title(" BARRACKS [1] ")
                            .borders(Borders::ALL)
                            .border_style(if focus_mode == FocusMode::Roster {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            }),
                    ),
                    area,
                );
            }

            // 2. MISSION CONTROL
            if focus_mode == FocusMode::None || focus_mode == FocusMode::MissionControl {
                let area = middle_chunks[if focus_mode == FocusMode::MissionControl {
                    0
                } else {
                    1
                }];
                let mut rows = Vec::new();
                const TASK_SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];
                let spinner_frame = TASK_SPINNER[(s.frame_count as usize) % TASK_SPINNER.len()];
                let blink_phase = ((s.frame_count / 5) % 2) == 0;

                for mission in &s.missions {
                    for task in &mission.tasks {
                        let status_text = match task.status {
                            TaskStatus::Running => format!("{} EXECUTING...", spinner_frame),
                            _ => format!("{:?}", task.status),
                        };
                        let status_style = match task.status {
                            TaskStatus::Pending => Style::default().fg(Color::DarkGray),
                            TaskStatus::Running => {
                                if blink_phase {
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .bg(Color::Black)
                                        .add_modifier(Modifier::BOLD)
                                }
                            }
                            TaskStatus::Success => Style::default().fg(Color::Green),
                            TaskStatus::Failure => Style::default().fg(Color::Red),
                        };
                        let task_name_style = if task.status == TaskStatus::Running {
                            if blink_phase {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .bg(Color::DarkGray)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            }
                        } else {
                            Style::default().add_modifier(Modifier::BOLD)
                        };
                        rows.push(Row::new(vec![
                            Cell::from(mission.name.clone())
                                .style(Style::default().fg(Color::DarkGray)),
                            Cell::from(task.name.clone()).style(task_name_style),
                            Cell::from(status_text).style(status_style),
                            Cell::from(task.assigned_worker.clone().unwrap_or_default())
                                .style(Style::default().fg(Color::Yellow)),
                        ]));
                    }
                }
                let widths = [
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ];
                f.render_widget(
                    Table::new(rows, widths)
                        .header(
                            Row::new(vec!["MISSION", "TASK", "STATUS", "WORKER"]).style(
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        )
                        .block(
                            Block::default()
                                .title(" MISSION CONTROL [2] ")
                                .borders(Borders::ALL)
                                .border_style(if focus_mode == FocusMode::MissionControl {
                                    Style::default().fg(Color::Cyan)
                                } else {
                                    Style::default().fg(Color::DarkGray)
                                }),
                        ),
                    area,
                );
            }

            // 3. MENTAL MAP
            if focus_mode == FocusMode::None || focus_mode == FocusMode::MentalMap {
                let area = middle_chunks[if focus_mode == FocusMode::MentalMap {
                    0
                } else {
                    2
                }];
                let mut map_items = Vec::new();
                use petgraph::Direction;
                let roots: Vec<_> = s
                    .mental_map
                    .graph
                    .node_indices()
                    .filter(|&idx| {
                        s.mental_map
                            .graph
                            .neighbors_directed(idx, Direction::Incoming)
                            .count()
                            == 0
                    })
                    .collect();
                let count = roots.len();
                let mut visited = std::collections::HashSet::new();

                for (i, node_idx) in roots.into_iter().enumerate() {
                    render_node_recursive(
                        &s.mental_map.graph,
                        node_idx,
                        0,
                        "",
                        i == count - 1,
                        &mut map_items,
                        &mut visited,
                    );
                }
                f.render_widget(
                    List::new(map_items).block(
                        Block::default()
                            .title(" MENTAL MAP [3] ")
                            .borders(Borders::ALL)
                            .border_style(if focus_mode == FocusMode::MentalMap {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            }),
                    ),
                    area,
                );
            }

            // 4. EVENT FEED
            if focus_mode == FocusMode::None || focus_mode == FocusMode::Feed {
                let area = middle_chunks[if focus_mode == FocusMode::Feed { 0 } else { 3 }];
                let feed_items: Vec<ListItem> = s
                    .events
                    .iter()
                    .map(|e| {
                        let color = match e.event_type.as_str() {
                            "LoopStarted" => Color::Green,
                            "WorkerJoined" => Color::Blue,
                            "AiResponse" => Color::Yellow,
                            "RewardEarned" => Color::Green,
                            "LoopStatusChanged" | "Log" => Color::Yellow,
                            "ManualCommandInjected" => Color::Magenta,
                            "WorkerError" => Color::Red,
                            "WorkerThought" => Color::Cyan,
                            _ => Color::White,
                        };
                        let content = if e.event_type == "AiResponse" {
                            format!(" > AI: {}", e.payload.chars().take(40).collect::<String>())
                        } else if e.event_type == "RewardEarned" {
                            format!(" + REWARD: {}", e.payload)
                        } else if e.event_type == "LoopStatusChanged" {
                            format!(" # STATUS: {}", e.payload)
                        } else if e.event_type == "Log" {
                            if let Ok(p) = serde_json::from_str::<LogPayload>(&e.payload) {
                                format!(" * {}: {}", p.level, p.message)
                            } else {
                                format!(" * LOG: {}", e.payload)
                            }
                        } else if e.event_type == "WorkerThought" {
                            if let Ok(p) = serde_json::from_str::<ThoughtPayload>(&e.payload) {
                                format!(
                                    " ? [{:.1}%] {}",
                                    p.confidence * 100.0,
                                    p.reasoning.last().unwrap_or(&"Thinking...".to_string())
                                )
                            } else {
                                format!(" ? THINKING: {}", e.payload)
                            }
                        } else if e.event_type == "ManualCommandInjected" {
                            format!(" @ GOD: {}", e.payload)
                        } else if e.event_type == "WorkerError" {
                            format!(" ! ERR: {}", e.payload)
                        } else {
                            format!(" {:<8} | {}", e.timestamp.format("%H:%M:%S"), e.event_type)
                        };
                        ListItem::new(content).style(Style::default().fg(color))
                    })
                    .collect();

                let feed_list = List::new(feed_items)
                    .block(
                        Block::default()
                            .title(" FEED [4] ")
                            .borders(Borders::ALL)
                            .border_style(if focus_mode == FocusMode::Feed {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            }),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                if s.selected_event_index.is_none() && !s.events.is_empty() {
                    let last_idx = s.events.len().saturating_sub(1);
                    s.feed_state.select(Some(last_idx));
                }

                f.render_stateful_widget(feed_list, area, &mut s.feed_state);
            }

            // 5. AI TERMINAL
            if focus_mode == FocusMode::None || focus_mode == FocusMode::Terminal {
                let area = middle_chunks[if focus_mode == FocusMode::Terminal {
                    0
                } else {
                    4
                }];
                let mut ai_content = String::new();
                for output in &s.ai_outputs {
                    ai_content.push_str(&format!(
                        "[{}] {}>\n{}\n",
                        output.timestamp.format("%H:%M:%S"),
                        output.worker_id,
                        output.content
                    ));
                }
                if ai_content.is_empty() {
                    ai_content = "Waiting for AI response...".to_string();
                }

                f.render_widget(
                    Paragraph::new(ai_content).wrap(Wrap { trim: true }).block(
                        Block::default()
                            .title(" AI TERMINAL [5] ")
                            .borders(Borders::ALL)
                            .border_style(if focus_mode == FocusMode::Terminal {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            }),
                    ),
                    area,
                );
            }

            // 6. INSIGHTS & OPTIMIZATIONS
            if focus_mode == FocusMode::None || focus_mode == FocusMode::Learnings {
                let area = middle_chunks[if focus_mode == FocusMode::Learnings {
                    0
                } else {
                    5
                }];
                let mut learning_items = Vec::new();
                for insight in &s.insights {
                    learning_items.push(
                        ListItem::new(format!(" 󰋗 {}", insight.description))
                            .style(Style::default().fg(Color::Cyan)),
                    );
                }
                for opt in &s.optimizations {
                    learning_items.push(
                        ListItem::new(format!(" 󰒓 [{}]: {}", opt.target_component, opt.suggestion))
                            .style(Style::default().fg(Color::Magenta)),
                    );
                }
                if learning_items.is_empty() {
                    let recorded = s.recorded_missions.len();
                    let total = s.missions.len();
                    learning_items.push(
                        ListItem::new(format!(
                            " Gathering experience ({}/{} missions)...",
                            recorded, total
                        ))
                        .style(Style::default().fg(Color::DarkGray)),
                    );
                }
                f.render_widget(
                    List::new(learning_items).block(
                        Block::default()
                            .title(" LEARNINGS [6] ")
                            .borders(Borders::ALL)
                            .border_style(if focus_mode == FocusMode::Learnings {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            }),
                    ),
                    area,
                );
            }

            // --- Activity Bar ---
            let has_running_task = s
                .missions
                .iter()
                .flat_map(|m| m.tasks.iter())
                .any(|t| t.status == TaskStatus::Running);
            let current_task_name = s
                .missions
                .iter()
                .flat_map(|m| m.tasks.iter())
                .find(|t| t.status == TaskStatus::Running)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| "Idle".to_string());

            let activity_prefix = if has_running_task {
                const ACTIVITY_FRAMES: [&str; 4] = ["●", "◔", "◑", "◕"];
                format!(
                    " {} ",
                    ACTIVITY_FRAMES[(s.frame_count as usize) % ACTIVITY_FRAMES.len()]
                )
            } else {
                " ○ ".to_string()
            };

            let last_activity_text = format!(
                "{}[ACTIVITY] Task: {} | Last Event: {} ({}s ago)",
                activity_prefix, current_task_name, s.last_event_type, last_activity_secs
            );

            let activity_color = if has_running_task {
                let blink_phase = ((s.frame_count / 5) % 2) == 0;
                if blink_phase {
                    Color::Cyan
                } else {
                    Color::LightCyan
                }
            } else if last_activity_secs > 30 {
                Color::Red
            } else if last_activity_secs > 10 {
                Color::Yellow
            } else {
                Color::Green
            };

            let activity_bar = Paragraph::new(last_activity_text)
                .style(
                    Style::default()
                        .fg(activity_color)
                        .add_modifier(if has_running_task {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                )
                .alignment(Alignment::Left);
            f.render_widget(activity_bar, main_layout[3]);

            let footer_text = if s.is_intervening {
                " [ESC] Cancel | [ENTER] Send | MODE: INTERVENTION"
            } else if s.show_event_details {
                " [ESC] Close | MODE: DETAIL VIEW"
            } else {
                " [Q] Quit | [SPACE] Pause | [I] Intervene | [1-6] Focus | [J/K] Feed Scroll | [ENTER] Event Details"
            };
            let footer = Paragraph::new(footer_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            f.render_widget(footer, main_layout[4]);

            if s.show_event_details {
                if let Some(idx) = s.selected_event_index {
                    if let Some(event) = s.events.get(idx) {
                        let area = centered_rect(80, 80, f.size());
                        f.render_widget(Clear, area);
                        let content = format!(
                            "ID: {}\nType: {}\nTimestamp: {}\nWorker: {}\nPayload:\n{}",
                            event.id,
                            event.event_type,
                            event.timestamp,
                            event.worker_id,
                            serde_json::to_string_pretty(
                                &serde_json::from_str::<serde_json::Value>(&event.payload)
                                    .unwrap_or(serde_json::json!({"raw": event.payload}))
                            )
                            .unwrap()
                        );
                        f.render_widget(
                            Paragraph::new(content)
                                .block(
                                    Block::default()
                                        .title(" EVENT DETAILS ")
                                        .borders(Borders::ALL)
                                        .border_style(Style::default().fg(Color::Cyan)),
                                )
                                .wrap(Wrap { trim: false }),
                            area,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

pub fn render_node_recursive(
    graph: &petgraph::graph::DiGraph<NodeType, ()>,
    node_idx: petgraph::graph::NodeIndex,
    depth: usize,
    prefix: &str,
    is_last: bool,
    items: &mut Vec<ListItem>,
    visited: &mut std::collections::HashSet<petgraph::graph::NodeIndex>,
) {
    if visited.contains(&node_idx) {
        return;
    }
    visited.insert(node_idx);

    let node = &graph[node_idx];
    let connector = if depth == 0 {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };

    let style = match node {
        NodeType::Mission(_) => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        NodeType::Task(_) => Style::default().fg(Color::White),
        NodeType::Worker(_) => Style::default().fg(Color::Yellow),
    };

    let name = match node {
        NodeType::Mission(n) => n,
        NodeType::Task(n) => n,
        NodeType::Worker(n) => n,
    };

    items.push(ListItem::new(format!("{}{}{}", prefix, connector, name)).style(style));

    let new_prefix = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    let children: Vec<_> = graph
        .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
        .collect();
    let count = children.len();
    for (i, child_idx) in children.into_iter().enumerate() {
        render_node_recursive(
            graph,
            child_idx,
            depth + 1,
            &new_prefix,
            i == count - 1,
            items,
            visited,
        );
    }
}
