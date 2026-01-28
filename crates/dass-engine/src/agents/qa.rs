use crate::agents::Agent;
use crate::agents::cli_client::AiCliClient;

pub struct QaEngineer<C: AiCliClient> {
    #[allow(dead_code)]
    client: C,
}

impl<C: AiCliClient> Agent for QaEngineer<C> {
    fn role(&self) -> &str {
        "QA Engineer"
    }
}

impl<C: AiCliClient> QaEngineer<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }
}
