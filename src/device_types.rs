use candid::{CandidType, Deserialize};
use ic_stable_structures::{Storable, storable::Bound};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::BTreeMap;
use candid::Principal;

/// Device information structure
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeviceInfo {
    pub id: String,                    // Device unique identifier
    pub name: String,                  // Device name
    pub device_type: DeviceType,       // Device type
    pub owner: Principal,              // Device owner
    pub status: DeviceStatus,          // Device status
    pub capabilities: Vec<DeviceCapability>, // Device capabilities
    pub metadata: BTreeMap<String, String>, // Device metadata
    pub created_at: u64,               // Creation timestamp
    pub updated_at: u64,               // Update timestamp
    pub last_seen: u64,                // Last seen timestamp
}

/// Device type enumeration
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DeviceType {
    Mobile,        // Mobile device
    Desktop,       // Desktop device
    Server,        // Server
    IoT,           // Internet of Things device
    Embedded,      // Embedded device
    Other(String), // Other type
}

/// Device status enumeration
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DeviceStatus {
    Online,        // Online
    Offline,       // Offline
    Maintenance,   // Under maintenance
    Disabled,      // Disabled
}

/// Device capability enumeration
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DeviceCapability {
    Audio,         // Audio processing
    Video,         // Video processing
    Storage,       // Storage
    Network,       // Network communication
    Compute,       // Computing capability
    Sensor,        // Sensor
    Custom(String), // Custom capability
}

/// Device owner key
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceOwnerKey {
    pub owner: Principal,
    pub device_id: String,
}

/// Device ID key
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceIdKey {
    pub device_id: String,
}

/// Device query filter
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DeviceFilter {
    pub owner: Option<Principal>,
    pub device_type: Option<DeviceType>,
    pub status: Option<DeviceStatus>,
    pub capability: Option<DeviceCapability>,
}

/// Device list response
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceInfo>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

// Implement Storable trait for DeviceInfo
impl Storable for DeviceInfo {
    const BOUND: Bound = Bound::Unbounded;
    
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize DeviceInfo");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize DeviceInfo")
    }
}

// Implement Storable trait for DeviceOwnerKey
impl Storable for DeviceOwnerKey {
    const BOUND: Bound = Bound::Unbounded;
    
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize DeviceOwnerKey");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize DeviceOwnerKey")
    }
}

// Implement Storable trait for DeviceIdKey
impl Storable for DeviceIdKey {
    const BOUND: Bound = Bound::Unbounded;
    
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize DeviceIdKey");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize DeviceIdKey")
    }
}

/// Device management service
pub struct DeviceService;

impl DeviceService {
    /// Add a new device
    pub fn add_device(device_info: DeviceInfo) -> Result<u64, String> {
        use crate::stable_mem_storage::{DEVICES, DEVICE_OWNER_INDEX, DEVICE_ID_INDEX};
        
        // Check if device ID already exists
        let device_id_key = DeviceIdKey {
            device_id: device_info.id.clone(),
        };
        
        if DEVICE_ID_INDEX.with(|index| {
            index.borrow().contains_key(&device_id_key)
        }) {
            return Err("Device ID already exists".to_string());
        }

        // Add device to storage
        DEVICES.with(|devices| {
            devices.borrow_mut().push(&device_info)
        }).map_err(|_| "Failed to add device")?;

        // Get device index (length - 1 after push)
        let device_index = DEVICES.with(|devices| {
            devices.borrow().len() - 1
        });

        // Update indices
        let owner_key = DeviceOwnerKey {
            owner: device_info.owner,
            device_id: device_info.id.clone(),
        };

        DEVICE_OWNER_INDEX.with(|index| {
            index.borrow_mut().insert(owner_key, device_index);
        });

        DEVICE_ID_INDEX.with(|index| {
            index.borrow_mut().insert(device_id_key, device_index);
        });

        Ok(device_index)
    }

    /// Get device information by device ID
    pub fn get_device_by_id(device_id: &str) -> Option<DeviceInfo> {
        use crate::stable_mem_storage::{DEVICES, DEVICE_ID_INDEX};
        
        let device_id_key = DeviceIdKey {
            device_id: device_id.to_string(),
        };

        let device_index = DEVICE_ID_INDEX.with(|index| {
            index.borrow().get(&device_id_key)
        })?;

        DEVICES.with(|devices| {
            devices.borrow().get(device_index)
        })
    }

    /// Get device list by owner
    pub fn get_devices_by_owner(owner: &Principal) -> Vec<DeviceInfo> {
        use crate::stable_mem_storage::{DEVICES, DEVICE_OWNER_INDEX};
        
        let mut devices = Vec::new();
        
        DEVICE_OWNER_INDEX.with(|index| {
            let index_ref = index.borrow();
            for (key, device_index) in index_ref.iter() {
                if key.owner == *owner {
                    if let Some(device) = DEVICES.with(|devices| {
                        devices.borrow().get(device_index)
                    }) {
                        devices.push(device);
                    }
                }
            }
        });

        devices
    }

