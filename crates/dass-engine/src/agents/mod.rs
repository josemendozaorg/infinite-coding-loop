pub mod architect;
pub mod cli_client;
pub mod engineer;
pub mod product_manager;
pub mod qa;

pub trait Agent {
    /// The unique role name (e.g., "Product Manager")
    fn role(&self) -> &str;
}
