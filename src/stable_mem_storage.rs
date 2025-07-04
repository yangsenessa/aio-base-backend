// Centralized stable memory storage for all modules
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use std::cell::RefCell;
use crate::mining_reword::{MiningRewardPolicy, RewardEntry, UserRewardKey};
use crate::token_economy_types::RewardIdList;
use crate::account_storage::AccountKey;
use crate::token_economy_types::AccountInfo;

// Type alias for memory
pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    // Global memory manager
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    // Agent Items
    pub static AGENT_ITEMS: RefCell<StableVec<crate::agent_asset_types::AgentItem, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ).unwrap()
    );
    pub static USER_AGENT_INDEX: RefCell<StableBTreeMap<crate::agent_asset_types::UserAgentKey, (), Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))
        )
    );

    // MCP Items
    pub static MCP_ITEMS: RefCell<StableBTreeMap<String, crate::mcp_asset_types::McpItem, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(31)))
        )
    );
    pub static USER_MCP_INDEX: RefCell<StableBTreeMap<crate::mcp_asset_types::UserMcpKey, (), Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(32)))
        )
    );
    pub static MCP_STACK_RECORDS: RefCell<StableBTreeMap<u64, crate::mcp_asset_types::McpStackRecord, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(33)))
        )
    );

    // Inverted Index
    pub static INVERTED_INDEX_STORE: RefCell<crate::aio_invert_index_types::InvertedIndexStore> = RefCell::new(
        crate::aio_invert_index_types::InvertedIndexStore::new(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(111))))
    );

    // Work Ledger
    pub static TRACE_ITEMS: RefCell<StableVec<crate::aio_workledger_types::TraceItem, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
        ).unwrap()
    );
    pub static USER_TRACE_INDEX: RefCell<StableBTreeMap<crate::aio_workledger_types::UserTraceKey, u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
        )
    );
    pub static TRACE_ID_INDEX: RefCell<StableBTreeMap<String, u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5)))
        )
    );

    // Protocol Index
    pub static AIO_INDICES: RefCell<StableBTreeMap<String, crate::aio_protocal_types::AioIndex, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(15)))
        )
    );
    pub static KEYWORD_INDEX: RefCell<StableBTreeMap<String, crate::aio_protocal_types::StringVec, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6)))
        )
    );

    // Token Economy
    pub static EMISSION_POLICY: RefCell<StableBTreeMap<String, crate::token_economy_types::EmissionPolicy, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(21)))
        )
    );
    pub static NEWUSER_GRANTS: RefCell<StableBTreeMap<crate::token_economy_types::TokenGrantKey, crate::token_economy_types::TokenGrant, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(24)))
        )
    );
    pub static NEWMCP_GRANTS: RefCell<StableBTreeMap<crate::token_economy_types::NewMcpGrantKey, crate::token_economy_types::NewMcpGrant, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(25)))
        )
    );
    pub static TOKEN_ACTIVITIES: RefCell<StableBTreeMap<u64, crate::token_economy_types::TokenActivity, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(26)))
        )
    );
    pub static CREDIT_ACTIVITIES: RefCell<StableBTreeMap<u64, crate::token_economy_types::CreditActivity, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(27)))
        )
    );
    pub static GRANT_POLICIES: RefCell<StableBTreeMap<crate::token_economy_types::GrantAction, crate::token_economy_types::GrantPolicy, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(28)))
        )
    );

    // Mining Reward Storage
    pub static MINING_REWARD_POLICY: RefCell<StableBTreeMap<String, MiningRewardPolicy, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(34)))
        )
    );
    pub static REWARD_ENTRIES: RefCell<StableBTreeMap<u64, RewardEntry, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(35)))
        )
    );
    pub static USER_REWARD_INDEX: RefCell<StableBTreeMap<UserRewardKey, RewardIdList, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(36)))
        )
    );
    pub static MCP_REWARD_INDEX: RefCell<StableBTreeMap<String, RewardIdList, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(37)))
        )
    );

    // Trace Storage (for trace_storage.rs)
    pub static TRACE_STORAGE: RefCell<StableBTreeMap<String, crate::trace_storage::TraceLog, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(11)))
        )
    );

    // Credit Exchange & Recharge
    pub static CREDIT_CONVERT_CONTRACT: RefCell<StableBTreeMap<String, crate::token_economy_types::CreditConvertContract, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(51)))
        )
    );
    pub static RECHARGE_RECORDS: RefCell<StableBTreeMap<u64, crate::token_economy_types::RechargeRecord, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(52)))
        )
    );
    pub static RECHARGE_PRINCIPAL_ACCOUNTS: RefCell<StableVec<crate::token_economy_types::RechargePrincipalAccount, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(53)))
        ).unwrap()
    );

    pub static ACCOUNTS: RefCell<StableBTreeMap<AccountKey, AccountInfo, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(10)))
        )
    );
} 