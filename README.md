# AIO Base Backend

This is the backend service for the AIO platform, providing core functionality for agent management, MCP (Multi-Chain Protocol) operations, and token economy.

## Features

### Agent Management
- Create, read, update, and delete agent items
- Paginated queries for agent listings
- User-specific agent management
- Search and filter capabilities

### MCP (Multi-Chain Protocol) Management
- CRUD operations for MCP items
- Paginated queries for MCP listings
- User-specific MCP management
- Search and filter capabilities

### Work Ledger System
- Trace management for operations
- Status tracking (Todo, InProgress, Completed, Cancelled)
- Paginated queries for traces
- Filtering and sorting capabilities

### Finance System
- Token account management
- Credit stacking and unstacking
- Token claiming and balance management
- Balance summaries and statistics
- Trace-based operation tracking

#### Key Finance Features:
- **Account Management**
  - Create and manage token accounts
  - View account balances and information
  - Delete accounts when needed

- **Credit Operations**
  - Stack credits for enhanced benefits
  - Unstack credits when needed
  - Track credit activities and history

- **Token Operations**
  - Claim tokens based on eligibility
  - Add token balances
  - Transfer tokens between accounts
  - Track token activities and history

- **Balance Tracking**
  - View AIO balance
  - Monitor staked credits
  - Track credit balance
  - Check unclaimed balance

### Token Economy
- Token emission and distribution
- Subscription plan management
- Kappa multiplier system
- Token grants and vesting

#### Key Token Economy Features:
- **Emission System**
  - Base emission rate configuration
  - Kappa factor adjustments
  - Staking bonus calculations
  - Subscription-based multipliers

- **Subscription Plans**
  - Free tier
  - Basic plan
  - Premium plan
  - Enterprise plan

- **Token Grants**
  - Create token grants with vesting periods
  - Track grant status and claimed amounts
  - Manage vesting schedules
  - Claim vested tokens

- **Activity Tracking**
  - Token activity monitoring
  - Credit activity tracking
  - Operation statistics
  - Historical data analysis

## API Endpoints

### Agent API
- `get_agent_item`: Retrieve a specific agent
- `get_all_agent_items`: List all agents
- `get_user_agent_items`: Get user's agents
- `add_agent_item`: Create a new agent
- `update_agent_item`: Modify an existing agent

### MCP API
- `get_mcp_item`: Retrieve a specific MCP
- `get_all_mcp_items`: List all MCPs
- `get_user_mcp_items`: Get user's MCPs
- `add_mcp_item`: Create a new MCP
- `update_mcp_item`: Modify an existing MCP

### Work Ledger API
- `get_trace`: Retrieve a specific trace
- `get_user_traces`: Get user's traces
- `add_trace`: Create a new trace
- `get_traces_with_filters`: Search traces with filters

### Finance API
- `get_account_info`: Get account details
- `add_account`: Create a new account
- `stack_credit`: Stack credits for benefits
- `unstack_credit`: Remove stacked credits
- `claim_token`: Claim available tokens
- `add_token_balance`: Add tokens to account
- `get_balance_summary`: Get account balance overview

### Token Economy API
- `convert_aio_to_credits`: Convert AIO tokens to credits
- `update_exchange_ratio`: Modify exchange rates
- `subscribe_plan`: Manage subscription plans
- `get_kappa`: Retrieve kappa multiplier
- `claim_reward`: Claim available rewards
- `init_emission_policy`: Initialize emission rules
- `calculate_emission`: Compute token emissions
- `create_token_grant`: Create new token grants
- `claim_vested_tokens`: Claim vested tokens

## Development

### Prerequisites
- Rust (latest stable version)
- Cargo
- Internet Computer SDK (dfx)

### Building
```bash
cargo build
```

### Testing
```bash
cargo test
```

### Deployment
```bash
dfx deploy
```

## License
This project is licensed under the MIT License - see the LICENSE file for details.

## Contact
For questions and support, please open an issue in the repository.
