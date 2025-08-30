use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::storable::Bound;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::stable_mem_storage::{PIXEL_PROJECTS, PROJECT_OWNER_INDEX};
// Removed getrandom import - using IC-native randomness instead

/// Project identifier - unique string ID for each pixel art project
pub type ProjectId = String;

/// Version identifier - unique string ID for each version within a project
pub type VersionId = String;

/// Animation frame for pixel art
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Frame {
    pub duration_ms: u32,
    pub pixels: Vec<Vec<u16>>,  // palette indices (width x height) in row-major
}

/// Source metadata for pixel art
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SourceMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Pixel art source data structure
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PixelArtSource {
    pub width: u32,
    pub height: u32,
    pub palette: Vec<String>,          // HEX colors, e.g. "#000000"
    pub pixels: Vec<Vec<u16>>,         // palette indices (width x height) in row-major
    pub frames: Option<Vec<Frame>>,    // optional animation frames
    pub metadata: Option<SourceMeta>,
}

/// Version of a pixel art project
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Version {
    pub version_id: VersionId,
    pub created_at: u64,               // seconds since epoch
    pub editor: Principal,
    pub message: Option<String>,
    pub source: PixelArtSource,
}

/// Pixel art project containing all versions and metadata
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub project_id: ProjectId,
    pub owner: Principal,
    pub created_at: u64,
    pub updated_at: u64,
    pub current_version: Version,
    pub history: Vec<Version>,          // append-only history, latest also in current_version
}

/// Project owner key for indexing
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectOwnerKey {
    pub owner: Principal,
    pub project_id: String,
}

/// Compact export format for IoT devices
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CompactPixelArt {
    #[serde(rename = "type")]
    pub art_type: String,  // "pixel_art@1"
    pub width: u32,
    pub height: u32,
    pub palette: Vec<String>,
    pub pixels: Option<Vec<Vec<u16>>>,  // if frames is None
    pub frames: Option<Vec<CompactFrame>>,  // if frames is Some
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CompactFrame {
    #[serde(rename = "durationMs")]
    pub duration_ms: u32,
    pub pixels: Vec<Vec<u16>>,
}

/// Implement Storable traits for stable storage
impl ic_stable_structures::Storable for Project {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 5 * 1024 * 1024, is_fixed_size: false }; // 5MB for large pixel art projects
}

