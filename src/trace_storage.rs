use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use icrc_ledger_types::icrc1::account::Account;
use candid::Principal;

// Constants for time calculations
const SECS_PER_MIN: u64 = 60;
const SECS_PER_HOUR: u64 = 60 * SECS_PER_MIN;
const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;
const DAYS_PER_YEAR: u64 = 365;
const DAYS_PER_MONTH: [u64; 12] = [31,28,31,30,31,30,31,31,30,31,30,31];

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CallItem {
    pub id: u64,
    pub protocol: String,
    pub agent: String,
    pub call_type: String,
    pub method: String,
    pub inputs: Vec<IOData>,
    pub outputs: Vec<IOData>,
    pub status: String,
}

impl ic_stable_structures::Storable for CallItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode CallItem"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode CallItem")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct IOData {
    pub data_type: String,
    pub value: String,
}

impl ic_stable_structures::Storable for IOData {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode IOData"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode IOData")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TraceItem {
    pub context_id: String,
    pub trace_id: String,
    pub owner: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub calls: Vec<CallItem>,
    pub metadata: Option<String>,
}

impl ic_stable_structures::Storable for TraceItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode TraceItem"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode TraceItem")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 64, is_fixed_size: false };
}

/// Format a UNIX timestamp (seconds since epoch) into a string.
/// Supported formats: "YYYY-MM-DD", "YYYY-MM", "YYYY-MM-DD-HH", "YYYY", "%V" (ISO week), "%A" (weekday), "%B" (month name), "%H" (hour)
fn format_time(timestamp: u64, fmt: &str) -> String {
    // Basic calculation for UTC time
    let mut secs = timestamp;
    let mut year = 1970;
    let mut month = 1;
    let mut day = 1;
    let mut hour = 0;
    let mut min = 0;
    let mut sec = 0;
    
    // Calculate year
    let mut days = secs / SECS_PER_DAY;
    secs %= SECS_PER_DAY;
    while days >= if is_leap_year(year) { 366 } else { 365 } {
        days -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }
    
    // Calculate month
    let mut month_days = DAYS_PER_MONTH;
    if is_leap_year(year) { month_days[1] = 29; }
    for (i, &mdays) in month_days.iter().enumerate() {
        if days + 1 > mdays {
            days -= mdays;
            month += 1;
        } else {
            break;
        }
    }
    
    day += days as u32;
    hour = (secs / SECS_PER_HOUR) as u32;
    secs %= SECS_PER_HOUR;
    min = (secs / SECS_PER_MIN) as u32;
    sec = (secs % SECS_PER_MIN) as u32;
    
    match fmt {
        "%Y-%m-%d" => format!("{:04}-{:02}-{:02}", year, month, day),
        "%Y-%m" => format!("{:04}-{:02}", year, month),
        "%Y" => format!("{:04}", year),
        "%Y-%m-%d-%H" => format!("{:04}-{:02}-{:02}-{:02}", year, month, day, hour),
        "%H" => format!("{:02}", hour),
        "%A" => weekday_name(year, month, day),
        "%B" => month_name(month),
        "%V" => iso_week_number(year, month, day).to_string(),
        _ => format!("{:04}-{:02}-{:02}", year, month, day),
    }
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn month_name(month: u32) -> String {
    match month {
        1 => "January", 2 => "February", 3 => "March", 4 => "April", 5 => "May", 6 => "June",
        7 => "July", 8 => "August", 9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "Unknown"
    }.to_string()
}

fn weekday_name(year: u64, month: u32, day: u32) -> String {
    // Zeller's congruence
    let (y, m) = if month < 3 { (year - 1, month + 12) } else { (year, month) };
    let k = y % 100;
    let j = y / 100;
    let h = (day as u64 + (13 * (m as u64 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    match h {
        0 => "Saturday", 1 => "Sunday", 2 => "Monday", 3 => "Tuesday", 4 => "Wednesday", 5 => "Thursday", 6 => "Friday", _ => "Unknown"
    }.to_string()
}

fn iso_week_number(year: u64, month: u32, day: u32) -> u32 {
    // Simple ISO week calculation (not 100% accurate for all edge cases)
    let y = year as i32;
    let m = month as i32;
    let d = day as i32;
    let mut a = (14 - m) / 12;
    let y = y - a;
    let m = m + 12 * a - 2;
    let dow = (d + y + y/4 - y/100 + y/400 + (31*m)/12) % 7;
    let mut doy = d;
    for i in 1..m { doy += DAYS_PER_MONTH[(i-1) as usize] as i32; }
    ((doy - dow + 10) / 7) as u32
}

fn transfer_status_from_str(s: &str) -> TransferStatus {
    match s {
        "Completed" => TransferStatus::Completed,
        "Failed" => TransferStatus::Failed,
        _ => TransferStatus::Pending,
    }
}

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Define the key for trace data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceKey {
    pub id: String,
}

impl ic_stable_structures::Storable for TraceKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.id).expect("Failed to encode TraceKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let id = Decode!(bytes.as_ref(), String).expect("Failed to decode TraceKey");
        Self { id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static TRACES: RefCell<StableBTreeMap<TraceKey, TraceItem, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(11)))
        )
    );
}

