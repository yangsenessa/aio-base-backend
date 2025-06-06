use candid::{CandidType, Principal, Decode, Encode};
use ic_cdk::api::time;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use crate::stable_mem_storage::{MINING_REWARD_POLICY, REWARD_ENTRIES, USER_REWARD_INDEX, MCP_REWARD_INDEX};
use crate::token_economy_types::RewardIdList;
use ic_stable_structures::storable::Bound;
use std::borrow::Cow;

// Quarterly reward configuration
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct QuarterRewardConfig {
    pub quarter: u32,
    pub base_reward: u64,
    pub estimated_calls: u64,
    pub total_quarter_emission: u64,
}

// Global mining policy
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct MiningRewardPolicy {
    pub start_epoch_sec: u64,
    pub decay_rate: f32,
    pub total_emission_cap: u64,
    pub quarters: Vec<QuarterRewardConfig>,
}

// Kappa tier structure
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct KappaTier {
    pub stake_ratio_threshold: f32,
    pub kappa_multiplier: f32,
}

// Stake record
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct StakeRecord {
    pub principal_id: Principal,
    pub mcp_name: String,
    pub stack_amount: u64,
}

// Reward entry
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct RewardEntry {
    pub principal_id: Principal,
    pub mcp_name: String,
    pub reward_amount: u64,
    pub block_id: u64,
    pub status: String,
}

// Call record
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct CallRecord {
    pub block_id: u64,
    pub mcp_name: String,
    pub quality_score: f32,
}

// User reward key for indexing
#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserRewardKey {
    pub principal_id: Principal,
    pub mcp_name: String,
}

// Implement Storable for QuarterRewardConfig
impl ic_stable_structures::Storable for QuarterRewardConfig {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode QuarterRewardConfig"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode QuarterRewardConfig")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for MiningRewardPolicy
impl ic_stable_structures::Storable for MiningRewardPolicy {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode MiningRewardPolicy"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode MiningRewardPolicy")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for RewardEntry
impl ic_stable_structures::Storable for RewardEntry {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode RewardEntry"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode RewardEntry")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for UserRewardKey
impl ic_stable_structures::Storable for UserRewardKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode UserRewardKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode UserRewardKey")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Default mining configuration
pub fn default_mining_config() -> MiningRewardPolicy {
    let mut quarters = Vec::new();
    let mut base_reward = 300_000u64;
    let mut estimated_calls = 1000u64;
    let decay_rate = 0.045f32; // 4.5% decay per quarter
    
    for quarter in 1..=40 {
        let total_quarter_emission = base_reward * estimated_calls;
        quarters.push(QuarterRewardConfig {
            quarter,
            base_reward,
            estimated_calls,
            total_quarter_emission,
        });
        
        // Update values for next quarter
        base_reward = (base_reward as f32 * (1.0 - decay_rate)) as u64;
        estimated_calls = (estimated_calls as f32 * 1.2) as u64; // 20% growth
    }

    MiningRewardPolicy {
        start_epoch_sec: time(),
        decay_rate,
        total_emission_cap: 8_400_000_000_000_000u64, // 8.4 Quadrillion
        quarters,
    }
}

// Default Kappa tiers
pub fn default_kappa_tiers() -> Vec<KappaTier> {
    vec![
        KappaTier { stake_ratio_threshold: 0.01, kappa_multiplier: 1.0 },
        KappaTier { stake_ratio_threshold: 0.05, kappa_multiplier: 1.1 },
        KappaTier { stake_ratio_threshold: 0.10, kappa_multiplier: 1.3 },
        KappaTier { stake_ratio_threshold: 0.25, kappa_multiplier: 1.5 },
        KappaTier { stake_ratio_threshold: 0.50, kappa_multiplier: 1.7 },
        KappaTier { stake_ratio_threshold: 0.75, kappa_multiplier: 1.85 },
        KappaTier { stake_ratio_threshold: 1.00, kappa_multiplier: 2.0 },
    ]
}

// Initialize mining policy
pub fn init_mining_policy() {
    let policy = default_mining_config();
    MINING_REWARD_POLICY.with(|p| {
        p.borrow_mut().insert("default".to_string(), policy);
    });
}

