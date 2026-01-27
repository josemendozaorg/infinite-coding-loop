use crate::agents::{
    architect::Architect, cli_client::AiCliClient, planner::Planner,
    product_manager::ProductManager,
};
use crate::domain::application::SoftwareApplication;
use crate::domain::db::StateStore;
use crate::domain::primitive::Primitive;
use crate::plan::action::ImplementationPlan;
use crate::product::requirement::Requirement;
use crate::spec::feature_spec::FeatureSpec;
use anyhow::Result;

pub struct Orchestrator<C: AiCliClient + Clone> {
    pm: ProductManager<C>,
    architect: Architect<C>,
    planner: Planner<C>,
    pub app: SoftwareApplication,
    store: StateStore,
    work_dir: std::path::PathBuf,
}

impl<C: AiCliClient + Clone> Orchestrator<C> {
    pub async fn list_available_apps() -> Result<Vec<(String, String, Option<String>)>> {
        let store = StateStore::new(".dass.db").await?;
        store.list_applications().await
    }

    pub async fn get_app_work_dir(app_id: &str) -> Result<Option<String>> {
        let store = StateStore::new(".dass.db").await?;
        match store.load_application(app_id).await {
            Ok(app) => Ok(app.work_dir),
            Err(_) => Ok(None),
        }
    }

    pub async fn new(
        client: C,
        app_id: String,
        app_name: String,
        work_dir: std::path::PathBuf,
    ) -> Result<Self> {
        let store = StateStore::new(".dass.db").await?;

        let mut app = store
            .load_application(&app_id)
            .await
            .unwrap_or_else(|_| SoftwareApplication::new(app_id, app_name));

        app.work_dir = Some(work_dir.to_string_lossy().to_string());
        store.save_application(&app).await?;

        Ok(Self {
            pm: ProductManager::new(client.clone()),
            architect: Architect::new(client.clone()),
            planner: Planner::new(client.clone()),
            app,
            store,
            work_dir,
        })
    }

    async fn persist(&self) -> Result<()> {
        self.store.save_application(&self.app).await?;
        Ok(())
    }

    pub async fn analyze_request(&mut self, idea: &str) -> Result<Vec<Requirement>> {
        let reqs = self.pm.process_request(idea)?;
        for req in &reqs {
            self.app.add_primitive(Primitive::Requirement(req.clone()));
        }
        self.persist().await?;
        Ok(reqs)
    }

    pub async fn design_feature(&mut self, id: &str, reqs: &[Requirement]) -> Result<FeatureSpec> {
        let spec = self.architect.design(id, reqs)?;
        self.app
            .add_primitive(Primitive::Specification(spec.clone()));
        self.persist().await?;
        Ok(spec)
    }

    pub async fn create_plan(&mut self, spec: &FeatureSpec) -> Result<ImplementationPlan> {
        let plan = self.planner.plan(spec)?;
        self.app.add_primitive(Primitive::Plan(plan.clone()));
        self.persist().await?;
        Ok(plan)
    }

    pub async fn record_code(&mut self, path: String, content: String) -> Result<()> {
        // 1. Write to filesystem
        let file_path = self.work_dir.join(&path);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&file_path, &content).await?;

