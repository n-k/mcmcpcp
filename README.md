# MCMCPCP - My Cool MCP Command Post

A modern chat interface for interacting with Language Learning Models (LLMs) through the Model Context Protocol (MCP), built with Rust and Dioxus.

## Overview

MCMCPCP provides a powerful and extensible chat interface that allows you to:

- **Chat with LLMs**: Connect to OpenAI-compatible APIs (OpenAI, Anthropic, local models, etc.)
- **Execute Tools**: Use MCP servers to extend LLM capabilities with external tools and services
- **Cross-Platform**: Runs natively on desktop and in web browsers via WebAssembly
- **Real-time Streaming**: See LLM responses as they're generated
- **Built-in Tools**: Includes web fetching capabilities out of the box

## Features

### 🤖 LLM Integration
- Support for OpenAI-compatible APIs
- Streaming responses for real-time interaction
- Configurable model selection
- Message history management

### 🔧 Model Context Protocol (MCP)
- Connect to external MCP servers for extended functionality
- Built-in web fetching tool
- Tool discovery and execution
- Server management and timeout configuration

### 🎨 Modern UI
- Clean, responsive interface built with Dioxus
- Markdown rendering for LLM responses
- Real-time message streaming
- Settings management

### 🌐 Cross-Platform
- Native desktop application
- Web browser support via WebAssembly
- Consistent experience across platforms

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- For desktop: Standard Rust toolchain
- For web: `wasm-pack` and a web server

### Installation

1. Clone the repository:
```bash
git clone https://github.com/n-k/mcmcpcp.git
cd mcmcpcp
```

2. Build and run (desktop):
```bash
cargo run
```

3. For web deployment:
```bash
# Install required tools
cargo install dioxus-cli

# Build for web
dx build --platform web

# Serve the web app
dx serve --platform web
```

### Configuration

1. **LLM Setup**: Configure your LLM API settings in the Settings page:
   - API URL (e.g., `https://api.openai.com/v1`)
   - API Key
   - Model selection

2. **MCP Servers**: Add external MCP servers for additional functionality (see MCP documentation)

## Usage

### Basic Chat
1. Configure your LLM settings
2. Start chatting with the AI
3. The AI can use available tools to help with your requests

### Tool Usage
The AI automatically has access to:
- **Web Fetching**: Retrieve content from URLs
- **External Tools**: Any tools provided by connected MCP servers

### Adding MCP Servers
MCP servers can be added programmatically to extend functionality. See the MCP documentation for creating and configuring servers.

## Architecture

### Core Components

- **`src/main.rs`**: Application entry point and initialization
- **`src/lib.rs`**: Main application component and routing
- **`src/llm.rs`**: LLM client implementation for API communication
- **`src/mcp/`**: Model Context Protocol implementation
  - `host.rs`: MCP host for managing servers
  - `server.rs`: Individual MCP server management
  - `transport.rs`: Communication layer (native only)
- **`src/ui/`**: User interface components
  - `home.rs`: Main chat interface
  - `settings.rs`: Configuration interface
  - `message.rs`: Message display components

### Technology Stack

- **Rust**: Core language for performance and safety
- **Dioxus**: React-like framework for building UIs
- **Tokio**: Async runtime for concurrent operations
- **Reqwest**: HTTP client for API communication
- **Serde**: Serialization/deserialization
- **Pulldown-cmark**: Markdown parsing and rendering

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Project Structure

```
mcmcpcp/
├── src/
│   ├── main.rs          # Application entry point
│   ├── lib.rs           # Main app component
│   ├── llm.rs           # LLM client implementation
│   ├── utils.rs         # Utility functions
│   ├── md2rsx.rs        # Markdown to RSX conversion
│   ├── mcp/             # MCP implementation
│   └── ui/              # UI components
├── assets/              # Static assets (CSS, icons)
├── Cargo.toml          # Rust dependencies
├── Dioxus.toml         # Dioxus configuration
└── README.md           # This file
```

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

### Development Guidelines

1. Follow Rust best practices and idioms
2. Add comprehensive documentation for new features
3. Include tests for new functionality
4. Ensure cross-platform compatibility

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Dioxus](https://dioxuslabs.com/) for the excellent React-like framework
- [Model Context Protocol](https://modelcontextprotocol.io/) for the extensible tool interface
- The Rust community for amazing tools and libraries
