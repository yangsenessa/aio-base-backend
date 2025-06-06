use candid::{CandidType, Deserialize};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, Storable};
use ic_stable_structures::storable::Bound;
use crate::stable_mem_storage::TRACE_STORAGE;
use std::cell::RefCell;
use std::borrow::Cow;
use std::cmp::Ordering;

const TRACE_BUFFER_SIZE: usize = 100;

#[derive(CandidType, Deserialize, Clone)]
pub struct IOValue {
    pub data_type: String,
    pub value: IOValueType,
}

#[derive(CandidType, Deserialize, Clone)]
pub enum IOValueType {
    Text(String),
    Number(f64),
    Boolean(bool),
    Object(String),
    Array(String),
    Null,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct ProtocolCall {
    pub id: u32,
    pub protocol: String,
    pub agent: String,
    pub call_type: String,
    pub method: String,
    pub input: IOValue,
    pub output: IOValue,
    pub status: String,
    pub error_message: Option<String>,
    pub timestamp: u64,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct TraceLog {
    pub trace_id: String,
    pub context_id: String,
    pub calls: Vec<ProtocolCall>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct TraceStatistics {
    pub total_count: u64,
    pub success_count: u64,
    pub error_count: u64,
}

#[derive(CandidType, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct TraceKey {
    pub trace_id: String,
}

impl Ord for TraceKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.trace_id.cmp(&other.trace_id)
    }
}

impl PartialOrd for TraceKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(CandidType, Deserialize, Clone)]
pub struct TraceItem {
    pub trace_id: String,
    pub context_id: String,
    pub protocol: String,
    pub agent: String,
    pub call_type: String,
    pub method: String,
    pub input: IOValue,
    pub output: IOValue,
    pub status: String,
    pub error_message: Option<String>,
    pub timestamp: u64,
}

type Memory = VirtualMemory<DefaultMemoryImpl>;

impl Storable for IOValue {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

impl Storable for IOValueType {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

impl Storable for ProtocolCall {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

impl Storable for TraceLog {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

impl Storable for TraceKey {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

impl Storable for TraceItem {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(candid::encode_one(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }
}

pub fn record_trace_call(
    trace_id: String,
    context_id: String,
    protocol: String,
    agent: String,
    call_type: String,
    method: String,
    input: IOValue,
    output: IOValue,
    status: String,
    error_message: Option<String>,
) -> Result<(), String> {
    TRACE_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        let mut trace_log = storage.get(&trace_id).unwrap_or_else(|| TraceLog {
            trace_id: trace_id.clone(),
            context_id,
            calls: Vec::new(),
        });

        let call = ProtocolCall {
            id: trace_log.calls.len() as u32 + 1,
            protocol,
            agent,
            call_type,
            method,
            input,
            output,
            status,
            error_message,
            timestamp: ic_cdk::api::time(),
        };

        trace_log.calls.push(call);

        // Trim buffer if it exceeds maximum size
        if trace_log.calls.len() > TRACE_BUFFER_SIZE {
            trace_log.calls.drain(0..trace_log.calls.len() - TRACE_BUFFER_SIZE);
        }

        storage.insert(trace_id, trace_log);
        Ok(())
    })
}

pub fn get_trace_by_id(trace_id: String) -> Option<TraceLog> {
    TRACE_STORAGE.with(|storage| storage.borrow().get(&trace_id))
}

pub fn get_trace_by_context_id(context_id: String) -> Option<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .find(|(_, trace)| trace.context_id == context_id)
            .map(|(_, trace)| trace.clone())
    })
}

pub fn get_all_trace_logs() -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_paginated(offset: u64, limit: u64) -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_by_protocol_name(protocol: String) -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| call.protocol == protocol)
            })
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_by_method_name(method: String) -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| call.method == method)
            })
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_by_status(status: String, offset: u64, limit: u64) -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| call.status == status)
            })
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_with_filters(
    protocols: Vec<String>,
    methods: Vec<String>,
    statuses: Vec<String>,
    owners: Vec<String>,
    time_ranges: Vec<(u64, u64)>,
    amount_ranges: Vec<(u64, u64)>,
    status_ranges: Vec<String>,
    limit: u64,
) -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| {
                    (protocols.is_empty() || protocols.contains(&call.protocol))
                        && (methods.is_empty() || methods.contains(&call.method))
                        && (statuses.is_empty() || statuses.contains(&call.status))
                })
            })
            .take(limit as usize)
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_statistics(
    start_time: u64,
    end_time: u64,
    limit: u64,
) -> TraceStatistics {
    TRACE_STORAGE.with(|storage| {
        let mut total_count = 0u64;
        let mut success_count = 0u64;
        let mut error_count = 0u64;

        for (_, trace) in storage.borrow().iter() {
            for call in &trace.calls {
                if call.status == "ok" {
                    success_count += 1;
                } else {
                    error_count += 1;
                }
                total_count += 1;
            }
        }

        TraceStatistics {
            total_count,
            success_count,
            error_count,
        }
    })
}

