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

### User Profile Management
- User profile creation and management
- Multi-authentication method support
- Profile data indexing and search
- Privacy and security features

#### Key User Profile Features:
- **Profile Management**
  - Create and update user profiles
  - Support for multiple authentication methods (Wallet, Google, Internet Identity)
  - Flexible profile data structure with optional fields
  - Automatic timestamp management (created_at, updated_at)

- **Authentication Integration**
  - Principal ID-based authentication
  - User ID management
  - Email-based profile lookup
  - Login status tracking

- **Data Indexing**
  - Multi-index support for fast lookups
  - Principal ID indexing for authentication
  - User ID indexing for profile management
  - Email indexing for contact purposes

- **Privacy & Security**
  - Optional metadata support for extensibility
  - Secure storage with stable memory structures
  - Audit trail maintenance
  - Data integrity preservation

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

### User Profile API
- `upsert_user_profile`: Create or update user profile
- `get_user_profile_by_principal`: Retrieve profile by principal ID
- `get_user_profile_by_user_id`: Retrieve profile by user ID
- `get_user_profile_by_email`: Retrieve profile by email address
- `update_user_nickname`: Update user nickname
- `get_user_profiles_paginated`: List all profiles with pagination
- `delete_user_profile`: Remove user profile
- `get_total_user_profiles`: Get total number of profiles

## Data Structures

### UserProfile Structure
The UserProfile structure provides comprehensive user management capabilities:

```rust
pub struct UserProfile {
    pub user_id: String,                    // Unique user identifier
    pub principal_id: String,               // Internet Computer principal ID
    pub name: Option<String>,               // Legacy compatibility field
    pub nickname: String,                   // User display name
    pub login_method: LoginMethod,          // Authentication method used
    pub login_status: LoginStatus,          // Current authentication status
    pub email: Option<String>,              // User email address (optional)
    pub picture: Option<String>,            // Profile picture URL (optional)
    pub wallet_address: Option<String>,     // Wallet address (optional)
    pub created_at: u64,                    // Profile creation timestamp
    pub updated_at: u64,                    // Last update timestamp
    pub metadata: Option<String>,           // Additional JSON metadata
}
```

### Authentication Enums
```rust
pub enum LoginMethod {
    Wallet,     // Wallet-based authentication
    Google,     // Google OAuth authentication
    II,         // Internet Identity authentication
}

pub enum LoginStatus {
    Authenticated,      // User is currently authenticated
    Unauthenticated,    // User is not authenticated
}
```

### Storage Architecture
- **Main Storage**: `StableVec<UserProfile>` for profile data persistence
- **Indexing**: Multiple `StableBTreeMap` structures for fast lookups
- **Memory Management**: Centralized memory allocation via `stable_mem_storage.rs`
- **Data Integrity**: Automatic index synchronization and validation

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