// Calculate user rewards
pub fn calculate_user_rewards_for_mcp(
    mcp_name: String,
    calls: Vec<CallRecord>,
    stakes: Vec<StakeRecord>,
    current_quarter: u32,
) -> Vec<RewardEntry> {
    let mut reward_entries = Vec::new();
    let mut next_reward_id = 0u64;
    
    // Get base reward for current quarter
    let policy = MINING_REWARD_POLICY.with(|p| {
        let policy_store = p.borrow();
        if policy_store.is_empty() {
            // Initialize policy if empty
            drop(policy_store); // Release the borrow before mutating
            init_mining_policy();
            p.borrow().get(&"default".to_string()).unwrap()
        } else {
            policy_store.get(&"default".to_string()).unwrap_or_else(default_mining_config)
        }
    });
    
    let quarter_config = policy.quarters
        .iter()
        .find(|q| q.quarter == current_quarter)
        .expect("Invalid quarter");
    
    // Calculate total stake for MCP
    let total_stake: u64 = stakes.iter()
        .filter(|s| s.mcp_name == mcp_name)
        .map(|s| s.stack_amount)
        .sum();
    
    // Process each call record
    for call in calls.iter().filter(|c| c.mcp_name == mcp_name) {
        for stake in stakes.iter().filter(|s| s.mcp_name == mcp_name) {
            let stake_ratio = stake.stack_amount as f32 / total_stake as f32;
            
            // Determine kappa multiplier
            let kappa = default_kappa_tiers()
                .iter()
                .rev()
                .find(|tier| stake_ratio >= tier.stake_ratio_threshold)
                .map(|tier| tier.kappa_multiplier)
                .unwrap_or(1.0);
            
            // Calculate reward
            let reward = (quarter_config.base_reward as f32 * kappa * call.quality_score) as u64;
            
            let reward_entry = RewardEntry {
                principal_id: stake.principal_id,
                mcp_name: mcp_name.clone(),
                reward_amount: reward,
                block_id: call.block_id,
                status: "pending".to_string(),
            };
            
            // Store in stable memory
            REWARD_ENTRIES.with(|entries| {
                entries.borrow_mut().insert(next_reward_id, reward_entry.clone());
            });
            
            // Update user reward index
            let user_key = UserRewardKey {
                principal_id: stake.principal_id,
                mcp_name: mcp_name.clone(),
            };
            USER_REWARD_INDEX.with(|index| {
                let mut reward_ids = index.borrow()
                    .get(&user_key)
                    .map(|list| list.0)
                    .unwrap_or_default();
                reward_ids.push(next_reward_id);
                index.borrow_mut().insert(user_key, RewardIdList(reward_ids));
            });
            
            // Update MCP reward index
            MCP_REWARD_INDEX.with(|index| {
                let mut reward_ids = index.borrow()
                    .get(&mcp_name)
                    .map(|list| list.0)
                    .unwrap_or_default();
                reward_ids.push(next_reward_id);
                index.borrow_mut().insert(mcp_name.clone(), RewardIdList(reward_ids));
            });
            
            reward_entries.push(reward_entry);
            next_reward_id += 1;
        }
    }
    
    reward_entries
}

// Query interfaces
pub fn get_mining_policy() -> MiningRewardPolicy {
    MINING_REWARD_POLICY.with(|p| {
        p.borrow()
            .get(&"default".to_string())
            .unwrap_or_else(default_mining_config)
    })
}

pub fn get_kappa_tiers() -> Vec<KappaTier> {
    default_kappa_tiers()
}

pub fn get_pending_rewards(principal: Principal) -> Vec<RewardEntry> {
    let mut pending_rewards = Vec::new();
    
    // Get all reward IDs for the user
    let reward_ids = USER_REWARD_INDEX.with(|index| {
        let mut ids = Vec::new();
        for (key, list) in index.borrow().iter() {
            if key.principal_id == principal {
                ids.extend(list.0.clone());
            }
        }
        ids
    });
    
    // Get reward entries
    REWARD_ENTRIES.with(|entries| {
        for id in reward_ids {
            if let Some(entry) = entries.borrow().get(&id) {
                if entry.status == "pending" {
                    pending_rewards.push(entry.clone());
                }
            }
        }
    });
    
    pending_rewards
}

// Get rewards for specific MCP
pub fn get_mcp_rewards(mcp_name: String) -> Vec<RewardEntry> {
    let mut mcp_rewards = Vec::new();
    
    // Get all reward IDs for the MCP
    let reward_ids = MCP_REWARD_INDEX.with(|index| {
        index.borrow()
            .get(&mcp_name)
            .map(|list| list.0)
            .unwrap_or_default()
    });
    
    // Get reward entries
    REWARD_ENTRIES.with(|entries| {
        for id in reward_ids {
            if let Some(entry) = entries.borrow().get(&id) {
                mcp_rewards.push(entry.clone());
            }
        }
    });
    
    mcp_rewards
}

// Calculate quality score based on stake ratio
fn calculate_quality_score(stake_ratio: f32) -> f32 {
    match stake_ratio {
        r if r < 0.01 => 1.0,    // < 1%
        r if r < 0.05 => 1.1,    // 1% - 5%
        r if r < 0.10 => 1.3,    // 5% - 10%
        r if r < 0.25 => 1.5,    // 10% - 25%
        r if r < 0.50 => 1.7,    // 25% - 50%
        r if r < 0.75 => 1.85,   // 50% - 75%
        _ => 2.0,                // > 75%
    }
}