pub fn get_traces_by_operation(principal_id: String, operation: String) -> Vec<TraceItem> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| {
                    call.agent == principal_id && call.method == operation
                })
            })
            .map(|(_, trace)| {
                trace.calls.iter().map(|call| {
                    TraceItem {
                        trace_id: trace.trace_id.clone(),
                        context_id: trace.context_id.clone(),
                        protocol: call.protocol.clone(),
                        agent: call.agent.clone(),
                        call_type: call.call_type.clone(),
                        method: call.method.clone(),
                        input: call.input.clone(),
                        output: call.output.clone(),
                        status: call.status.clone(),
                        error_message: call.error_message.clone(),
                        timestamp: ic_cdk::api::time(),
                    }
                }).collect::<Vec<TraceItem>>()
            })
            .flatten()
            .collect()
    })
}

pub fn get_traces_sorted(principal_id: String, sort_by: String, ascending: bool) -> Vec<TraceItem> {
    let mut traces = get_traces_by_operation(principal_id, "all".to_string());
    
    traces.sort_by(|a, b| {
        let comparison = match sort_by.as_str() {
            "timestamp" => a.timestamp.cmp(&b.timestamp),
            "method" => a.method.cmp(&b.method),
            "status" => a.status.cmp(&b.status),
            _ => a.timestamp.cmp(&b.timestamp),
        };
        
        if ascending {
            comparison
        } else {
            comparison.reverse()
        }
    });
    
    traces
}

pub fn get_traces_by_time_period(principal_id: String, time_period: String) -> Vec<TraceItem> {
    let current_time = ic_cdk::api::time();
    let period_seconds = match time_period.as_str() {
        "day" => 24 * 60 * 60 * 1_000_000_000,
        "week" => 7 * 24 * 60 * 60 * 1_000_000_000,
        "month" => 30 * 24 * 60 * 60 * 1_000_000_000,
        "year" => 365 * 24 * 60 * 60 * 1_000_000_000,
        _ => return Vec::new(),
    };
    
    let start_time = current_time - period_seconds;
    
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| {
                    call.agent == principal_id && call.timestamp >= start_time
                })
            })
            .map(|(_, trace)| {
                trace.calls.iter().map(|call| {
                    TraceItem {
                        trace_id: trace.trace_id.clone(),
                        context_id: trace.context_id.clone(),
                        protocol: call.protocol.clone(),
                        agent: call.agent.clone(),
                        call_type: call.call_type.clone(),
                        method: call.method.clone(),
                        input: call.input.clone(),
                        output: call.output.clone(),
                        status: call.status.clone(),
                        error_message: call.error_message.clone(),
                        timestamp: call.timestamp,
                    }
                }).collect::<Vec<TraceItem>>()
            })
            .flatten()
            .collect()
    })
}

pub fn get_traces_by_agentname_paginated(agent_name: String, offset: u64, limit: u64) -> Vec<TraceLog> {
    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| call.agent == agent_name)
            })
            .skip(offset as usize)
            .take(limit as usize)
            .map(|(_, trace)| trace.clone())
            .collect()
    })
}

pub fn get_traces_for_mining_days(offset: u64, limit: u64) -> Vec<TraceItem> {
    let current_time = ic_cdk::api::time();
    let day_seconds = 24 * 60 * 60 * 1_000_000_000; // 一天的纳秒数
    let start_time = current_time - day_seconds;

    TRACE_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .filter(|(_, trace)| {
                trace.calls.iter().any(|call| 
                    call.status == "ok" && call.timestamp >= start_time
                )
            })
            .map(|(_, trace)| {
                trace.calls.iter()
                    .filter(|call| call.status == "ok" && call.timestamp >= start_time)
                    .map(|call| {
                        TraceItem {
                            trace_id: trace.trace_id.clone(),
                            context_id: trace.context_id.clone(),
                            protocol: call.protocol.clone(),
                            agent: call.agent.clone(),
                            call_type: call.call_type.clone(),
                            method: call.method.clone(),
                            input: call.input.clone(),
                            output: call.output.clone(),
                            status: call.status.clone(),
                            error_message: call.error_message.clone(),
                            timestamp: call.timestamp,
                        }
                    }).collect::<Vec<TraceItem>>()
            })
            .flatten()
            .skip(offset as usize)
            .take(limit as usize)
            .collect()
    })
} 