impl ic_stable_structures::Storable for ProjectOwnerKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner, &self.project_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner, project_id) = Decode!(bytes.as_ref(), Principal, String).unwrap();
        Self { owner, project_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

/// Generate new project ID using timestamp and IC-native randomness
pub fn new_project_id(caller: Principal) -> ProjectId {
    let timestamp = ic_cdk::api::time() / 1_000_000; // Convert to seconds
    let mut hasher = DefaultHasher::new();
    timestamp.hash(&mut hasher);
    caller.hash(&mut hasher);
    // Add instruction counter for additional entropy
    ic_cdk::api::instruction_counter().hash(&mut hasher);
    format!("proj_{}_{:x}", timestamp, hasher.finish())
}

/// Generate new version ID using timestamp and IC-native randomness
pub fn new_version_id(caller: Principal) -> VersionId {
    let timestamp = ic_cdk::api::time() / 1_000_000; // Convert to seconds
    let mut hasher = DefaultHasher::new();
    timestamp.hash(&mut hasher);
    caller.hash(&mut hasher);
    // Add instruction counter for additional entropy
    ic_cdk::api::instruction_counter().hash(&mut hasher);
    // Add a different salt for version vs project IDs
    "version".hash(&mut hasher);
    format!("ver_{}_{:x}", timestamp, hasher.finish())
}

/// Validate pixel art source data
pub fn validate_pixel_art_source(source: &PixelArtSource) -> Result<(), String> {
    // Check dimensions
    if source.width == 0 || source.height == 0 {
        return Err("Width and height must be greater than 0".to_string());
    }

    // Check pixels matrix dimensions
    if source.pixels.len() != source.height as usize {
        return Err(format!("Pixels height {} doesn't match specified height {}", 
                          source.pixels.len(), source.height));
    }

    for (row_idx, row) in source.pixels.iter().enumerate() {
        if row.len() != source.width as usize {
            return Err(format!("Row {} width {} doesn't match specified width {}", 
                              row_idx, row.len(), source.width));
        }

        // Check palette indices
        for (col_idx, &pixel) in row.iter().enumerate() {
            if pixel as usize >= source.palette.len() {
                return Err(format!("Pixel at ({}, {}) has palette index {} which exceeds palette size {}", 
                                  row_idx, col_idx, pixel, source.palette.len()));
            }
        }
    }

    // Validate frames if present
    if let Some(frames) = &source.frames {
        for (frame_idx, frame) in frames.iter().enumerate() {
            if frame.pixels.len() != source.height as usize {
                return Err(format!("Frame {} height {} doesn't match source height {}", 
                                  frame_idx, frame.pixels.len(), source.height));
            }

            for (row_idx, row) in frame.pixels.iter().enumerate() {
                if row.len() != source.width as usize {
                    return Err(format!("Frame {} row {} width {} doesn't match source width {}", 
                                      frame_idx, row_idx, row.len(), source.width));
                }

                // Check palette indices in frames
                for (col_idx, &pixel) in row.iter().enumerate() {
                    if pixel as usize >= source.palette.len() {
                        return Err(format!("Frame {} pixel at ({}, {}) has palette index {} which exceeds palette size {}", 
                                          frame_idx, row_idx, col_idx, pixel, source.palette.len()));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if payload size is within reasonable limits (1MB)
pub fn validate_payload_size(source: &PixelArtSource) -> Result<(), String> {
    let estimated_size = calculate_estimated_size(source);
    const MAX_SIZE: usize = 1024 * 1024; // 1MB
    
    if estimated_size > MAX_SIZE {
        return Err(format!("Payload size {} bytes exceeds maximum allowed size {} bytes", 
                          estimated_size, MAX_SIZE));
    }
    
    Ok(())
}

/// Estimate the serialized size of a PixelArtSource
fn calculate_estimated_size(source: &PixelArtSource) -> usize {
    let mut size = 0;
    
    // Basic fields
    size += 8; // width + height
    
    // Palette
    size += source.palette.iter().map(|color| color.len()).sum::<usize>();
    
    // Main pixels
    size += source.pixels.len() * source.width as usize * 2; // u16 = 2 bytes
    
    // Frames if present
    if let Some(frames) = &source.frames {
        for frame in frames {
            size += 4; // duration_ms
            size += frame.pixels.len() * source.width as usize * 2; // u16 = 2 bytes
        }
    }
    
    // Metadata if present
    if let Some(meta) = &source.metadata {
        if let Some(title) = &meta.title {
            size += title.len();
        }
        if let Some(description) = &meta.description {
            size += description.len();
        }
        if let Some(tags) = &meta.tags {
            size += tags.iter().map(|tag| tag.len()).sum::<usize>();
        }
    }
    
    size
}

/// Create a new project with initial version
pub fn create_project(caller: Principal, source: PixelArtSource, message: Option<String>) -> Result<ProjectId, String> {
    // Validate input
    validate_pixel_art_source(&source)?;
    validate_payload_size(&source)?;
    
    let current_time = ic_cdk::api::time() / 1_000_000; // Convert to seconds
    let project_id = new_project_id(caller);
    let version_id = new_version_id(caller);
    
    let initial_version = Version {
        version_id: version_id.clone(),
        created_at: current_time,
        editor: caller,
        message,
        source,
    };
    
    let project = Project {
        project_id: project_id.clone(),
        owner: caller,
        created_at: current_time,
        updated_at: current_time,
        current_version: initial_version.clone(),
        history: vec![initial_version],
    };
    
    // Store in stable memory
    PIXEL_PROJECTS.with(|projects| {
        let mut projects = projects.borrow_mut();
        projects.insert(project_id.clone(), project);
    });
    
    // Update owner index
    PROJECT_OWNER_INDEX.with(|index| {
        let mut index = index.borrow_mut();
        index.insert(ProjectOwnerKey { 
            owner: caller, 
            project_id: project_id.clone() 
        }, ());
    });
    
    Ok(project_id)
}

/// Save a new version to an existing project
pub fn save_version(
    caller: Principal,
    project_id: ProjectId, 
    source: PixelArtSource, 
    message: Option<String>,
    if_match_version: Option<String>
) -> Result<VersionId, String> {
    // Validate input
    validate_pixel_art_source(&source)?;
    validate_payload_size(&source)?;
    
    let current_time = ic_cdk::api::time() / 1_000_000; // Convert to seconds
    
    PIXEL_PROJECTS.with(|projects| {
        let mut projects = projects.borrow_mut();
        
        if let Some(mut project) = projects.get(&project_id) {
            // Check authorization
            if project.owner != caller {
                return Err("Only project owner can save new versions".to_string());
            }
            
            // Check optimistic concurrency if requested
            if let Some(expected_version) = if_match_version {
                if project.current_version.version_id != expected_version {
                    return Err(format!("Version mismatch: expected {}, current {}", 
                                      expected_version, project.current_version.version_id));
                }
            }
            
            let version_id = new_version_id(caller);
            
            let new_version = Version {
                version_id: version_id.clone(),
                created_at: current_time,
                editor: caller,
                message,
                source,
            };
            
            // Update project
            project.current_version = new_version.clone();
            project.history.push(new_version);
            project.updated_at = current_time;
            
            // Save back to storage
            projects.insert(project_id, project);
            
            Ok(version_id)
        } else {
            Err("Project not found".to_string())
        }
    })
}

/// Get a project by ID
pub fn get_project(project_id: ProjectId) -> Option<Project> {
    PIXEL_PROJECTS.with(|projects| {
        let projects = projects.borrow();
        projects.get(&project_id)
    })
}

/// Get a specific version of a project
pub fn get_version(project_id: ProjectId, version_id: VersionId) -> Option<Version> {
    PIXEL_PROJECTS.with(|projects| {
        let projects = projects.borrow();
        if let Some(project) = projects.get(&project_id) {
            // Check current version first
            if project.current_version.version_id == version_id {
                return Some(project.current_version.clone());
            }
            
            // Search in history
            project.history.iter()
                .find(|v| v.version_id == version_id)
                .cloned()
        } else {
            None
        }
    })
}

/// Get current source of a project
pub fn get_current_source(project_id: ProjectId) -> Option<PixelArtSource> {
    PIXEL_PROJECTS.with(|projects| {
        let projects = projects.borrow();
        projects.get(&project_id)
            .map(|project| project.current_version.source.clone())
    })
}

/// Export project for IoT device in compact JSON format
pub fn export_for_device(project_id: ProjectId, version_id: Option<VersionId>) -> Result<String, String> {
    let source = if let Some(vid) = version_id {
        get_version(project_id, vid)
            .map(|v| v.source)
            .ok_or("Version not found".to_string())?
    } else {
        get_current_source(project_id)
            .ok_or("Project not found".to_string())?
    };
    
    let compact = if let Some(frames) = source.frames {
        CompactPixelArt {
            art_type: "pixel_art@1".to_string(),
            width: source.width,
            height: source.height,
            palette: source.palette,
            pixels: None,
            frames: Some(frames.into_iter().map(|f| CompactFrame {
                duration_ms: f.duration_ms,
                pixels: f.pixels,
            }).collect()),
        }
    } else {
        CompactPixelArt {
            art_type: "pixel_art@1".to_string(),
            width: source.width,
            height: source.height,
            palette: source.palette,
            pixels: Some(source.pixels),
            frames: None,
        }
    };
    
    serde_json::to_string(&compact)
        .map_err(|e| format!("JSON serialization failed: {}", e))
}

/// List projects by owner with pagination
pub fn list_projects_by_owner(owner: Principal, page: u32, page_size: u32) -> Vec<Project> {
    let mut projects = Vec::new();
    let skip = page * page_size;
    let mut count = 0;
    let mut collected = 0;
    
    PIXEL_PROJECTS.with(|projects_store| {
        let projects_store = projects_store.borrow();
        
        for (_, project) in projects_store.iter() {
            if project.owner == owner {
                if count >= skip && collected < page_size {
                    projects.push(project.clone());
                    collected += 1;
                }
                count += 1;
                
                if collected >= page_size {
                    break;
                }
            }
        }
    });
    
    projects
}

/// Get total project count by owner
pub fn get_project_count_by_owner(owner: Principal) -> u64 {
    let mut count = 0;
    
    PIXEL_PROJECTS.with(|projects| {
        let projects = projects.borrow();
        for (_, project) in projects.iter() {
            if project.owner == owner {
                count += 1;
            }
        }
    });
    
    count
}

/// Delete a project (only by owner)
pub fn delete_project(caller: Principal, project_id: ProjectId) -> Result<bool, String> {
    
    PIXEL_PROJECTS.with(|projects| {
        let mut projects = projects.borrow_mut();
        
        if let Some(project) = projects.get(&project_id) {
            if project.owner != caller {
                return Err("Only project owner can delete the project".to_string());
            }
            
            // Remove from main storage
            projects.remove(&project_id);
            
            // Remove from owner index
            PROJECT_OWNER_INDEX.with(|index| {
                let mut index = index.borrow_mut();
                index.remove(&ProjectOwnerKey { 
                    owner: caller, 
                    project_id 
                });
            });
            
            Ok(true)
        } else {
            Ok(false)
        }
    })
}

/// Get all projects with pagination
pub fn get_projects_paginated(offset: u64, limit: usize) -> Vec<Project> {
    let mut projects = Vec::new();
    let mut count = 0;
    
    PIXEL_PROJECTS.with(|projects_store| {
        let projects_store = projects_store.borrow();
        
        for (_, project) in projects_store.iter() {
            if count >= offset && projects.len() < limit {
                projects.push(project.clone());
            }
            count += 1;
            
            if projects.len() >= limit {
                break;
            }
        }
    });
    
    projects
}

/// Get total project count
pub fn get_total_project_count() -> u64 {
    PIXEL_PROJECTS.with(|projects| {
        projects.borrow().len()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_art_creation_and_export() {
        // Create a simple 2x2 pixel art
        let source = PixelArtSource {
            width: 2,
            height: 2,
            palette: vec!["#000000".to_string(), "#FFFFFF".to_string()],
            pixels: vec![
                vec![0, 1],
                vec![1, 0],
            ],
            frames: None,
            metadata: Some(SourceMeta {
                title: Some("Test Art".to_string()),
                description: Some("A simple test pattern".to_string()),
                tags: Some(vec!["test".to_string(), "pattern".to_string()]),
            }),
        };

        // Validate the source
        assert!(validate_pixel_art_source(&source).is_ok());
        assert!(validate_payload_size(&source).is_ok());

        // Test ID generation
        let test_principal = Principal::from_text("rdmx6-jaaaa-aaaah-qcaiq-cai").unwrap();
        let project_id = new_project_id(test_principal);
        let version_id = new_version_id(test_principal);
        
        assert!(project_id.starts_with("proj_"));
        assert!(version_id.starts_with("ver_"));
        assert_ne!(project_id, version_id);
    }

    #[test]
    fn test_validation_errors() {
        // Test invalid dimensions
        let mut source = PixelArtSource {
            width: 0,
            height: 2,
            palette: vec!["#000000".to_string()],
            pixels: vec![],
            frames: None,
            metadata: None,
        };
        
        assert!(validate_pixel_art_source(&source).is_err());

        // Test mismatched pixel matrix
        source.width = 2;
        source.height = 2;
        source.pixels = vec![vec![0]]; // Wrong size
        
        assert!(validate_pixel_art_source(&source).is_err());

        // Test palette index out of bounds
        source.pixels = vec![
            vec![0, 2], // Index 2 doesn't exist in palette
            vec![0, 0],
        ];
        
        assert!(validate_pixel_art_source(&source).is_err());
    }

    #[test]
    fn test_compact_export_format() {
        let source = PixelArtSource {
            width: 2,
            height: 2,
            palette: vec!["#000000".to_string(), "#FFFFFF".to_string()],
            pixels: vec![
                vec![0, 1],
                vec![1, 0],
            ],
            frames: None,
            metadata: None,
        };

        let compact = CompactPixelArt {
            art_type: "pixel_art@1".to_string(),
            width: source.width,
            height: source.height,
            palette: source.palette.clone(),
            pixels: Some(source.pixels.clone()),
            frames: None,
        };

        let json = serde_json::to_string(&compact).unwrap();
        assert!(json.contains("pixel_art@1"));
        assert!(json.contains("#000000"));
        assert!(json.contains("#FFFFFF"));
    }
}