// Predict mining rewards for all pages
pub fn perdic_mining() -> Result<Vec<RewardEntry>, String> {
    let mut all_reward_entries = Vec::new();
    let mut offset = 0u64;
    let limit = 100u64; // Process 100 records per page
    let mut has_more = true;
    
    // Get current quarter
    let current_quarter = 1u32; // This should be calculated based on actual time
    
    while has_more {
        // Get paginated TraceItems
        let trace_items = crate::trace_storage::get_traces_for_mining_days(offset, limit);
        
        if (trace_items.is_empty() || trace_items.len() < limit as usize) {
            has_more = false;
            continue;
        }
        
        // Group by MCP name
        let mut mcp_traces: HashMap<String, Vec<&crate::trace_storage::TraceItem>> = HashMap::new();
        for item in &trace_items {
            mcp_traces.entry(item.agent.clone())
                .or_insert_with(Vec::new)
                .push(item);
        }
        
        // Process rewards for each MCP
        for (mcp_name, traces) in mcp_traces {
            // Get all stack records for this MCP
            let stack_records = get_all_mcp_stack_records(mcp_name.clone());
            
            if stack_records.is_empty() {
                continue;
            }
            
            // Calculate total stake
            let total_stake: u64 = stack_records.iter()
                .filter(|r| matches!(r.stack_status, crate::mcp_asset_types::StackStatus::Stacked))
                .map(|r| r.stack_amount)
                .sum();
            
            if total_stake == 0 {
                continue;
            }
            
            // Get mining policy
            let policy = get_mining_policy();
            let quarter_config = policy.quarters
                .iter()
                .find(|q| q.quarter == current_quarter)
                .ok_or_else(|| format!("Invalid quarter configuration: {}", current_quarter))?;
            
            // Calculate rewards for each stack record
            for stack_record in stack_records {
                if !matches!(stack_record.stack_status, crate::mcp_asset_types::StackStatus::Stacked) {
                    continue;
                }
                
                let stake_ratio = stack_record.stack_amount as f32 / total_stake as f32;
                let quality_score = calculate_quality_score(stake_ratio);
                
                // Calculate reward for each trace
                for trace in &traces {
                    let reward = (quarter_config.base_reward as f32 * quality_score) as u64;
                    
                    let reward_entry = RewardEntry {
                        principal_id: candid::Principal::from_text(&stack_record.principal_id)
                            .unwrap_or_else(|_| candid::Principal::anonymous()),
                        mcp_name: mcp_name.clone(),
                        reward_amount: reward,
                        block_id: trace.timestamp, // Use timestamp as block_id
                        status: "pending".to_string(),
                    };
                    
                    // Store reward record
                    let next_id = REWARD_ENTRIES.with(|entries| {
                        let mut entries = entries.borrow_mut();
                        let next_id = entries.len() as u64;
                        entries.insert(next_id, reward_entry.clone());
                        next_id
                    });
                    
                    // Update user reward index
                    let user_key = UserRewardKey {
                        principal_id: candid::Principal::from_text(&stack_record.principal_id)
                            .unwrap_or_else(|_| candid::Principal::anonymous()),
                        mcp_name: mcp_name.clone(),
                    };
                    
                    USER_REWARD_INDEX.with(|index| {
                        let mut reward_ids = index.borrow()
                            .get(&user_key)
                            .map(|list| list.0)
                            .unwrap_or_default();
                        reward_ids.push(next_id);
                        index.borrow_mut().insert(user_key, RewardIdList(reward_ids));
                    });
                    
                    // Update MCP reward index
                    MCP_REWARD_INDEX.with(|index| {
                        let mut reward_ids = index.borrow()
                            .get(&mcp_name)
                            .map(|list| list.0)
                            .unwrap_or_default();
                        reward_ids.push(next_id);
                        index.borrow_mut().insert(mcp_name.clone(), RewardIdList(reward_ids));
                    });
                    
                    all_reward_entries.push(reward_entry);
                }
            }
        }
        
        offset += limit;
    }
    
    Ok(all_reward_entries)
}

// Get all stack records for a specific MCP
fn get_all_mcp_stack_records(mcp_name: String) -> Vec<crate::mcp_asset_types::McpStackRecord> {
    let mut all_records = Vec::new();
    let mut offset = 0u64;
    let limit = 100u64;
    let mut has_more = true;

    while has_more {
        let records = crate::mcp_asset_types::get_mcp_stack_records_paginated(
            mcp_name.clone(),
            offset,
            limit
        );

        if records.is_empty() {
            has_more = false;
            continue;
        }

        all_records.extend(records);
        offset += limit;
    }

    all_records
}
