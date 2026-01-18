//! MCP message types for tools, resources, and prompts

use serde::{Deserialize, Serialize};

// ============================================================================
// Tool Types
// ============================================================================

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Unique tool name
    pub name: String,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Tool description
    pub description: String,

    /// JSON Schema for input parameters
    pub input_schema: serde_json::Value,

    /// JSON Schema for output (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,

    /// Tool annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

/// Tool behavior annotations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// Tool is read-only (no side effects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,

    /// Tool has destructive/irreversible effects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,

    /// Tool is idempotent (safe to retry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,

    /// Tool accesses external systems
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// tools/list request parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolsListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// tools/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// tools/call request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCallParams {
    /// Tool name to invoke
    pub name: String,

    /// Tool arguments
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Tool call result content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<ContentAnnotations>,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<ContentAnnotations>,
    },
    Resource {
        resource: ResourceContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<ContentAnnotations>,
    },
}

impl ToolContent {
    pub fn text(text: impl Into<String>) -> Self {
        ToolContent::Text {
            text: text.into(),
            annotations: None,
        }
    }

    pub fn json<T: Serialize>(value: &T) -> Self {
        ToolContent::Text {
            text: serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()),
            annotations: None,
        }
    }
}

/// Content annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnnotations {
    /// Intended audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,

    /// Priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

/// tools/call response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCallResult {
    /// Result content
    pub content: Vec<ToolContent>,

    /// Whether this represents an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,

    /// Structured content (matches output_schema if defined)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
}

impl ToolsCallResult {
    pub fn success(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: None,
            structured_content: None,
        }
    }

    pub fn success_with_structured(
        content: Vec<ToolContent>,
        structured: serde_json::Value,
    ) -> Self {
        Self {
            content,
            is_error: None,
            structured_content: Some(structured),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(message)],
            is_error: Some(true),
            structured_content: None,
        }
    }
}

// ============================================================================
// Resource Types
// ============================================================================

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// Resource URI
    pub uri: String,

    /// Resource name
    pub name: String,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Content annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
}

/// Resource template (URI template)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
    /// URI template (RFC 6570)
    pub uri_template: String,

    /// Template name
    pub name: String,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Template description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type of resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// resources/list request parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// resources/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// resources/templates/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplatesListResult {
    pub resource_templates: Vec<ResourceTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// resources/read request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadParams {
    pub uri: String,
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    /// Resource URI
    pub uri: String,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Binary content (base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,

    /// Content annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ContentAnnotations>,
}

/// resources/read response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadResult {
    pub contents: Vec<ResourceContent>,
}

/// resources/subscribe request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesSubscribeParams {
    pub uri: String,
}

// ============================================================================
// Prompt Types
// ============================================================================

/// Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// Unique prompt name
    pub name: String,

    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,

    /// Argument description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// prompts/list request parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// prompts/list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsListResult {
    pub prompts: Vec<Prompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// prompts/get request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsGetParams {
    /// Prompt name
    pub name: String,

    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Prompt message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    User,
    Assistant,
}

/// Prompt message content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptContent {
    Text {
        text: String,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    Resource {
        resource: ResourceContent,
    },
}

/// Prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: PromptContent,
}

/// prompts/get response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsGetResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

// ============================================================================
// Notification Types
// ============================================================================

/// tools/list_changed notification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolsListChangedNotification {}

/// resources/list_changed notification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesListChangedNotification {}

/// resources/updated notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesUpdatedNotification {
    pub uri: String,
}

/// prompts/list_changed notification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsListChangedNotification {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            name: "test_tool".to_string(),
            title: Some("Test Tool".to_string()),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            output_schema: None,
            annotations: Some(ToolAnnotations {
                read_only_hint: Some(true),
                ..Default::default()
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("readOnlyHint"));
    }

    #[test]
    fn test_tool_content_text() {
        let content = ToolContent::text("Hello, world!");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_tools_call_result_error() {
        let result = ToolsCallResult::error("Something went wrong");
        assert_eq!(result.is_error, Some(true));
    }
}
