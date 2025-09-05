//! Main entry point for the MCMCPCP (My Cool MCP Command Post) application.
//! 
//! This application provides a chat interface for interacting with Language Learning Models (LLMs)
//! through the Model Context Protocol (MCP), allowing users to execute tools and commands
//! through connected MCP servers.

/// Main function that initializes and launches the MCMCPCP application.
/// 
/// This function:
/// 1. Initializes the Dioxus logger with WARN level logging
/// 2. Creates an MCP Host with configured timeouts for server communication
/// 3. Launches the Dioxus application with the MCP Host as shared context
fn main() {
    // Initialize logging for the application with WARN level to reduce noise
    dioxus::logger::init(dioxus::logger::tracing::Level::WARN).unwrap();
    
    // Create an MCP Host instance with timeouts:
    // - 10 second timeout for individual operations
    // - 30 second timeout for server startup/initialization
    // let host = Arc::new(mcmcpcp::mcp::host::MCPHost::new(
    //     Duration::from_millis(10_000),  // Operation timeout
    //     Duration::from_millis(30_000),  // Server startup timeout
    // ));

    // Launch the Dioxus application with the MCP Host as shared context
    // This allows all components to access the MCP Host for tool execution
    // LaunchBuilder::new().with_context(host).launch(mcmcpcp::App)
    dioxus::launch(mcmcpcp::App)
}
