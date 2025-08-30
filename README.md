# AIO Base Backend

An advanced blockchain-based backend service for the AIO (AI Operations) platform, providing comprehensive functionality for agent management, MCP (Multi-Chain Protocol) operations, token economy, and distributed computing infrastructure.

## License

This project is licensed under the MIT License - see the [LICENSE](#license) section for details.

## Overview

The AIO Base Backend is built on the Internet Computer Protocol (ICP) and serves as the core infrastructure for:

- **Agent Asset Management**: Create, manage, and deploy AI agents
- **MCP Protocol Support**: Multi-Chain Protocol operations and management
- **Token Economy**: Comprehensive token and credit system with staking, rewards, and governance
- **Work Ledger System**: Distributed task tracking and execution tracing
- **Inverted Index System**: Advanced search and discovery capabilities
- **Mining Rewards**: Automated reward distribution system
- **Credit Exchange**: ICP-to-Credit conversion system
- **Social Chat System**: Point-to-point messaging with notification queue support
- **Pixel Art Creation**: Complete pixel art project management with version control and IoT export

## Quick Start

### Prerequisites

- Rust (latest stable version)
- Internet Computer SDK (dfx)
- Node.js and npm (for frontend components)

### Installation and Deployment

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd project/src/aio-base-backend
   ```

2. **Start the local development environment**
   ```bash
   chmod +x ../../build.sh
   ../../build.sh
   ```

   This script will:
   - Stop any running dfx instances
   - Start dfx in background with clean state
   - Deploy the aio-base-frontend
   - Configure recharge principal account
   - Initialize token minting

3. **Verify deployment**
   ```bash
   dfx canister status aio-base-backend
   ```

## API Reference

### Core Data Types

#### Agent Management
```candid
type AgentItem = record {
  id: nat64;
  name: text;
  description: text;
  author: text;
  owner: text;
  platform: opt Platform;
  git_repo: text;
  homepage: opt text;
  input_params: opt text;
  output_example: opt text;
  image_url: opt text;
  exec_file_url: opt text;
  version: text;
};
```

#### MCP Management
```candid
type McpItem = record {
  id: nat64;
  name: text;
  description: text;
  author: text;
  owner: text;
  git_repo: text;
  exec_file: opt text;
  homepage: opt text;
  remote_endpoint: opt text;
  mcp_type: text;
  community_body: opt text;
  resources: bool;
  prompts: bool;
  tools: bool;
  sampling: bool;
};
```

#### Token Economy
```candid
type AccountInfo = record {
  principal_id: text;
  token_info: TokenInfo;
  created_at: nat64;
  updated_at: opt nat64;
  metadata: opt text;
};

type TokenInfo = record {
  token_balance: nat64;
  credit_balance: nat64;
  staked_credits: nat64;
  kappa_multiplier: float64;
};
```

#### Social Chat System
```candid
type MessageMode = variant {
  Text;
  Voice;
  Image;
  Emoji;
};

type ChatMessage = record {
  send_by: text;
  content: text;
  mode: MessageMode;
  timestamp: nat64;
};

type NotificationItem = record {
  social_pair_key: text;
  to_who: text;
  message_id: nat64;
  timestamp: nat64;
};
```

#### Pixel Art Creation System
```candid
type ProjectId = text;
type VersionId = text;

type PixelArtSource = record {
  width: nat32;
  height: nat32;
  palette: vec text;              // HEX color values like "#FF0000"
  pixels: vec (vec nat16);        // Color palette index matrix
  frames: opt vec Frame;          // Optional animation frames
  metadata: opt SourceMeta;       // Optional metadata
};

type Frame = record {
  duration_ms: nat32;             // Frame duration in milliseconds
  pixels: vec (vec nat16);        // Pixel matrix for this frame
};

type SourceMeta = record {
  title: opt text;
  description: opt text;
  tags: opt vec text;
};

type Version = record {
  version_id: VersionId;
  created_at: nat64;              // Unix timestamp in seconds
  editor: principal;              // Version creator
  message: opt text;              // Version commit message
  source: PixelArtSource;
};

type Project = record {
  project_id: ProjectId;
  owner: principal;
  created_at: nat64;
  updated_at: nat64;
  current_version: Version;
  history: vec Version;           // Complete version history
};
```

### API Endpoints

#### 1. Agent Asset Management

##### Basic Operations
- **`get_agent_item(id: nat64) -> opt AgentItem`**
  - Retrieve specific agent by ID
  
- **`get_all_agent_items() -> vec AgentItem`**
  - Get all available agents
  
- **`add_agent_item(agent: AgentItem, principal_id: text) -> variant { Ok: nat64; Err: text }`**
  - Create new agent with automatic owner assignment
  
- **`update_agent_item(id: nat64, agent: AgentItem) -> variant { Ok; Err: text }`**
  - Update existing agent (owner verification required)

##### Advanced Queries
- **`get_user_agent_items() -> vec AgentItem`**
  - Get agents owned by caller
  
- **`get_agent_items_paginated(offset: nat64, limit: nat64) -> vec AgentItem`**
  - Paginated agent listing
  
- **`get_agent_item_by_name(name: text) -> opt AgentItem`**
  - Find agent by name

#### 2. MCP (Multi-Chain Protocol) Management

##### Core MCP Operations
- **`get_mcp_item(name: text) -> opt McpItem`**
  - Retrieve MCP by name
  
- **`add_mcp_item(mcp: McpItem, principal_id: text) -> variant { Ok: text; Err: text }`**
  - Register new MCP with validation
  
- **`update_mcp_item(name: text, mcp: McpItem) -> variant { Ok; Err: text }`**
  - Update MCP configuration
  
- **`delete_mcp_item(name: text) -> variant { Ok; Err: text }`**
  - Remove MCP and associated indices

##### MCP Staking System
- **`stack_credit(principal_id: text, mcp_name: text, amount: nat64) -> variant { Ok: AccountInfo; Err: text }`**
  - Stake credits to specific MCP
  
- **`get_mcp_stack_records_paginated(mcp_name: text, offset: nat64, limit: nat64) -> vec McpStackRecord`**
  - Get staking records for MCP

#### 3. Token Economy System

##### Account Management
- **`add_account(principal_id: text) -> variant { Ok: AccountInfo; Err: text }`**
  - Create new token account
  
- **`get_account_info(principal_id: text) -> opt AccountInfo`**
  - Retrieve account information
  
- **`get_balance_summary(principal_id: text) -> record { total_count: nat64; total_amount: nat64; success_count: nat64; unclaimed_balance: nat64 }`**
  - Get comprehensive balance overview

##### Credit Operations
- **`use_credit(principal_id: text, amount: nat64, service: text, metadata: opt text) -> variant { Ok: AccountInfo; Err: text }`**
  - Consume credits for services
  
- **`unstack_credit(principal_id: text, amount: nat64) -> variant { Ok: AccountInfo; Err: text }`**
  - Unstake credits from MCPs

##### Token Grants and Rewards
- **`create_and_claim_newuser_grant(principal_id: text) -> variant { Ok: nat64; Err: text }`**
  - Create and claim new user bonus
  
- **`create_and_claim_newmcp_grant(principal_id: text, mcp_name: text) -> variant { Ok: nat64; Err: text }`**
  - Create and claim MCP developer grant

#### 4. Mining Rewards System

##### Reward Distribution
- **`dispatch_mining_rewards() -> variant { Ok; Err: text }`**
  - Start automated reward distribution (runs every 5 minutes)
  
- **`stop_mining_rewards() -> variant { Ok; Err: text }`**
  - Stop automated reward distribution
  
- **`cal_unclaim_rewards(principal_id: text) -> nat64`**
  - Calculate unclaimed rewards for user
  
- **`claim_rewards(principal_id: text) -> variant { Ok: nat64; Err: text }`**
  - Claim accumulated mining rewards

#### 5. Work Ledger & Trace System

##### Trace Management
- **`record_trace_call(trace_id: text, context_id: text, protocol: text, agent: text, call_type: text, method: text, input: IOValue, output: IOValue, status: text, error_message: opt text) -> variant { Ok: null; Err: text }`**
  - Record execution trace for operations
  
- **`get_traces_paginated(offset: nat64, limit: nat64) -> vec TraceLog`**
  - Paginated trace retrieval
  
- **`get_traces_with_filters(protocols: opt vec text, methods: opt vec text, statuses: opt vec text) -> vec TraceLog`**
  - Advanced trace filtering
  
- **`get_traces_statistics() -> record { total_count: nat64; success_count: nat64; error_count: nat64 }`**
  - Get trace execution statistics

#### 6. AIO Protocol Index System

##### Index Management
- **`create_aio_index_from_json(name: text, json_str: text) -> variant { Ok; Err: text }`**
  - Create protocol index from JSON specification
  
- **`get_aio_index(id: text) -> opt AioIndex`**
  - Retrieve protocol index
  
- **`search_aio_indices_by_keyword(keyword: text) -> vec AioIndex`**
  - Search indices by keyword

#### 7. Inverted Index System

##### Search & Discovery
- **`store_inverted_index(mcp_name: text, json_str: text) -> variant { Ok; Err: text }`**
  - Store searchable index for MCP
  
- **`find_inverted_index_by_keywords(keywords: vec text, min_confidence: float32) -> text`**
  - Multi-keyword search with confidence scoring
  
- **`revert_Index_find_by_keywords_strategy(keywords: vec text) -> text`**
  - Advanced keyword matching strategy

#### 8. Credit Exchange System

##### ICP-Credit Conversion
- **`get_credits_per_icp_api() -> nat64`**
  - Get current ICP to Credit exchange rate
  
- **`simulate_credit_from_icp_api(icp_amount: float64) -> nat64`**
  - Simulate credit amount from ICP
  
- **`recharge_and_convert_credits_api(icp_amount: float64) -> nat64`**
  - Execute ICP to Credit conversion
  
- **`get_recharge_history_api(principal: text, offset: nat64, limit: nat64) -> vec RechargeRecord`**
  - Get recharge transaction history

#### 9. Social Chat System

##### Core Chat Operations
- **`generate_social_pair_key(principal1: text, principal2: text) -> text`**
  - Generate deterministic social pair key from two principal IDs
  - Uses sorting algorithm to ensure same key regardless of sender/receiver order
  
- **`send_chat_message(sender_principal: text, receiver_principal: text, content: text, mode: MessageMode) -> variant { Ok: nat64; Err: text }`**
  - Send chat message between two users
  - Supports Text, Voice, Image, and Emoji modes
  - Non-text content should be base64 encoded
  - Automatically pushes notification to receiver's queue
  
- **`get_recent_chat_messages(principal1: text, principal2: text) -> vec ChatMessage`**
  - Get last 5 chat messages between two users
  - Messages returned in chronological order
  
- **`get_chat_messages_paginated(principal1: text, principal2: text, offset: nat64, limit: nat64) -> vec ChatMessage`**
  - Get paginated chat messages with offset and limit
  - Useful for loading chat history in chunks
  
- **`get_chat_message_count(principal1: text, principal2: text) -> nat64`**
  - Get total number of messages between two users

##### Notification Queue System
- **`pop_notification(receiver_principal: text) -> opt NotificationItem`**
  - Pop and remove the first notification for receiver
  - Returns None if no notifications available
  - Used for polling new messages
  
- **`get_notifications_for_receiver(receiver_principal: text) -> vec NotificationItem`**
  - Get all notifications for receiver without removing them
  - Useful for checking notification count
  
- **`clear_notifications_for_pair(social_pair_key: text, receiver_principal: text) -> variant { Ok: nat64; Err: text }`**
  - Clear all notifications for specific social pair and receiver
  - Returns number of notifications removed
  - Useful for marking conversations as read

#### 10. Pixel Art Creation System

##### Project Management
- **`create_pixel_project(source: PixelArtSource, message: opt text) -> variant { Ok: ProjectId; Err: text }`**
  - Create new pixel art project with initial version
  - Caller becomes project owner
  - Returns unique project identifier
  
- **`get_pixel_project(project_id: ProjectId) -> opt Project`**
  - Retrieve complete project information including version history
  - Returns None if project doesn't exist or access denied
  
- **`delete_pixel_project(project_id: ProjectId) -> variant { Ok: text; Err: text }`**
  - Delete entire project and all its versions
  - Only project owner can delete
  - Returns confirmation message

##### Version Control
- **`save_pixel_version(project_id: ProjectId, source: PixelArtSource, message: opt text, if_match_version: opt text) -> variant { Ok: VersionId; Err: text }`**
  - Save new version to existing project
  - Supports optimistic concurrency control with if_match_version
  - Updates project's current_version and appends to history
  
- **`get_pixel_version(project_id: ProjectId, version_id: VersionId) -> opt Version`**
  - Retrieve specific version from project history
  - Useful for version comparison and rollback
  
- **`get_pixel_current_source(project_id: ProjectId) -> opt PixelArtSource`**
  - Get current version's pixel art source data
  - Optimized for quick access to latest artwork

##### Export and Sharing
- **`export_pixel_for_device(project_id: ProjectId, version_id: opt VersionId) -> variant { Ok: text; Err: text }`**
  - Export compact JSON format optimized for IoT devices
  - If version_id not specified, exports current version
  - Returns minified JSON with type identifier "pixel_art@1"

##### Discovery and Listing
- **`list_pixel_projects_by_owner(owner: principal, offset: nat64, limit: nat64) -> vec Project`**
  - Get paginated list of projects owned by specific user
  - Supports efficient browsing of large project collections
  
- **`get_total_pixel_project_count() -> nat64`**
  - Get total number of pixel art projects in system
  - Useful for pagination and statistics

##### Data Validation Features
- **Canvas Size Validation**: Supports 1x1 to 512x512 pixel canvases
- **Color Palette Management**: Up to 256 colors per project with HEX validation
- **Animation Support**: Up to 60 frames with duration control
- **Payload Size Limits**: Maximum 1MB per project for optimal performance
- **Optimistic Locking**: Prevents conflicting concurrent edits

## Architecture

### Core Components

1. **Agent Asset Types** (`agent_asset_types.rs`): Agent lifecycle management
2. **MCP Asset Types** (`mcp_asset_types.rs`): Multi-Chain Protocol handling
3. **Token Economy** (`token_economy.rs`): Economic system implementation
4. **Trace Storage** (`trace_storage.rs`): Execution tracking and audit trails
5. **Stable Memory Storage** (`stable_mem_storage.rs`): Persistent data storage
6. **Mining Rewards** (`mining_reword.rs`): Automated reward distribution
7. **Society Profile Types** (`society_profile_types.rs`): User profiles, contacts, and social chat system
8. **Pixel Creation Types** (`pixel_creation_types.rs`): Pixel art project management with version control

### Data Flow

```
User Request → API Endpoint → Core Logic → Stable Storage → Response
                     ↓
            Trace Logging → Audit Trail
                     ↓
            Mining Rewards → Token Distribution
```

## Development

### Building
```bash
cargo build --release --target wasm32-unknown-unknown
```

### Testing
```bash
cargo test
```

### Local Development
```bash
# Start local replica
dfx start --background

# Deploy backend
dfx deploy aio-base-backend

# Deploy frontend
dfx deploy aio-base-frontend
```

### Environment Configuration

The system automatically configures:
- Recharge principal account for ICP-Credit conversion
- Token minting initialization
- Frontend-backend integration

## Configuration

### Recharge Principal Setup
The build script automatically configures a recharge principal for ICP-Credit conversion:
```bash
RECHARGE_PRINCIPAL_ID="jzpwm-zsjcq-ugkzp-nr7au-bydmm-c7rqk-tzp2r-gtode-fws2v-ehkfl-cqe"
```

### Token Economics
- Base emission rate: Configurable through emission policy
- Staking bonuses: Applied automatically for staked credits
- New user grants: 1000 credits
- New MCP grants: 10000 credits

## Security Features

- **Principal-based Authentication**: All operations verified against caller identity
- **Owner Verification**: Asset modifications restricted to owners
- **Trace Auditing**: Complete operation logging for transparency
- **Stable Storage**: Crash-resistant data persistence
- **Error Handling**: Comprehensive error reporting and recovery

## Monitoring

### Available Statistics
- Total staked credits across all MCPs
- Mining reward distribution metrics
- User activity and trace statistics
- Token circulation and grant status

### Logging
All API calls are logged with:
- Input parameters
- Execution results
- Error details
- Performance metrics

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with comprehensive tests
4. Submit a pull request with detailed description

## Troubleshooting

### Common Issues

1. **Deployment Failures**
   - Ensure dfx is running: `dfx ping`
   - Check available cycles: `dfx wallet balance`

2. **Permission Errors**
   - Verify principal identity: `dfx identity whoami`
   - Check asset ownership

3. **Storage Issues**
   - Monitor canister memory usage
   - Consider stable storage optimization

## License

MIT License

Copyright (c) 2024 AIO Platform

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Recent Updates

### Version 1.1.0 - Pixel Art Creation System (December 2024)

#### New Features
- **Complete Pixel Art Creation System**: Added comprehensive pixel art project management with full version control capabilities
- **Project Management**: Create, retrieve, update, and delete pixel art projects with owner-based access control
- **Version History**: Full version tracking with optimistic concurrency control and rollback capabilities
- **Animation Support**: Support for animated pixel art with frame-based timeline and duration control
- **IoT Export**: Compact JSON export format optimized for IoT devices and embedded systems
- **Advanced Validation**: Comprehensive data validation including canvas size, color palette, and animation constraints

#### Technical Implementation
- **New Module**: `pixel_creation_types.rs` - Core data structures and business logic
- **Stable Storage**: Memory allocation for pixel projects (MemoryId: 90-91) with ic-stable-structures
- **API Endpoints**: 9 new Candid API functions for complete pixel art workflow
- **Data Types**: ProjectId, VersionId, PixelArtSource, Frame, Version, Project with full Candid support
- **Performance**: 1MB payload limits, up to 512x512 canvas, 256 colors, 60 animation frames

#### API Additions
- `create_pixel_project` - Project creation with initial version
- `save_pixel_version` - Version control with optimistic locking
- `get_pixel_project` - Complete project retrieval with history
- `get_pixel_version` - Specific version access
- `get_pixel_current_source` - Current version quick access
- `export_pixel_for_device` - IoT-optimized export
- `list_pixel_projects_by_owner` - User project discovery
- `delete_pixel_project` - Project removal
- `get_total_pixel_project_count` - System statistics

#### Integration Ready
- **Frontend Integration**: Complete JavaScript service class with React/Vue hooks
- **Authentication**: Full IC identity integration with Principal-based access control
- **Documentation**: Comprehensive API documentation and integration examples
- **Error Handling**: Robust validation and error reporting system

---

## Support

For questions and support, please open an issue in the repository or contact the development team.

---

**Version**: 1.1.0  
**Last Updated**: December 2024  
**Platform**: Internet Computer Protocol (ICP)
