use crate::agents::cli_client::AiCliClient;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct GoogleGenerativeAIClient {
    api_key: String,
    model: String,
    client: Client,
}

impl GoogleGenerativeAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gemini-2.5-flash".to_string(),
            client: Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

impl AiCliClient for GoogleGenerativeAIClient {
    fn prompt(&self, prompt_text: &str) -> Result<String> {
        // Since the trait is synchronous (returns Result<String>), we must block.
        // BUT, blocking in a Tokio runtime causes the panic.
        // We need to change the trait to be async or use `tokio::task::block_in_place`.
        // However, changing the trait ripples everywhere.
        //
        // OPTION 1: Change AiCliClient::prompt to async fn prompt(&self, ...) -> Result<String>
        // This is best but requires refactoring Orchestrator and Agent traits.
        //
        // OPTION 2: Use a blocking HTTP client (reqwest::blocking) inside `task::spawn_blocking`?
        // Still requires async context.
        //
        // OPTION 3: The `ShellCliClient` was using `std::process::Command`, which is blocking but usually fine in `tokio::main` UNLESS it tries to start a new runtime or something.
        //
        // WAIT. The panic "Cannot drop a runtime..." usually comes from `tokio::runtime::Handle::current().block_on(...)` or starting a new Runtime inside async.
        // `ShellCliClient` uses `std::process::Command`. That interprets to synchronous blocking IO.
        // Tokio shouldn't panic just because of blocking IO, strictly speaking, it just blocks the thread.
        //
        // The panic implies something *inside* `ShellCliClient` or `Orchestrator` is trying to mess with Runtimes.
        // `ShellCliClient` code: `cmd.output()?`. This is purely std.
        //
        // Let's re-read the panic: `tokio::...::shutdown.rs:51:21: Cannot drop a runtime...`
        // This happens when a `Runtime` is dropped inside an async context.
        // does `dass-cli/src/main.rs` create a runtime?
        // It uses `#[tokio::main]`.
        //
        // Does `ShellCliClient` spawn a child process that somehow inherits environment variables or something that confuses things? Unlikely.
        //
        // HYPOTHESIS: The `dass-engine` might be using `tokio::spawn` or similar, and something is dropping a runtime handle?
        //
        // Actually, looking at `ShellCliClient`, it runs `gemini` executable.
        // If the `gemini` executable *itself* is a Rust binary using Tokio, that's a separate process, totally fine.
        //
        // Wait, is `ShellCliClient` calling `cargo run`?
        // User command: `cargo run -p dass-cli`
        // CLI calls `gemini`.
        //
        // If the panic is in `dass-cli`, it's the *CLI process* panicking.
        //
        // Let's look at `Orchestrator::run`.
        // It's async.
        // It calls agents.
        // Agents calls `execute`.
        // `GenericAgent::execute` calls `self.model.prompt()`.
        // `self.model` is `ShellCliClient`.
        // `ShellCliClient::prompt` is sync.
        //
        // The panic "Cannot drop a runtime" is very specific.
        // It often happens if you have `let rt = Runtime::new()...` inside an async block.
        // I don't see that in `main.rs`.
        //
        // However, `dialoguer` uses `console`, `term`.
        // Is it possible `dialoguer` interacts poorly?
        //
        // Let's look at `UserInteraction` implementation in `main.rs`.
        // `ask_user` uses `Input::...interact_text()?`. This blocks stdin.
        // Blocking stdin in async tokio main is generally okay-ish (just blocks the thread), but shouldn't panic *Runtime drop*.
        //
        // CHECK `dass-engine/src/orchestrator.rs`.
        // Does it create a Runtime?
        Err(anyhow::anyhow!("Placeholder"))
    }
}