/// Add or update a trace item
pub fn upsert_trace(trace: TraceItem) -> Result<(), String> {
    TRACES.with(|traces| {
        let mut traces = traces.borrow_mut();
        let key = TraceKey { id: trace.trace_id.clone() };
        traces.insert(key, trace);
        Ok(())
    })
}

/// Get a trace item by ID
pub fn get_trace(id: String) -> Option<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        let key = TraceKey { id };
        traces.get(&key).map(|trace| trace.clone())
    })
}

/// Get all traces for a specific owner
pub fn get_owner_traces(owner: String) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner)
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get all traces
pub fn get_all_traces() -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter().map(|(_, trace)| trace).collect()
    })
}

/// Get traces with pagination
pub fn get_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces
            .iter()
            .skip(offset as usize)
            .take(limit)
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get owner traces with pagination
pub fn get_owner_traces_paginated(owner: String, offset: u64, limit: usize) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner)
            .map(|(_, trace)| trace)
            .skip(offset as usize)
            .take(limit)
            .collect()
    })
}

/// Delete a trace
pub fn delete_trace(id: String) -> Result<(), String> {
    TRACES.with(|traces| {
        let mut traces = traces.borrow_mut();
        let key = TraceKey { id };
        if traces.remove(&key).is_some() {
            Ok(())
        } else {
            Err("Trace not found".to_string())
        }
    })
}

/// Get traces by operation type
pub fn get_traces_by_operation(owner: String, operation: String) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner && trace.calls.get(0).map_or(false, |call| call.method == operation))
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get traces by status
pub fn get_traces_by_status(owner: String, status: TransferStatus) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner && trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", status)))
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get traces by time period
pub fn get_traces_by_time_period(owner: String, time_period: String) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner)
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get traces with sorting
pub fn get_traces_sorted(owner: String, sort_by: String, ascending: bool) -> Vec<TraceItem> {
    let mut traces = get_owner_traces(owner);
    
    match sort_by.as_str() {
        "amount" => {
            traces.sort_by(|a, b| {
                let a_amount = a.calls.get(0).map_or(0, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0));
                let b_amount = b.calls.get(0).map_or(0, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0));
                if ascending {
                    a_amount.cmp(&b_amount)
                } else {
                    b_amount.cmp(&a_amount)
                }
            });
        },
        "time" => {
            traces.sort_by(|a, b| {
                if ascending {
                    a.created_at.cmp(&b.created_at)
                } else {
                    b.created_at.cmp(&a.created_at)
                }
            });
        },
        _ => {
            traces.sort_by(|a, b| {
                if ascending {
                    a.created_at.cmp(&b.created_at)
                } else {
                    b.created_at.cmp(&a.created_at)
                }
            });
        }
    }
    
    traces
}

/// Get traces with filters
pub fn get_traces_with_filters(
    owner: String,
    operations: Option<Vec<String>>,
    statuses: Option<Vec<TransferStatus>>,
    start_time: Option<u64>,
    end_time: Option<u64>,
    min_amount: Option<u128>,
    max_amount: Option<u128>,
    accounts: Option<Vec<Account>>
) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| {
                let matches_owner = trace.owner == owner;
                let matches_operation = operations.as_ref()
                    .map(|ops| ops.iter().any(|op| trace.calls.get(0).map_or(false, |call| call.method == *op)))
                    .unwrap_or(true);
                let matches_status = statuses.as_ref()
                    .map(|statuses| statuses.contains(&trace.calls.get(0).map_or(TransferStatus::Pending, |call| {
                        transfer_status_from_str(&call.status)
                    })))
                    .unwrap_or(true);
                let matches_time = start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true);
                let matches_amount = min_amount.as_ref()
                    .map(|min| trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)) >= *min as u64)
                    .unwrap_or(true)
                    && max_amount.as_ref()
                    .map(|max| trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)) <= *max as u64)
                    .unwrap_or(true);
                let matches_account = accounts.as_ref()
                    .map(|accounts| accounts.contains(&trace.calls.get(0).map_or(Account { owner: Principal::anonymous(), subaccount: None }, |call| {
                        Account {
                            owner: Principal::from_text(&call.agent).unwrap_or(Principal::anonymous()),
                            subaccount: None,
                        }
                    })))
                    .unwrap_or(true);

                matches_owner && matches_operation && matches_status && matches_time && matches_amount && matches_account
            })
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get trace statistics
pub fn get_traces_statistics(owner: String, start_time: Option<u64>, end_time: Option<u64>) -> (u64, u128, u128, u128) {
    let traces = get_owner_traces(owner);
    let filtered_traces: Vec<&TraceItem> = traces.iter()
        .filter(|trace| {
            start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
        })
        .collect();

    let total_count = filtered_traces.len() as u64;
    let total_amount: u128 = filtered_traces.iter()
        .map(|trace| trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)) as u128)
        .sum();
    let success_amount: u128 = filtered_traces.iter()
        .filter(|trace| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Completed)))
        .map(|trace| trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)) as u128)
        .sum();
    let failed_amount: u128 = filtered_traces.iter()
        .filter(|trace| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Failed)))
        .map(|trace| trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)) as u128)
        .sum();

    (total_count, total_amount, success_amount, failed_amount)
} 