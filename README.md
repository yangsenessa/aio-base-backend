# AIO Base Backend

## Overview
AIO Base Backend is a Rust-based Internet Computer (IC) canister that serves as the foundation for the AIO ecosystem. It provides core functionality for managing agents, MCPs (Multi-Channel Protocols), work items, and traces within the AIO platform.

## Features
- **Agent Management**: Register, update, and query AI agents with detailed metadata
- **MCP Management**: Handle Multi-Channel Protocols with comprehensive configuration options
- **Work Tracking**: Manage work items with status tracking and metadata
- **Trace System**: Track and monitor agent interactions and executions
- **AIO Protocol Index**: Create and manage protocol indices for better discovery and integration
- **Inverted Index**: Advanced search capabilities for efficient resource discovery

## Architecture

### Core Components
1. **Agent Registry**
   - Agent registration and management
   - Platform-specific configurations
   - Version control and metadata

2. **MCP Registry**
   - Protocol registration and management
   - Resource and tool management
   - Community integration

3. **Work Management System**
   - Task tracking and status management
   - Assignment and ownership
   - Metadata and tagging

4. **Trace System**
   - Execution tracking
   - Call history
   - Performance monitoring

5. **Index Management**
   - Protocol indexing
   - Search optimization
   - Resource discovery

## API Reference

### Agent Management
```candid
get_agent_item: (nat64) -> (opt AgentItem) query
get_all_agent_items: () -> (vec AgentItem) query
add_agent_item: (AgentItem, text) -> (variant { Ok: nat64; Err: text })
update_agent_item: (nat64, AgentItem) -> (variant { Ok; Err: text })
```

### MCP Management
```candid
get_mcp_item: (nat64) -> (opt McpItem) query
get_all_mcp_items: () -> (vec McpItem) query
add_mcp_item: (McpItem, text) -> (variant { Ok: nat64; Err: text })
update_mcp_item: (nat64, McpItem) -> (variant { Ok; Err: text })
```

### Trace System
```candid
get_trace: (nat64) -> (opt TraceItem) query
get_trace_by_id: (text) -> (opt TraceItem) query
add_trace: (TraceItem) -> (variant { Ok: nat64; Err: text })
```

### Index Management
```candid
create_aio_index_from_json: (text, text) -> (variant { Ok; Err: text })
get_aio_index: (text) -> (opt AioIndex) query
search_aio_indices_by_keyword: (text) -> (vec AioIndex) query
```

### Inverted Index Strategy Search
The `revert_Index_find_by_keywords_strategy` function implements an advanced search algorithm for finding the most relevant inverted index item based on multiple keywords. Here's how it works:

#### Input
- An array of keywords to search for

#### Algorithm Steps
1. **Initial Filtering**
   - Excludes items where `method_name` equals 'help'
   - Filters out items with `confidence` less than 0.9
   - Collects all matching items for each keyword

2. **Scoring System**
   - Counts how many keywords match each item
   - Maintains a mapping of MCP names to their matching items and scores

3. **Prioritization Rules**
   The algorithm sorts results using the following priority order:
   1. `standard_match` equals 'true' (highest priority)
   2. Number of keyword matches (secondary priority)
   3. Confidence score (tertiary priority)

4. **Result Selection**
   - Returns the single most relevant item as a JSON string
   - Returns an empty JSON object `{}` if no matches are found

#### Example Usage
```candid
revert_Index_find_by_keywords_strategy: (vec text) -> (text) query
```

#### Return Format
```json
{
    "keyword": "string",
    "keyword_group": "string",
    "mcp_name": "string",
    "method_name": "string",
    "source_field": "string",
    "confidence": float,
    "standard_match": "string"
}
```

## Getting Started

### Prerequisites
- Rust toolchain
- Internet Computer SDK (dfx)
- Cargo package manager

### Installation
1. Clone the repository
2. Install dependencies:
```bash
cargo build
```

### Development
1. Start local IC replica:
```bash
dfx start --background
```

2. Deploy the canister:
```bash
dfx deploy
```

## Dependencies
- candid = "0.10"
- ic-cdk = "0.13"
- ic-cdk-timers = "0.7"
- ic-stable-structures = "0.6"
- serde = "1"
- serde_json = "1.0"
- serde_bytes = "0.11"
- serde_cbor = "0.11"

## Contributing
1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## License
This project is licensed under the MIT License - see the LICENSE file for details.

## Contact
For questions and support, please open an issue in the repository.
