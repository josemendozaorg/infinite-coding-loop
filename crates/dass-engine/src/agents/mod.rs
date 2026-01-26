pub mod cli_client;
pub mod product_manager;
pub mod architect;
pub mod planner;

pub trait Agent {
    fn name(&self) -> &str;
}
