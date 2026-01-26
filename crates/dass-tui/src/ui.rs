use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::{App, PipelineStatus};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main Content
            Constraint::Length(3), // Footer
        ])
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_dashboard(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, _app: &App, area: Rect) {
    let text = vec![
        Line::from(" INFINITE LOOP // SOFTWARE FACTORY "),
        Line::from(" Deterministic Autonomous Software Synthesis "),
    ];
    let block = Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(text).block(block).style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(paragraph, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33), // Product (Reqs)
            Constraint::Percentage(33), // Design (Spec)
            Constraint::Percentage(33), // Plan (Steps)
        ])
        .split(area);

    draw_product_pane(f, app, chunks[0]);
    draw_design_pane(f, app, chunks[1]);
    draw_plan_pane(f, app, chunks[2]);
}

fn draw_product_pane(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.requirements.iter()
        .map(|r| ListItem::new(format!("• {}", r.user_story)))
        .collect();

    let block = Block::default()
        .title(" 1. PRODUCT (Requirements) ")
        .borders(Borders::ALL)
        .style(if !app.requirements.is_empty() { Style::default().fg(Color::Green) } else { Style::default() });

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_design_pane(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(spec) = &app.current_spec {
        format!("ID: {}\n\nUI: {} chars\nLogic: {} chars", spec.id, spec.ui_spec.len(), spec.logic_spec.len())
    } else {
        "Waiting for Architect...".to_string()
    };
    
    let block = Block::default()
        .title(" 2. DESIGN (Spec) ")
        .borders(Borders::ALL)
        .style(if app.current_spec.is_some() { Style::default().fg(Color::Green) } else { Style::default() });

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}

fn draw_plan_pane(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(plan) = &app.current_plan {
        format!("Feature: {}\nSteps: {}", plan.feature_id, plan.steps.len())
    } else {
        "Waiting for Engineer...".to_string()
    };

    let block = Block::default()
        .title(" 3. CONSTRUCTION (Plan) ")
        .borders(Borders::ALL)
        .style(if app.current_plan.is_some() { Style::default().fg(Color::Green) } else { Style::default() });

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status_style = match app.pipeline_status {
        PipelineStatus::Idle => Style::default(),
        PipelineStatus::Error(_) => Style::default().fg(Color::Red),
        _ => Style::default().fg(Color::Yellow),
    };
    
    let info = format!("STATUS: {:?} | Press 'n' to New Feature | 'q' to Quit", app.pipeline_status);
    let block = Block::default().borders(Borders::ALL).style(status_style);
    let p = Paragraph::new(info).block(block);
    f.render_widget(p, area);
}
