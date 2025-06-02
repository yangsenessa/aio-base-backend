use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use serde_json::Value;
use crate::stable_mem_storage::{AIO_INDICES, KEYWORD_INDEX};

type Memory = VirtualMemory<DefaultMemoryImpl>;

/// A wrapper around Vec<String> that implements Storable
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StringVec(pub Vec<String>);

impl Storable for StringVec {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.0).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Self(Decode!(bytes.as_ref(), Vec<String>).unwrap())
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 64, is_fixed_size: false };
}

/// Property in schema
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SchemaProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaProperty>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<SchemaProperty>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Method parameter schema definition
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, Box<SchemaProperty>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Method definition
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Method {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_params: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<InputSchema>,
}

/// Source information
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Source {
    pub author: String,
    pub version: String,
    pub github: String,
}

/// AioIndex represents an index item in the system
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AioIndex {
    pub id: String,           // agent/mcp name
    pub description: String,
    pub transport: Vec<String>,
    pub methods: Vec<Method>,
    pub source: Source,
    pub keywords: Vec<String>,
    pub scenarios: Vec<String>,
}

impl Default for AioIndex {
    fn default() -> Self {
        Self {
            id: String::new(),
            description: String::new(),
            transport: Vec::new(),
            methods: Vec::new(),
            source: Source {
                author: String::new(),
                version: String::new(),
                github: String::new(),
            },
            keywords: Vec::new(),
            scenarios: Vec::new(),
        }
    }
}

// Implement Storable for AioIndex
impl ic_stable_structures::Storable for AioIndex {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 128, is_fixed_size: false };
}

/// Manager for AioIndex storage and operations
pub struct AioIndexManager;

impl AioIndexManager {
    /// Create a new instance of the manager
    pub fn new() -> Self {
        Self {}
    }

    /// Create a new AioIndex
    pub fn create(&self, index: AioIndex) -> Result<(), String> {
        let id = index.id.clone();
        
        AIO_INDICES.with(|indices| {
            let mut indices = indices.borrow_mut();
            
            if indices.contains_key(&id) {
                return Err(format!("Index with ID {} already exists", id));
            }
            // Log the index being created
            ic_cdk::println!("Creating new AioIndex: id={}, description={}, keywords={:?}", 
                id, index.description, index.keywords);
            indices.insert(id.clone(), index.clone());
            
            // Add to keyword indices
            for keyword in &index.keywords {
                self.add_to_keyword_index(keyword, &id);
            }
            
            Ok(())
        })
    }

