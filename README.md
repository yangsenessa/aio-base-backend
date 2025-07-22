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

## Architecture

### Core Components

1. **Agent Asset Types** (`agent_asset_types.rs`): Agent lifecycle management
2. **MCP Asset Types** (`mcp_asset_types.rs`): Multi-Chain Protocol handling
3. **Token Economy** (`token_economy.rs`): Economic system implementation
4. **Trace Storage** (`trace_storage.rs`): Execution tracking and audit trails
5. **Stable Memory Storage** (`stable_mem_storage.rs`): Persistent data storage
6. **Mining Rewards** (`mining_reword.rs`): Automated reward distribution

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

## Support

For questions and support, please open an issue in the repository or contact the development team.

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**Platform**: Internet Computer Protocol (ICP)