    /// Update device information
    pub fn update_device(device_id: &str, updated_device: DeviceInfo) -> Result<(), String> {
        use crate::stable_mem_storage::{DEVICES, DEVICE_ID_INDEX, DEVICE_OWNER_INDEX};
        
        let device_id_key = DeviceIdKey {
            device_id: device_id.to_string(),
        };

        let device_index = DEVICE_ID_INDEX.with(|index| {
            index.borrow().get(&device_id_key)
        }).ok_or("Device not found")?;

        // Validate device ID matches
        if updated_device.id != device_id {
            return Err("Device ID mismatch".to_string());
        }

        // Update device information
        DEVICES.with(|devices| {
            devices.borrow_mut().set(device_index, &updated_device)
        });

        // If owner changes, update owner index
        let old_device = DEVICES.with(|devices| {
            devices.borrow().get(device_index)
        }).ok_or("Failed to get old device info")?;

        if old_device.owner != updated_device.owner {
            // Remove old index entry
            let old_owner_key = DeviceOwnerKey {
                owner: old_device.owner,
                device_id: device_id.to_string(),
            };
            DEVICE_OWNER_INDEX.with(|index| {
                index.borrow_mut().remove(&old_owner_key);
            });

            // Add new index entry
            let new_owner_key = DeviceOwnerKey {
                owner: updated_device.owner,
                device_id: device_id.to_string(),
            };
            DEVICE_OWNER_INDEX.with(|index| {
                index.borrow_mut().insert(new_owner_key, device_index);
            });
        }

        Ok(())
    }

    /// Delete device
    pub fn delete_device(device_id: &str) -> Result<(), String> {
        use crate::stable_mem_storage::{DEVICES, DEVICE_ID_INDEX, DEVICE_OWNER_INDEX};
        
        let device_id_key = DeviceIdKey {
            device_id: device_id.to_string(),
        };

        let device_index = DEVICE_ID_INDEX.with(|index| {
            index.borrow().get(&device_id_key)
        }).ok_or("Device not found")?;

        // Get device info to get owner
        let device = DEVICES.with(|devices| {
            devices.borrow().get(device_index)
        }).ok_or("Failed to get device info")?;

        // Remove index entries
        DEVICE_ID_INDEX.with(|index| {
            index.borrow_mut().remove(&device_id_key);
        });

        let owner_key = DeviceOwnerKey {
            owner: device.owner,
            device_id: device_id.to_string(),
        };
        DEVICE_OWNER_INDEX.with(|index| {
            index.borrow_mut().remove(&owner_key);
        });

        // Note: This doesn't actually delete device data, just marks as deleted
        // In real applications, soft delete mechanism may be needed
        Ok(())
    }

    /// Get all devices (paginated)
    pub fn get_all_devices(offset: u64, limit: u64) -> DeviceListResponse {
        use crate::stable_mem_storage::DEVICES;
        
        let mut devices = Vec::new();
        let total = DEVICES.with(|devices_storage| {
            devices_storage.borrow().len()
        });

        let start = offset as usize;
        let end = std::cmp::min(start + limit as usize, total as usize);

        for i in start..end {
            if let Some(device) = DEVICES.with(|devices_storage| {
                devices_storage.borrow().get(i as u64)
            }) {
                devices.push(device);
            }
        }

        DeviceListResponse {
            devices,
            total,
            offset,
            limit,
        }
    }

    /// Search devices by filter
    pub fn search_devices(filter: DeviceFilter) -> Vec<DeviceInfo> {
        use crate::stable_mem_storage::DEVICES;
        
        let mut devices = Vec::new();
        
        DEVICES.with(|devices_storage| {
            let devices_ref = devices_storage.borrow();
            for i in 0..devices_ref.len() {
                if let Some(device) = devices_ref.get(i) {
                    let mut matches = true;

                    // Check owner filter
                    if let Some(ref owner) = filter.owner {
                        if device.owner != *owner {
                            matches = false;
                        }
                    }

                    // Check device type filter
                    if let Some(ref device_type) = filter.device_type {
                        if device.device_type != *device_type {
                            matches = false;
                        }
                    }

                    // Check status filter
                    if let Some(ref status) = filter.status {
                        if device.status != *status {
                            matches = false;
                        }
                    }

                    // Check capability filter
                    if let Some(ref capability) = filter.capability {
                        if !device.capabilities.contains(capability) {
                            matches = false;
                        }
                    }

                    if matches {
                        devices.push(device);
                    }
                }
            }
        });

        devices
    }

    /// Update device status
    pub fn update_device_status(device_id: &str, status: DeviceStatus) -> Result<(), String> {
        if let Some(mut device) = Self::get_device_by_id(device_id) {
            device.status = status;
            device.updated_at = ic_cdk::api::time();
            Self::update_device(device_id, device)
        } else {
            Err("Device not found".to_string())
        }
    }

    /// Update device last seen timestamp
    pub fn update_last_seen(device_id: &str) -> Result<(), String> {
        if let Some(mut device) = Self::get_device_by_id(device_id) {
            device.last_seen = ic_cdk::api::time();
            device.status = DeviceStatus::Online;
            Self::update_device(device_id, device)
        } else {
            Err("Device not found".to_string())
        }
    }
}