        // 2. Persist to DB
        self.app.add_primitive(Primitive::Code { path, content });
        self.persist().await
    }

    pub async fn run(&mut self, ui: &impl crate::interaction::UserInteraction) -> Result<()> {
        // 1. Check for existing session or ask user
        let has_requirements = self
            .app
            .primitives
            .values()
            .any(|p| matches!(p, Primitive::Requirement(_)));

        let feature_idea = if has_requirements {
            ui.log_info("Resuming existing session...");
            String::new()
        } else {
            ui.ask_for_feature("What feature do you want to build?")
                .await?
        };

        if feature_idea.is_empty() && !has_requirements {
            return Ok(()); // User abort
        }

        // 2. Product Phase
        let reqs = if has_requirements {
            self.app
                .primitives
                .values()
                .filter_map(|p| {
                    if let Primitive::Requirement(r) = p {
                        Some(r.clone())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            ui.start_step("PRODUCT MANAGER: Analyzing Request...");
            let reqs = self.analyze_request(&feature_idea).await?;
            ui.end_step("Analysis Complete");
            reqs
        };

        ui.render_requirements(&reqs);

        if !has_requirements && !ui.confirm("Proceed with these requirements?").await? {
            ui.log_info("Aborted.");
            return Ok(());
        }

        // 3. Architect Phase
        let has_spec = self
            .app
            .primitives
            .values()
            .any(|p| matches!(p, Primitive::Specification(_)));
        let spec = if has_spec {
            self.app
                .primitives
                .values()
                .find_map(|p| {
                    if let Primitive::Specification(s) = p {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .unwrap()
        } else {
            ui.start_step("ARCHITECT: Designing Spec...");
            let spec = self.design_feature("new-feature", &reqs).await?;
            ui.end_step("Design Complete");
            spec
        };

        ui.render_spec(&spec);

        if !has_spec && !ui.confirm("Proceed with this specification?").await? {
            ui.log_info("Aborted.");
            return Ok(());
        }

        // 4. Planner Phase
        let has_plan = self
            .app
            .primitives
            .values()
            .any(|p| matches!(p, Primitive::Plan(_)));
        let plan = if has_plan {
            self.app
                .primitives
                .values()
                .find_map(|p| {
                    if let Primitive::Plan(p) = p {
                        Some(p.clone())
                    } else {
                        None
                    }
                })
                .unwrap()
        } else {
            ui.start_step("PLANNER: Creating Plan...");
            let plan = self.create_plan(&spec).await?;
            ui.end_step("Plan Created");
            plan
        };

        ui.render_plan(&plan);

        if !has_plan && !ui.confirm("Proceed with this plan?").await? {
            ui.log_info("Aborted.");
            return Ok(());
        }

        // 5. Execution Phase
        ui.start_step("CONSTRUCTION: Executing Plan...");
        let (results, all_success) =
            crate::plan::executor::PlanExecutor::execute(&plan, &self.work_dir)?;

        // Find the failing step to record an observation later if needed
        let failure_info = results.iter().find_map(|p| {
            if let Primitive::ExecutionStep {
                status,
                action_ref,
                stderr,
                ..
            } = p
            {
                if status == "Failure" {
                    return Some((action_ref.clone(), stderr.clone()));
                }
            }
            None
        });

        let mut successful_steps = 0;
        for p in results {
            if let Primitive::ExecutionStep { status, .. } = &p {
                if status == "Success" {
                    successful_steps += 1;
                }
            }
            self.app.add_primitive(p);
        }

        // Update plan progress
        if let Some(Primitive::Plan(p)) = self.app.primitives.get_mut(&plan.feature_id) {
            p.completed_steps += successful_steps;
        }

        self.persist().await?;

        if !all_success {
            ui.log_error("Execution failed. Initiating AI Self-Healing...");

            if let Some((action_ref, stderr)) = failure_info {
                let obs = Primitive::Observation {
                    insight: format!("Step {} failed with error: {}", action_ref, stderr),
                    context: "Plan execution failure".to_string(),
                    severity: "Error".to_string(),
                };
                ui.log_info(&format!("Recorded observation for {}", action_ref));
                self.app.add_primitive(obs);
                self.persist().await?;
            }

            // TODO: Call Repair Agent with self.get_observations_summary()
            ui.log_info("Observation recorded. Please repair the environment/code and resume.");
            return Ok(());
        }

        ui.end_step("Execution Complete");
        ui.log_info("Success! Pipeline Complete.");

        Ok(())
    }

    pub fn get_observations_summary(&self) -> String {
        let mut summary = String::from("Learnings from previous attempts:\n");
        let mut count = 0;
        for p in self.app.primitives.values() {
            if let Primitive::Observation { insight, .. } = p {
                summary.push_str(&format!("- {}\n", insight));
                count += 1;
            }
        }
        if count == 0 {
            "No previous observations.".to_string()
        } else {
            summary
        }
    }
}
