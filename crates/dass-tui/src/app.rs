use dass_engine::product::requirement::Requirement;
use dass_engine::spec::feature_spec::FeatureSpec;
use dass_engine::plan::action::ImplementationPlan;
use ratatui::widgets::ListState;

#[derive(Debug, Default, PartialEq)]
pub enum PipelineStatus {
    #[default]
    Idle,
    Planning,
    Designing,
    Implementing,
    Verifying,
    Error(String),
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub pipeline_status: PipelineStatus,
    
    // Artifacts
    pub requirements: Vec<Requirement>,
    pub current_spec: Option<FeatureSpec>,
    pub current_plan: Option<ImplementationPlan>,

    // UI State
    pub req_list_state: ListState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            pipeline_status: PipelineStatus::Idle,
            requirements: Vec::new(),
            current_spec: None,
            current_plan: None,
            req_list_state: ListState::default(),
        }
    }
}

impl App {
    pub fn on_tick(&mut self) {
        // Here we would poll async channels for updates from Agents
    }

    pub fn on_key(&mut self, key: char) {
        match key {
            'q' => self.should_quit = true,
            'n' => self.start_new_feature(),
            _ => {}
        }
    }

    fn start_new_feature(&mut self) {
        // Trigger the pipeline
        self.pipeline_status = PipelineStatus::Planning;
        // In a real async app, this would spawn a tokio task
    }
}
