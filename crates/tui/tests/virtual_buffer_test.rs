use tui::app::App;
use tui::state::AppState;
use tui::ui;
use ifcl_core::{
    learning::BasicLearningManager,
    BasicOrchestrator,
    InMemoryEventBus,
    SetupWizard,
    SqliteEventStore,
};
use ratatui::{backend::TestBackend, Terminal};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_main_menu_render() {
    // 1. Setup Internals
    // Mock the dependencies
    let bus: Arc<dyn ifcl_core::EventBus> = Arc::new(InMemoryEventBus::new(100));
    // Implementation Detail: SqliteEventStore specific construction might be tricky in test if it requires a file.
    // For TUI rendering test, we might not strictly need a working DB if we don't process events.
    // But App::new takes it.
    // We can use a memory-based sqlite if possible, or just a dummy/temp file.
    // SqliteEventStore::new("sqlite::memory:") might work if supported by the struct.
    // Let's assume for now we can pass a dummy connection string or handle error if it fails (but new() is async returning Result).
    // Actually, let's try to minimal mock.
    
    // NOTE: If SqliteEventStore requires real DB file, using ":memory:" is best.
    let store_result = SqliteEventStore::new("sqlite::memory:").await;
    let store = if let Ok(s) = store_result {
        Arc::new(s)
    } else {
        // Fallback or panic
        panic!("Failed to create in-memory sqlite store for testing");
    };

    let learning_manager = Arc::new(BasicLearningManager::new());
    let orchestrator = Arc::new(BasicOrchestrator::new());
    let wizard = SetupWizard::new();
    let available_groups = vec![];

    let state = Arc::new(Mutex::new(AppState::new(available_groups, wizard)));
    
    let _app = App::new(
        Arc::clone(&state),
        Arc::clone(&bus),
        store,
        orchestrator,
        learning_manager,
    );

    // 2. Setup Test Backend
    let backend = TestBackend::new(100, 50);
    let mut terminal = Terminal::new(backend).unwrap();

    // 3. Render
    terminal.draw(|f| {
        let mut s = state.lock().unwrap();
        ui::draw(f, &mut s);
    }).unwrap();

    // 4. Assertions
    let buffer = terminal.backend().buffer();
    
    // Check for "AUTONOMOUS CODING LOOP" title (Spaced out in ASCII art)
    let mut found_title = false;
    let target = "A U T O N O M O U S";
    
    for y in 0..buffer.area.height {
        let line_text: String = (0..buffer.area.width)
            .map(|x| buffer.get(x, y).symbol().to_string())
            .collect();
        if line_text.contains(target) {
            found_title = true;
            break;
        }
        
        let mut line_str = String::new();
        for x in 0..buffer.area.width {
             line_str.push_str(buffer.get(x, y).symbol());
        }
        if line_str.contains(target) {
            found_title = true;
            break;
        }
    }

    assert!(found_title, "Main menu title (AUTONOMOUS CODING LOOP) not found in rendered buffer");
    
    // Check for "MAIN MENU"
    let mut found_menu = false;
    let menu_target = "MAIN MENU";
     for y in 0..buffer.area.height {
        let mut line_str = String::new();
        for x in 0..buffer.area.width {
             line_str.push_str(buffer.get(x, y).symbol());
        }
        if line_str.contains(menu_target) {
            found_menu = true;
            break;
        }
    }
     assert!(found_menu, "Main menu block title not found");
}