    /// Helper function to add an index to the keyword index
    fn add_to_keyword_index(&self, keyword: &str, id: &str) {
        KEYWORD_INDEX.with(|keyword_index| {
            let mut keyword_index = keyword_index.borrow_mut();
            let keyword_lower = keyword.to_lowercase();
            
            let mut index_list = keyword_index
                .get(&keyword_lower)
                .map(|v| v.0.clone())
                .unwrap_or_default();
            
            if !index_list.contains(&id.to_string()) {
                index_list.push(id.to_string());
                keyword_index.insert(keyword_lower, StringVec(index_list));
            }
        });
    }
    pub fn read(&self, id: &str) -> Option<AioIndex> {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.get(&id.to_string())
        })
    }

    /// Update an existing AioIndex
    pub fn update(&self, id: &str, updated_index: AioIndex) -> Result<(), String> {
        AIO_INDICES.with(|indices| {
            let mut indices = indices.borrow_mut();
            
            if !indices.contains_key(&id.to_string()) {
                return Err(format!("Index with ID {} does not exist", id));
            }
            
            // Get the old index to update keyword references
            if let Some(old_index) = indices.get(&id.to_string()) {
                // Remove from old keywords
                for keyword in &old_index.keywords {
                    self.remove_from_keyword_index(keyword, id);
                }
            }
            
            // Add to new keywords
            for keyword in &updated_index.keywords {
                self.add_to_keyword_index(keyword, id);
            }
            
            // Update the index
            indices.insert(id.to_string(), updated_index);
            Ok(())
        })
    }

    /// Delete an AioIndex by ID
    pub fn delete(&self, id: &str) -> Result<(), String> {
        AIO_INDICES.with(|indices| {
            let mut indices = indices.borrow_mut();
            
            if !indices.contains_key(&id.to_string()) {
                return Err(format!("Index with ID {} does not exist", id));
            }
            
            // Get the index to remove its keywords
            if let Some(index) = indices.get(&id.to_string()) {
                // Remove from keyword indices
                for keyword in &index.keywords {
                    self.remove_from_keyword_index(keyword, id);
                }
            }
            
            // Remove the index
            indices.remove(&id.to_string());
            Ok(())
        })
    }

    /// Helper function to remove an index from the keyword index
    fn remove_from_keyword_index(&self, keyword: &str, id: &str) {
        KEYWORD_INDEX.with(|keyword_index| {
            let mut keyword_index = keyword_index.borrow_mut();
            let keyword_lower = keyword.to_lowercase();
            
            if let Some(index_list) = keyword_index.get(&keyword_lower) {
                let mut new_list = index_list.0.clone();
                new_list.retain(|item_id| item_id != id);
                
                if new_list.is_empty() {
                    keyword_index.remove(&keyword_lower);
                } else {
                    keyword_index.insert(keyword_lower, StringVec(new_list));
                }
            }
        });
    }

    /// List all indices
    pub fn list_all(&self) -> Vec<AioIndex> {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.iter().map(|(_, value)| value).collect()
        })
    }

    /// Get indices with pagination
    pub fn get_indices_paginated(&self, offset: usize, limit: usize) -> Vec<AioIndex> {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.iter()
                .skip(offset)
                .take(limit)
                .map(|(_, value)| value)
                .collect()
        })
    }

    /// Helper function to recursively parse a SchemaProperty from JSON
    fn parse_schema_property(value: &Value) -> Option<Box<SchemaProperty>> {
        let obj = value.as_object()?;
        
        let prop_type = obj.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("string").to_string();
        
        let description = obj.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let default = obj.get("default")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let enum_values = obj.get("enum")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            });
        
        let items = obj.get("items")
            .and_then(|v| Self::parse_schema_property(v));
        
        let properties = obj.get("properties")
            .and_then(|v| v.as_object())
            .map(|props| {
                props.iter()
                    .filter_map(|(k, v)| {
                        Some((k.clone(), Self::parse_schema_property(v)?))
                    })
                    .collect::<HashMap<String, Box<SchemaProperty>>>()
            });
        
        let required = obj.get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            });
        
        Some(Box::new(SchemaProperty {
            property_type: prop_type,
            description,
            default,
            enum_values,
            items,
            properties,
            required,
        }))
    }

    pub fn create_from_json(&self, name: &str, json_str: &str) -> Result<(), String> {
        let parsed: Value = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON parsing error: {}", e))?;
        
        let obj = parsed.as_object()
            .ok_or_else(|| "Invalid JSON: expected object".to_string())?;
        
        let mcp_id = name.to_string();
        
        let description = obj.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let transport = obj.get("transport")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Parse methods
        let methods = obj.get("methods")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let method_obj = item.as_object()?;
                    
                    let name = method_obj.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("").to_string();
                    
                    let description = method_obj.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("").to_string();
                    
                    let required_params = method_obj.get("parameters")
                        .and_then(|v| v.as_array())
                        .map(|params| {
                            params.iter()
                                .filter_map(|param| param.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        });
                    
                    let input_schema = method_obj.get("inputSchema")
                        .and_then(|v| v.as_object())
                        .map(|schema| {
                            let schema_type = schema.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("object").to_string();
                            
                            let properties = schema.get("properties")
                                .and_then(|v| v.as_object())
                                .map(|props| {
                                    props.iter()
                                        .filter_map(|(k, v)| {
                                            Some((k.clone(), Self::parse_schema_property(v)?))
                                        })
                                        .collect::<HashMap<String, Box<SchemaProperty>>>()
                                })
                                .unwrap_or_else(HashMap::new);
                            
                            let required = schema.get("required")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                                        .collect::<Vec<String>>()
                                });
                            
                            InputSchema {
                                schema_type,
                                properties,
                                required,
                            }
                        });
                    
                    Some(Method {
                        name,
                        description,
                        required_params,
                        input_schema,
                    })
                }).collect::<Vec<Method>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Parse source
        let source = obj.get("source")
            .and_then(|v| v.as_object())
            .map(|src| {
                Source {
                    author: src.get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    version: src.get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    github: src.get("github")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                }
            })
            .unwrap_or_else(|| Source {
                author: String::new(),
                version: String::new(),
                github: String::new(),
            });
        
        // Parse keywords
        let keywords = obj.get("functional_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Parse scenarios
        let scenarios = obj.get("scenario_phrases")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Create and store the index
        let aio_index = AioIndex {
            id: mcp_id.to_string(),
            description: description.to_string(),
            transport,
            methods,
            source,
            keywords,
            scenarios,
        };
        
        self.create(aio_index)
    }
    
    /// Search for indices by keyword
    pub fn search_by_keyword(&self, keyword: &str) -> Vec<AioIndex> {
        let keyword_lower = keyword.to_lowercase();
        let mut result = Vec::new();
        
        KEYWORD_INDEX.with(|keyword_index| {
            let keyword_index = keyword_index.borrow();
            
            // Check for exact keyword match
            if let Some(ids) = keyword_index.get(&keyword_lower) {
                for id in &ids.0 {
                    if let Some(index) = self.read(id) {
                        result.push(index);
                    }
                }
            }
            
            // Check for partial keyword matches
            for (key, index_ids) in keyword_index.iter() {
                if key.contains(&keyword_lower) && *key != keyword_lower {
                    for id in &index_ids.0 {
                        let index = self.read(id);
                        if let Some(index_item) = index {
                            if !result.iter().any(|i| i.id == index_item.id) {
                                result.push(index_item);
                            }
                        }
                    }
                }
            }
        });
        
        result
    }

    /// Get index as JSON string
    pub fn get_json(&self, id: &str) -> Result<String, String> {
        let index = self.read(id).ok_or_else(|| format!("Index with ID {} not found", id))?;
        serde_json::to_string(&index).map_err(|e| format!("Failed to convert index to JSON: {}", e))
    }
    
    pub fn search_full_text(&self, query: &str) -> Vec<AioIndex> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            
            for (_, index) in indices.iter() {
                // Check ID
                let id_match = index.id.to_lowercase().contains(&query_lower);
                
                // Check description
                let desc_match = index.description.to_lowercase().contains(&query_lower);
                
                // Check source author
                let author_match = index.source.author.to_lowercase().contains(&query_lower);
                
                // Check methods
                let methods_match = index.methods.iter().any(|method| {
                    method.name.to_lowercase().contains(&query_lower) || 
                    method.description.to_lowercase().contains(&query_lower)
                });
                
                // Check keywords
                let keywords_match = index.keywords.iter().any(|keyword| {
                    keyword.to_lowercase().contains(&query_lower)
                });
                
                // Check scenarios
                let scenarios_match = index.scenarios.iter().any(|scenario| {
                    scenario.to_lowercase().contains(&query_lower)
                });
                
                if id_match || desc_match || author_match || methods_match || keywords_match || scenarios_match {
                    seen_ids.insert(index.id.clone());
                    results.push(index.clone());
                }
            }
        });
        
        results
    }
    
    /// Get the count of all indices
    pub fn count(&self) -> usize {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.len() as usize
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aio_index() {
        let manager = AioIndexManager::new();
        // Assuming there's a previous test that creates this index
        let result = manager.read("1743948342885");
        assert!(result.is_some());
        let index = result.unwrap();
        assert_eq!(index.id, "1743948342885");
        assert_eq!(index.description, "This service provides MCP services related to memory");
        assert_eq!(index.keywords.len(), 2);
    }

    #[test]
    fn test_create_from_json() {
        let manager = AioIndexManager::new();
        let json_str = r#"
        {
            "description": "Test Service",
            "author": "Test Author",
            "version": "1.0.0",
            "github": "https://github.com/test",
            "transport": ["http", "https"],
            "methods": [
                {
                    "name": "test_method",
                    "description": "Test Method",
                    "parameters": ["param1", "param2"],
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "nested": {
                                "type": "object",
                                "properties": {
                                    "deep": {
                                        "type": "string",
                                        "description": "Deeply nested property"
                                    }
                                }
                            },
                            "array": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": {
                                            "type": "string"
                                        }
                                    }
                                }
                            }
                        },
                        "required": ["nested"]
                    }
                }
            ],
            "source": {
                "author": "Source Author",
                "version": "1.0.0",
                "github": "https://github.com/source"
            },
            "functional_keywords": ["test", "keyword"],
            "scenario_phrases": ["test scenario"]
        }"#;

        let result = manager.create_from_json("test_id", json_str);
        assert!(result.is_ok());

        let index = manager.read("test_id").unwrap();
        assert_eq!(index.id, "test_id");
        assert_eq!(index.description, "Test Service");
        assert_eq!(index.author, "Test Author");
        assert_eq!(index.version, "1.0.0");
        assert_eq!(index.github, "https://github.com/test");
        assert_eq!(index.transport, vec!["http", "https"]);
        assert_eq!(index.keywords, vec!["test", "keyword"]);
        assert_eq!(index.scenarios, vec!["test scenario"]);

        // Check methods
        assert_eq!(index.methods.len(), 1);
        let method = &index.methods[0];
        assert_eq!(method.name, "test_method");
        assert_eq!(method.description, "Test Method");
        assert_eq!(method.required_params, Some(vec!["param1".to_string(), "param2".to_string()]));

        // Check input schema
        let input_schema = method.input_schema.as_ref().unwrap();
        assert_eq!(input_schema.schema_type, "object");
        assert!(input_schema.required.as_ref().unwrap().contains(&"nested".to_string()));

        // Check nested properties
        let nested_prop = input_schema.properties.get("nested").unwrap();
        assert_eq!(nested_prop.property_type, "object");
        let deep_prop = nested_prop.properties.as_ref().unwrap().get("deep").unwrap();
        assert_eq!(deep_prop.property_type, "string");
        assert_eq!(deep_prop.description.as_ref().unwrap(), "Deeply nested property");

        // Check array properties
        let array_prop = input_schema.properties.get("array").unwrap();
        assert_eq!(array_prop.property_type, "array");
        let items = array_prop.items.as_ref().unwrap();
        assert_eq!(items.property_type, "object");
        let name_prop = items.properties.as_ref().unwrap().get("name").unwrap();
        assert_eq!(name_prop.property_type, "string");
    }
}
