use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::bail;
use dioxus::{logger::tracing::warn, prelude::*};
use serde_json::{json, Value};

use crate::mcp::{
    fetch::FetchMcpServer, host::{MCPHost, MCPServer}, McpTool, ToolResult, ToolResultContent
};

use super::Toolset;

#[derive(Clone)]
pub struct StoryWriter {
    pub host: Arc<MCPHost>,
}

impl StoryWriter {
    pub fn new(story: Story) -> Self {
        let mut servers: HashMap<String, Box<dyn MCPServer>> = HashMap::new();
        servers.insert(
            "fetch".into(),
            Box::new(FetchMcpServer {}),
        );
        servers.insert(
            "creative_writer".into(),
            Box::new(CreativeWriterMcpServer::new(story)),
        );
        let host =
            MCPHost::new_with_tools(servers, Duration::from_secs(10), Duration::from_secs(10));
        Self { host: Arc::new(host) }
    }
}

#[async_trait::async_trait]
impl Toolset for StoryWriter {
    fn get_system_prompt(&self) -> String {
        "You are a helpful story and article writing assistant. 
        You have access to tools which you can call to help the user 
        in the user's task.
        For writing, you must only use provided tools.
        You MUST NOT put the story in message.
        You MUST NOT put story content, like chapters, new text etc in chat or message
        Only use the tools to add them to the story.
        You must understand any instructions the user gives you
        and only say \"OK\" if you understnad, or ask clarifying questions.
        "
        .into()
    }

    fn get_mcp_host(&self) -> Arc<MCPHost> {
        self.host.clone()
    }

    async fn get_state(&self) -> Value {
        let tr = self.host.tool_call(
            "creative_writer", 
            "export_story", 
            json!({
                "format": "structured",
            })
        ).await
        .unwrap_or_else(|e| {
            warn!("Error getting state from MCP server: {e:?}");
            ToolResult { content: vec![
                    ToolResultContent {
                        r#type: "text".into(),
                        text: Some("".into()),
                        ..Default::default()
                    },
                ], 
                is_error: None 
            }
        });
        // let v = server
        //     .rpc(
        //         "tools/call",
        //         json!({
        //             "name": "export_story",
        //             "arguments": {
        //                 "format": "structured",
        //             },
        //         })
        //     ).await
        //     .unwrap_or_else(|e| {
        //         warn!("Error getting state from MCP server: {e:?}");
        //         json!({})
        //     });
        // let tr: ToolResult = serde_json::from_value(v).unwrap();
        let s = tr.content[0].text.clone().unwrap();
        serde_json::from_str(&s).unwrap()
    }

    async fn get_markdown_repr(&self) -> Option<String> {
        // let mut map = self.host.servers.write().await;
        // let Some(server) = map.get_mut("creative_writer") else {
        //     return None;
        // };
        // let v = server
        //     .rpc(
        //         "tools/call",
        //         json!({
        //             "name": "export_story",
        //             "arguments": {
        //                 "format": "markdown",
        //             },
        //         })
        //     ).await
        //     .unwrap_or_else(|e| {
        //         warn!("Error getting state from MCP server: {e:?}");
        //         json!({})
        //     });
        let tr = self.host
            .tool_call(
                "creative_writer", 
                "export_story", 
                json!({
                    "format": "markdown",
                })
            ).await
            .unwrap_or_else(|e| {
                warn!("Error getting state from MCP server: {e:?}");
                ToolResult { content: vec![
                        ToolResultContent {
                            r#type: "text".into(),
                            text: Some("".into()),
                            ..Default::default()
                        },
                    ], 
                    is_error: None 
                }
            });
        // let tr: ToolResult = serde_json::from_value(v).unwrap();
        let s = tr.content[0].text.clone().unwrap();
        Some(s)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Character {
    pub name: String,
    pub description: String,
    pub traits: Vec<String>,
    pub backstory: String,
    pub goals: String,
    pub relationships: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Chapter {
    pub title: String,
    pub content: String,
    pub summary: String,
    pub word_count: usize,
    pub plot_points: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WorldElement {
    pub name: String,
    pub element_type: String, // "location", "culture", "history", "magic_system", etc.
    pub description: String,
    pub properties: HashMap<String, String>,
}

#[derive(Props, Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct StoryMetadata {
    pub title: String,
    pub genre: String,
    pub themes: Vec<String>,
    pub target_audience: String,
    pub synopsis: String,
}

#[derive(Props, Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Story {
    pub metadata: StoryMetadata,
    pub characters: HashMap<String, Character>,
    pub chapters: Vec<Chapter>,
    pub world_elements: HashMap<String, WorldElement>,
    pub story_notes: Vec<String>,
    pub plot_points: Vec<String>,
}

pub struct CreativeWriterMcpServer {
    pub story: Story,
}

impl CreativeWriterMcpServer {
    pub fn new(story: Story) -> Self {
        Self {
            story,
        }
    }
}

#[async_trait::async_trait]
impl MCPServer for CreativeWriterMcpServer {
    async fn list_tools(&self) -> Vec<McpTool> {
        vec![
            // Story Structure & Management
            McpTool {
                name: "update_story_metadata".into(),
                description: Some("Update story metadata including title, genre, themes, target audience, and synopsis.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string", "description": "Story title"},
                        "genre": {"type": "string", "description": "Story genre"},
                        "themes": {"type": "array", "items": {"type": "string"}, "description": "Story themes"},
                        "target_audience": {"type": "string", "description": "Target audience"},
                        "synopsis": {"type": "string", "description": "Story synopsis"}
                    }
                }),
            },
            McpTool {
                name: "create_chapter".into(),
                description: Some("Create a new chapter with title, content, and metadata.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string", "description": "Chapter title"},
                        "content": {"type": "string", "description": "Chapter content"},
                        "summary": {"type": "string", "description": "Chapter summary"},
                        "plot_points": {"type": "array", "items": {"type": "string"}, "description": "Key plot points in this chapter"},
                        "position": {"type": "number", "description": "Position to insert chapter (0-based index, optional - defaults to end)"}
                    },
                    "required": ["title", "content"]
                }),
            },
            McpTool {
                name: "update_chapter".into(),
                description: Some("Update an existing chapter's content, title, summary, or plot points.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "chapter_index": {"type": "number", "description": "Chapter index (0-based)"},
                        "title": {"type": "string", "description": "Updated chapter title"},
                        "content": {"type": "string", "description": "Updated chapter content"},
                        "summary": {"type": "string", "description": "Updated chapter summary"},
                        "plot_points": {"type": "array", "items": {"type": "string"}, "description": "Updated plot points for this chapter"}
                    },
                    "required": ["chapter_index"]
                }),
            },
            McpTool {
                name: "append_to_chapter".into(),
                description: Some("Append content to an existing chapter without replacing existing content.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "chapter_index": {"type": "number", "description": "Chapter index (0-based)"},
                        "content": {"type": "string", "description": "Content to append to the chapter"},
                        "separator": {"type": "string", "description": "Text to insert between existing and new content", "default": "\n\n"}
                    },
                    "required": ["chapter_index", "content"]
                }),
            },
            McpTool {
                name: "delete_chapter".into(),
                description: Some("Delete a chapter by its index.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "chapter_index": {"type": "number", "description": "Chapter index to delete (0-based)"}
                    },
                    "required": ["chapter_index"]
                }),
            },
            McpTool {
                name: "move_chapter".into(),
                description: Some("Move a chapter to a different position in the story.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "from_index": {"type": "number", "description": "Current chapter index (0-based)"},
                        "to_index": {"type": "number", "description": "Target position index (0-based)"}
                    },
                    "required": ["from_index", "to_index"]
                }),
            },
            McpTool {
                name: "get_chapter".into(),
                description: Some("Get detailed information about a specific chapter.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "chapter_index": {"type": "number", "description": "Chapter index (0-based)"}
                    },
                    "required": ["chapter_index"]
                }),
            },
            McpTool {
                name: "list_chapters".into(),
                description: Some("List all chapters with basic information (titles, word counts, summaries).".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "get_story_outline".into(),
                description: Some("Get the complete story structure including chapters, word counts, and summaries.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            McpTool {
                name: "get_story_statistics".into(),
                description: Some("Get story statistics including total word count, chapter count, character count, and reading time estimate.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            
            // Character Development
            McpTool {
                name: "create_character".into(),
                description: Some("Create a new character with detailed profile including traits, backstory, and goals.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Character name"},
                        "description": {"type": "string", "description": "Physical and personality description"},
                        "traits": {"type": "array", "items": {"type": "string"}, "description": "Character traits"},
                        "backstory": {"type": "string", "description": "Character backstory"},
                        "goals": {"type": "string", "description": "Character goals and motivations"}
                    },
                    "required": ["name", "description"]
                }),
            },
            McpTool {
                name: "update_character".into(),
                description: Some("Update an existing character's details.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Character name"},
                        "description": {"type": "string", "description": "Updated description"},
                        "traits": {"type": "array", "items": {"type": "string"}, "description": "Updated traits"},
                        "backstory": {"type": "string", "description": "Updated backstory"},
                        "goals": {"type": "string", "description": "Updated goals"}
                    },
                    "required": ["name"]
                }),
            },
            McpTool {
                name: "add_character_relationship".into(),
                description: Some("Add or update a relationship between two characters.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "character1": {"type": "string", "description": "First character name"},
                        "character2": {"type": "string", "description": "Second character name"},
                        "relationship": {"type": "string", "description": "Description of their relationship"}
                    },
                    "required": ["character1", "character2", "relationship"]
                }),
            },
            McpTool {
                name: "get_character_details".into(),
                description: Some("Get detailed information about a specific character.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Character name"}
                    },
                    "required": ["name"]
                }),
            },
            McpTool {
                name: "list_characters".into(),
                description: Some("List all characters with basic information.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            
            // World-building
            McpTool {
                name: "create_world_element".into(),
                description: Some("Create a world-building element such as a location, culture, historical event, or magic system.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Element name"},
                        "element_type": {"type": "string", "description": "Type: location, culture, history, magic_system, technology, etc."},
                        "description": {"type": "string", "description": "Detailed description"},
                        "properties": {"type": "object", "description": "Additional properties as key-value pairs"}
                    },
                    "required": ["name", "element_type", "description"]
                }),
            },
            McpTool {
                name: "get_world_element".into(),
                description: Some("Get details about a specific world element.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Element name"}
                    },
                    "required": ["name"]
                }),
            },
            McpTool {
                name: "list_world_elements".into(),
                description: Some("List all world elements, optionally filtered by type.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "element_type": {"type": "string", "description": "Filter by element type (optional)"}
                    }
                }),
            },
            
            // Plot & Narrative
            McpTool {
                name: "add_plot_point".into(),
                description: Some("Add a major plot point or story event.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "plot_point": {"type": "string", "description": "Description of the plot point"}
                    },
                    "required": ["plot_point"]
                }),
            },
            McpTool {
                name: "analyze_story_structure".into(),
                description: Some("Analyze the current story structure and provide feedback on narrative arc.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            
            // Writing Enhancement
            McpTool {
                name: "analyze_chapter_content".into(),
                description: Some("Analyze a specific chapter for pacing, style, and narrative elements.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "chapter_index": {"type": "number", "description": "Chapter index (0-based)"}
                    },
                    "required": ["chapter_index"]
                }),
            },
            McpTool {
                name: "suggest_character_development".into(),
                description: Some("Suggest character development opportunities based on current story.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "character_name": {"type": "string", "description": "Character to analyze (optional)"}
                    }
                }),
            },
            
            // Notes & Organization
            McpTool {
                name: "add_story_note".into(),
                description: Some("Add a note or reminder about the story.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "note": {"type": "string", "description": "Note content"}
                    },
                    "required": ["note"]
                }),
            },
            McpTool {
                name: "get_story_notes".into(),
                description: Some("Get all story notes.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            
            // Export & Formatting
            McpTool {
                name: "export_story".into(),
                description: Some("Export the complete story in a formatted structure.".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "format": {"type": "string", "description": "Export format: 'markdown', 'plain_text', or 'structured'", "default": "markdown"}
                    }
                }),
            },
        ]
    }

    async fn rpc(&mut self, method: &str, params: Value) -> anyhow::Result<serde_json::Value> {
        if method == "get_state" {
            return Ok(json!(self.story));
        }

        if method != "tools/call" {
            bail!("Error: unknown RPC method {method}");
        }

        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let result = match name {
            // Story Structure & Management
            "update_story_metadata" => self.update_story_metadata(args),
            "create_chapter" => self.create_chapter(args),
            "update_chapter" => self.update_chapter(args),
            "append_to_chapter" => self.append_to_chapter(args),
            "delete_chapter" => self.delete_chapter(args),
            "move_chapter" => self.move_chapter(args),
            "get_chapter" => self.get_chapter(args),
            "list_chapters" => self.list_chapters(),
            "get_story_outline" => self.get_story_outline(),
            "get_story_statistics" => self.get_story_statistics(),
            
            // Character Development
            "create_character" => self.create_character(args),
            "update_character" => self.update_character(args),
            "add_character_relationship" => self.add_character_relationship(args),
            "get_character_details" => self.get_character_details(args),
            "list_characters" => self.list_characters(),
            
            // World-building
            "create_world_element" => self.create_world_element(args),
            "get_world_element" => self.get_world_element(args),
            "list_world_elements" => self.list_world_elements(args),
            
            // Plot & Narrative
            "add_plot_point" => self.add_plot_point(args),
            "analyze_story_structure" => self.analyze_story_structure(),
            
            // Writing Enhancement
            "analyze_chapter_content" => self.analyze_chapter_content(args),
            "suggest_character_development" => self.suggest_character_development(args),
            
            // Notes & Organization
            "add_story_note" => self.add_story_note(args),
            "get_story_notes" => self.get_story_notes(),
            
            // Export & Formatting
            "export_story" => self.export_story(args),
            
            _ => ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Unknown tool: {name}")),
                    ..Default::default()
                }],
                is_error: Some(true),
            }
        };

        Ok(serde_json::to_value(result)?)
    }
}

impl CreativeWriterMcpServer {
    // Story Structure & Management Methods
    fn update_story_metadata(&mut self, args: Value) -> ToolResult {
        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            self.story.metadata.title = title.to_string();
        }
        if let Some(genre) = args.get("genre").and_then(|v| v.as_str()) {
            self.story.metadata.genre = genre.to_string();
        }
        if let Some(themes) = args.get("themes").and_then(|v| v.as_array()) {
            self.story.metadata.themes = themes.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
        if let Some(audience) = args.get("target_audience").and_then(|v| v.as_str()) {
            self.story.metadata.target_audience = audience.to_string();
        }
        if let Some(synopsis) = args.get("synopsis").and_then(|v| v.as_str()) {
            self.story.metadata.synopsis = synopsis.to_string();
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some("Story metadata updated successfully.".to_string()),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn create_chapter(&mut self, args: Value) -> ToolResult {
        let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Chapter").to_string();
        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let plot_points = args.get("plot_points")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_else(Vec::new);
        let position = args.get("position").and_then(|v| v.as_u64()).map(|v| v as usize);

        let word_count = content.split_whitespace().count();
        
        let chapter = Chapter {
            title: title.clone(),
            content,
            summary,
            word_count,
            plot_points,
        };

        if let Some(pos) = position {
            if pos <= self.story.chapters.len() {
                self.story.chapters.insert(pos, chapter);
            } else {
                self.story.chapters.push(chapter);
            }
        } else {
            self.story.chapters.push(chapter);
        }

        let final_position = position.unwrap_or(self.story.chapters.len() - 1);
        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Chapter '{}' created successfully with {} words at position {}.", title, word_count, final_position)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn update_chapter(&mut self, args: Value) -> ToolResult {
        let chapter_index = args.get("chapter_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if chapter_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Chapter index {} is out of range. Story has {} chapters.", 
                        chapter_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let chapter = &mut self.story.chapters[chapter_index];
        let mut updated_fields = Vec::new();

        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            chapter.title = title.to_string();
            updated_fields.push("title");
        }

        if let Some(content) = args.get("content").and_then(|v| v.as_str()) {
            chapter.content = content.to_string();
            chapter.word_count = content.split_whitespace().count();
            updated_fields.push("content");
        }

        if let Some(summary) = args.get("summary").and_then(|v| v.as_str()) {
            chapter.summary = summary.to_string();
            updated_fields.push("summary");
        }

        if let Some(plot_points) = args.get("plot_points").and_then(|v| v.as_array()) {
            chapter.plot_points = plot_points.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            updated_fields.push("plot_points");
        }

        if updated_fields.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("No fields provided to update.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Chapter {} '{}' updated successfully. Updated fields: {}", 
                    chapter_index, chapter.title, updated_fields.join(", "))),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn append_to_chapter(&mut self, args: Value) -> ToolResult {
        let chapter_index = args.get("chapter_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if chapter_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Chapter index {} is out of range. Story has {} chapters.", 
                        chapter_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let content_to_append = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
        
        if content_to_append.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Content to append is required.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let separator = args.get("separator").and_then(|v| v.as_str()).unwrap_or("\n\n");
        
        let chapter = &mut self.story.chapters[chapter_index];
        let original_word_count = chapter.word_count;
        
        // Append the content with separator
        if !chapter.content.is_empty() {
            chapter.content.push_str(separator);
        }
        chapter.content.push_str(content_to_append);
        
        // Recalculate word count
        chapter.word_count = chapter.content.split_whitespace().count();
        let words_added = chapter.word_count - original_word_count;

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Successfully appended {} words to chapter {} '{}'. Total word count is now {}.", 
                    words_added, chapter_index, chapter.title, chapter.word_count)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn delete_chapter(&mut self, args: Value) -> ToolResult {
        let chapter_index = args.get("chapter_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if chapter_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Chapter index {} is out of range. Story has {} chapters.", 
                        chapter_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let removed_chapter = self.story.chapters.remove(chapter_index);

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Chapter {} '{}' deleted successfully.", chapter_index, removed_chapter.title)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn move_chapter(&mut self, args: Value) -> ToolResult {
        let from_index = args.get("from_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let to_index = args.get("to_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if from_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Source chapter index {} is out of range. Story has {} chapters.", 
                        from_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        if to_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Target chapter index {} is out of range. Story has {} chapters.", 
                        to_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        if from_index == to_index {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Source and target indices are the same. No move needed.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(false),
            };
        }

        let chapter = self.story.chapters.remove(from_index);
        let chapter_title = chapter.title.clone();
        self.story.chapters.insert(to_index, chapter);

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Chapter '{}' moved from position {} to position {}.", 
                    chapter_title, from_index, to_index)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn get_chapter(&self, args: Value) -> ToolResult {
        let chapter_index = args.get("chapter_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if chapter_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Chapter index {} is out of range. Story has {} chapters.", 
                        chapter_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let chapter = &self.story.chapters[chapter_index];
        let mut details = format!("# Chapter {}: {}\n\n", chapter_index + 1, chapter.title);
        
        details.push_str(&format!("**Word Count:** {}\n", chapter.word_count));
        details.push_str(&format!("**Estimated Reading Time:** {} minutes\n\n", 
            (chapter.word_count as f64 / 250.0).ceil() as usize));

        if !chapter.summary.is_empty() {
            details.push_str(&format!("**Summary:** {}\n\n", chapter.summary));
        }

        if !chapter.plot_points.is_empty() {
            details.push_str("**Plot Points:**\n");
            for point in &chapter.plot_points {
                details.push_str(&format!("- {}\n", point));
            }
            details.push('\n');
        }

        details.push_str("**Content:**\n\n");
        details.push_str(&chapter.content);

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(details),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn list_chapters(&self) -> ToolResult {
        if self.story.chapters.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("No chapters created yet.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(false),
            };
        }

        let mut list = "# Chapters\n\n".to_string();
        for (i, chapter) in self.story.chapters.iter().enumerate() {
            list.push_str(&format!("## {}. {} ({} words)\n\n", i + 1, chapter.title, chapter.word_count));
            
            if !chapter.summary.is_empty() {
                list.push_str(&format!("**Summary:** {}\n\n", chapter.summary));
            }

            if !chapter.plot_points.is_empty() {
                list.push_str("**Plot Points:** ");
                list.push_str(&chapter.plot_points.join(", "));
                list.push_str("\n\n");
            }
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(list),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn get_story_outline(&self) -> ToolResult {
        let mut outline = format!("# Story Outline: {}\n\n", self.story.metadata.title);
        outline.push_str(&format!("**Genre:** {}\n", self.story.metadata.genre));
        outline.push_str(&format!("**Themes:** {}\n", self.story.metadata.themes.join(", ")));
        outline.push_str(&format!("**Target Audience:** {}\n\n", self.story.metadata.target_audience));
        
        if !self.story.metadata.synopsis.is_empty() {
            outline.push_str(&format!("**Synopsis:** {}\n\n", self.story.metadata.synopsis));
        }

        outline.push_str("## Chapters:\n\n");
        for (i, chapter) in self.story.chapters.iter().enumerate() {
            outline.push_str(&format!("{}. **{}** ({} words)\n", i + 1, chapter.title, chapter.word_count));
            if !chapter.summary.is_empty() {
                outline.push_str(&format!("   Summary: {}\n", chapter.summary));
            }
            if !chapter.plot_points.is_empty() {
                outline.push_str(&format!("   Plot Points: {}\n", chapter.plot_points.join(", ")));
            }
            outline.push('\n');
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(outline),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn get_story_statistics(&self) -> ToolResult {
        let total_words: usize = self.story.chapters.iter().map(|c| c.word_count).sum();
        let reading_time = (total_words as f64 / 250.0).ceil() as usize; // Assuming 250 words per minute
        
        let stats = format!(
            "# Story Statistics\n\n\
            **Total Word Count:** {}\n\
            **Chapter Count:** {}\n\
            **Character Count:** {}\n\
            **World Elements:** {}\n\
            **Plot Points:** {}\n\
            **Estimated Reading Time:** {} minutes\n\
            **Story Notes:** {}",
            total_words,
            self.story.chapters.len(),
            self.story.characters.len(),
            self.story.world_elements.len(),
            self.story.plot_points.len(),
            reading_time,
            self.story.story_notes.len()
        );

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(stats),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    // Character Development Methods
    fn create_character(&mut self, args: Value) -> ToolResult {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Character name is required.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let character = Character {
            name: name.clone(),
            description: args.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            traits: args.get("traits")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
                .unwrap_or_else(Vec::new),
            backstory: args.get("backstory").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            goals: args.get("goals").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            relationships: HashMap::new(),
        };

        self.story.characters.insert(name.clone(), character);

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Character '{}' created successfully.", name)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn update_character(&mut self, args: Value) -> ToolResult {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
        
        if let Some(character) = self.story.characters.get_mut(name) {
            if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
                character.description = description.to_string();
            }
            if let Some(traits) = args.get("traits").and_then(|v| v.as_array()) {
                character.traits = traits.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
            }
            if let Some(backstory) = args.get("backstory").and_then(|v| v.as_str()) {
                character.backstory = backstory.to_string();
            }
            if let Some(goals) = args.get("goals").and_then(|v| v.as_str()) {
                character.goals = goals.to_string();
            }

            ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Character '{}' updated successfully.", name)),
                    ..Default::default()
                }],
                is_error: Some(false),
            }
        } else {
            ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Character '{}' not found.", name)),
                    ..Default::default()
                }],
                is_error: Some(true),
            }
        }
    }

    fn add_character_relationship(&mut self, args: Value) -> ToolResult {
        let char1 = args.get("character1").and_then(|v| v.as_str()).unwrap_or("");
        let char2 = args.get("character2").and_then(|v| v.as_str()).unwrap_or("");
        let relationship = args.get("relationship").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(character1) = self.story.characters.get_mut(char1) {
            character1.relationships.insert(char2.to_string(), relationship.to_string());
        }
        if let Some(character2) = self.story.characters.get_mut(char2) {
            character2.relationships.insert(char1.to_string(), relationship.to_string());
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Relationship between '{}' and '{}' added: {}", char1, char2, relationship)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn get_character_details(&self, args: Value) -> ToolResult {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
        
        if let Some(character) = self.story.characters.get(name) {
            let mut details = format!("# Character: {}\n\n", character.name);
            details.push_str(&format!("**Description:** {}\n\n", character.description));
            
            if !character.traits.is_empty() {
                details.push_str(&format!("**Traits:** {}\n\n", character.traits.join(", ")));
            }
            
            if !character.backstory.is_empty() {
                details.push_str(&format!("**Backstory:** {}\n\n", character.backstory));
            }
            
            if !character.goals.is_empty() {
                details.push_str(&format!("**Goals:** {}\n\n", character.goals));
            }
            
            if !character.relationships.is_empty() {
                details.push_str("**Relationships:**\n");
                for (other_char, relationship) in &character.relationships {
                    details.push_str(&format!("- {}: {}\n", other_char, relationship));
                }
            }

            ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(details),
                    ..Default::default()
                }],
                is_error: Some(false),
            }
        } else {
            ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Character '{}' not found.", name)),
                    ..Default::default()
                }],
                is_error: Some(true),
            }
        }
    }

    fn list_characters(&self) -> ToolResult {
        if self.story.characters.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("No characters created yet.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(false),
            };
        }

        let mut list = "# Characters\n\n".to_string();
        for (name, character) in &self.story.characters {
            list.push_str(&format!("## {}\n", name));
            list.push_str(&format!("{}\n", character.description));
            if !character.traits.is_empty() {
                list.push_str(&format!("*Traits: {}*\n", character.traits.join(", ")));
            }
            list.push('\n');
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(list),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    // World-building Methods
    fn create_world_element(&mut self, args: Value) -> ToolResult {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let element_type = args.get("element_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        
        if name.is_empty() || element_type.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Name and element type are required.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let properties = args.get("properties")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_else(HashMap::new);

        let element = WorldElement {
            name: name.clone(),
            element_type: element_type.clone(),
            description,
            properties,
        };

        self.story.world_elements.insert(name.clone(), element);

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("World element '{}' ({}) created successfully.", name, element_type)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn get_world_element(&self, args: Value) -> ToolResult {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
        
        if let Some(element) = self.story.world_elements.get(name) {
            let mut details = format!("# World Element: {}\n\n", element.name);
            details.push_str(&format!("**Type:** {}\n\n", element.element_type));
            details.push_str(&format!("**Description:** {}\n\n", element.description));
            
            if !element.properties.is_empty() {
                details.push_str("**Properties:**\n");
                for (key, value) in &element.properties {
                    details.push_str(&format!("- {}: {}\n", key, value));
                }
            }

            ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(details),
                    ..Default::default()
                }],
                is_error: Some(false),
            }
        } else {
            ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("World element '{}' not found.", name)),
                    ..Default::default()
                }],
                is_error: Some(true),
            }
        }
    }

    fn list_world_elements(&self, args: Value) -> ToolResult {
        let filter_type = args.get("element_type").and_then(|v| v.as_str());
        
        let filtered_elements: Vec<_> = if let Some(filter) = filter_type {
            self.story.world_elements.iter()
                .filter(|(_, element)| element.element_type == filter)
                .collect()
        } else {
            self.story.world_elements.iter().collect()
        };

        if filtered_elements.is_empty() {
            let message = if filter_type.is_some() {
                format!("No world elements of type '{}' found.", filter_type.unwrap())
            } else {
                "No world elements created yet.".to_string()
            };
            
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(message),
                    ..Default::default()
                }],
                is_error: Some(false),
            };
        }

        let mut list = "# World Elements\n\n".to_string();
        for (name, element) in filtered_elements {
            list.push_str(&format!("## {} ({})\n", name, element.element_type));
            list.push_str(&format!("{}\n\n", element.description));
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(list),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    // Plot & Narrative Methods
    fn add_plot_point(&mut self, args: Value) -> ToolResult {
        let plot_point = args.get("plot_point").and_then(|v| v.as_str()).unwrap_or("").to_string();
        
        if plot_point.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Plot point description is required.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        self.story.plot_points.push(plot_point.clone());

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Plot point added: {}", plot_point)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn analyze_story_structure(&self) -> ToolResult {
        let mut analysis = "# Story Structure Analysis\n\n".to_string();
        
        // Basic structure analysis
        let chapter_count = self.story.chapters.len();
        let total_words: usize = self.story.chapters.iter().map(|c| c.word_count).sum();
        
        analysis.push_str(&format!("**Structure Overview:**\n"));
        analysis.push_str(&format!("- Chapters: {}\n", chapter_count));
        analysis.push_str(&format!("- Total Words: {}\n", total_words));
        analysis.push_str(&format!("- Average Chapter Length: {} words\n\n", 
            if chapter_count > 0 { total_words / chapter_count } else { 0 }));

        // Plot point analysis
        analysis.push_str(&format!("**Plot Development:**\n"));
        analysis.push_str(&format!("- Major Plot Points: {}\n", self.story.plot_points.len()));
        
        if !self.story.plot_points.is_empty() {
            analysis.push_str("- Plot Points:\n");
            for (i, point) in self.story.plot_points.iter().enumerate() {
                analysis.push_str(&format!("  {}. {}\n", i + 1, point));
            }
        }
        analysis.push('\n');

        // Character analysis
        analysis.push_str(&format!("**Character Development:**\n"));
        analysis.push_str(&format!("- Total Characters: {}\n", self.story.characters.len()));
        
        let characters_with_goals = self.story.characters.values().filter(|c| !c.goals.is_empty()).count();
        let characters_with_backstory = self.story.characters.values().filter(|c| !c.backstory.is_empty()).count();
        
        analysis.push_str(&format!("- Characters with defined goals: {}\n", characters_with_goals));
        analysis.push_str(&format!("- Characters with backstory: {}\n\n", characters_with_backstory));

        // World-building analysis
        analysis.push_str(&format!("**World-building:**\n"));
        analysis.push_str(&format!("- World Elements: {}\n", self.story.world_elements.len()));
        
        let element_types: std::collections::HashSet<_> = self.story.world_elements.values()
            .map(|e| &e.element_type)
            .collect();
        
        if !element_types.is_empty() {
            let types: Vec<String> = element_types.iter().map(|s| s.to_string()).collect();
            analysis.push_str(&format!("- Element Types: {}\n", types.join(", ")));
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(analysis),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    // Writing Enhancement Methods
    fn analyze_chapter_content(&self, args: Value) -> ToolResult {
        let chapter_index = args.get("chapter_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        if chapter_index >= self.story.chapters.len() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some(format!("Chapter index {} is out of range. Story has {} chapters.", 
                        chapter_index, self.story.chapters.len())),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        let chapter = &self.story.chapters[chapter_index];
        let mut analysis = format!("# Chapter Analysis: {}\n\n", chapter.title);
        
        // Basic metrics
        analysis.push_str(&format!("**Basic Metrics:**\n"));
        analysis.push_str(&format!("- Word Count: {}\n", chapter.word_count));
        analysis.push_str(&format!("- Estimated Reading Time: {} minutes\n", 
            (chapter.word_count as f64 / 250.0).ceil() as usize));
        
        // Content analysis
        let sentences = chapter.content.split('.').count();
        let paragraphs = chapter.content.split('\n').filter(|p| !p.trim().is_empty()).count();
        
        analysis.push_str(&format!("- Sentences: ~{}\n", sentences));
        analysis.push_str(&format!("- Paragraphs: {}\n", paragraphs));
        analysis.push_str(&format!("- Average Words per Paragraph: {}\n\n", 
            if paragraphs > 0 { chapter.word_count / paragraphs } else { 0 }));

        // Plot points
        if !chapter.plot_points.is_empty() {
            analysis.push_str("**Plot Points in this Chapter:**\n");
            for point in &chapter.plot_points {
                analysis.push_str(&format!("- {}\n", point));
            }
            analysis.push('\n');
        }

        // Summary
        if !chapter.summary.is_empty() {
            analysis.push_str(&format!("**Summary:** {}\n", chapter.summary));
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(analysis),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn suggest_character_development(&self, args: Value) -> ToolResult {
        let character_name = args.get("character_name").and_then(|v| v.as_str());
        
        let mut suggestions = "# Character Development Suggestions\n\n".to_string();

        if let Some(name) = character_name {
            if let Some(character) = self.story.characters.get(name) {
                suggestions.push_str(&format!("## Suggestions for {}\n\n", name));
                
                if character.goals.is_empty() {
                    suggestions.push_str("- **Define Goals:** Consider adding specific goals and motivations for this character.\n");
                }
                
                if character.backstory.is_empty() {
                    suggestions.push_str("- **Develop Backstory:** Add background information that explains their current situation and personality.\n");
                }
                
                if character.traits.is_empty() {
                    suggestions.push_str("- **Add Traits:** Define personality traits that make this character unique.\n");
                }
                
                if character.relationships.is_empty() {
                    suggestions.push_str("- **Build Relationships:** Establish connections with other characters in the story.\n");
                }
            } else {
                return ToolResult {
                    content: vec![ToolResultContent {
                        r#type: "text".to_string(),
                        text: Some(format!("Character '{}' not found.", name)),
                        ..Default::default()
                    }],
                    is_error: Some(true),
                };
            }
        } else {
            // General suggestions for all characters
            suggestions.push_str("## General Character Development Opportunities\n\n");
            
            let incomplete_characters: Vec<_> = self.story.characters.iter()
                .filter(|(_, c)| c.goals.is_empty() || c.backstory.is_empty() || c.traits.is_empty())
                .collect();
            
            if !incomplete_characters.is_empty() {
                suggestions.push_str("**Characters needing development:**\n");
                for (name, character) in incomplete_characters {
                    suggestions.push_str(&format!("- **{}:** ", name));
                    let mut needs = vec![];
                    if character.goals.is_empty() { needs.push("goals"); }
                    if character.backstory.is_empty() { needs.push("backstory"); }
                    if character.traits.is_empty() { needs.push("traits"); }
                    suggestions.push_str(&format!("{}\n", needs.join(", ")));
                }
                suggestions.push('\n');
            }
            
            // Relationship suggestions
            let characters_without_relationships: Vec<_> = self.story.characters.iter()
                .filter(|(_, c)| c.relationships.is_empty())
                .map(|(name, _)| name)
                .collect();
            
            if !characters_without_relationships.is_empty() {
                suggestions.push_str("**Characters without relationships:**\n");
                for name in characters_without_relationships {
                    suggestions.push_str(&format!("- {}\n", name));
                }
            }
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(suggestions),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    // Notes & Organization Methods
    fn add_story_note(&mut self, args: Value) -> ToolResult {
        let note = args.get("note").and_then(|v| v.as_str()).unwrap_or("").to_string();
        
        if note.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Note content is required.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            };
        }

        self.story.story_notes.push(note.clone());

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(format!("Story note added: {}", note)),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn get_story_notes(&self) -> ToolResult {
        if self.story.story_notes.is_empty() {
            return ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("No story notes yet.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(false),
            };
        }

        let mut notes = "# Story Notes\n\n".to_string();
        for (i, note) in self.story.story_notes.iter().enumerate() {
            notes.push_str(&format!("{}. {}\n", i + 1, note));
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(notes),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    // Export & Formatting Methods
    fn export_story(&self, args: Value) -> ToolResult {
        let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("markdown");
        
        match format {
            "markdown" => self.export_markdown(),
            "plain_text" => self.export_plain_text(),
            "structured" => self.export_structured(),
            _ => ToolResult {
                content: vec![ToolResultContent {
                    r#type: "text".to_string(),
                    text: Some("Invalid format. Use 'markdown', 'plain_text', or 'structured'.".to_string()),
                    ..Default::default()
                }],
                is_error: Some(true),
            }
        }
    }

    fn export_markdown(&self) -> ToolResult {
        let mut export = format!("# {}\n\n", self.story.metadata.title);
        export.push_str(&format!("**Genre:** {}\n", self.story.metadata.genre));
        export.push_str(&format!("**Target Audience:** {}\n", self.story.metadata.target_audience));
        
        if !self.story.metadata.themes.is_empty() {
            export.push_str(&format!("**Themes:** {}\n", self.story.metadata.themes.join(", ")));
        }
        export.push('\n');
        
        if !self.story.metadata.synopsis.is_empty() {
            export.push_str(&format!("## Synopsis\n\n{}\n\n", self.story.metadata.synopsis));
        }

        // Export plot points
        if !self.story.plot_points.is_empty() {
            export.push_str("## Plot Points\n\n");
            for (i, point) in self.story.plot_points.iter().enumerate() {
                export.push_str(&format!("{}. {}\n", i + 1, point));
            }
            export.push('\n');
        }

        // Export characters
        if !self.story.characters.is_empty() {
            export.push_str("## Characters\n\n");
            for (name, character) in &self.story.characters {
                export.push_str(&format!("### {}\n\n", name));
                export.push_str(&format!("**Description:** {}\n\n", character.description));
                
                if !character.traits.is_empty() {
                    export.push_str(&format!("**Traits:** {}\n\n", character.traits.join(", ")));
                }
                
                if !character.backstory.is_empty() {
                    export.push_str(&format!("**Backstory:** {}\n\n", character.backstory));
                }
                
                if !character.goals.is_empty() {
                    export.push_str(&format!("**Goals:** {}\n\n", character.goals));
                }
                
                if !character.relationships.is_empty() {
                    export.push_str("**Relationships:**\n");
                    for (other_char, relationship) in &character.relationships {
                        export.push_str(&format!("- {}: {}\n", other_char, relationship));
                    }
                    export.push('\n');
                }
            }
        }

        // Export world elements
        if !self.story.world_elements.is_empty() {
            export.push_str("## World Elements\n\n");
            for (name, element) in &self.story.world_elements {
                export.push_str(&format!("### {} ({})\n\n", name, element.element_type));
                export.push_str(&format!("**Description:** {}\n\n", element.description));
                
                if !element.properties.is_empty() {
                    export.push_str("**Properties:**\n");
                    for (key, value) in &element.properties {
                        export.push_str(&format!("- {}: {}\n", key, value));
                    }
                    export.push('\n');
                }
            }
        }

        // Export chapters
        if !self.story.chapters.is_empty() {
            export.push_str("## Chapters\n\n");
            for (i, chapter) in self.story.chapters.iter().enumerate() {
                export.push_str(&format!("### Chapter {}: {}\n\n", i + 1, chapter.title));
                
                if !chapter.summary.is_empty() {
                    export.push_str(&format!("**Summary:** {}\n\n", chapter.summary));
                }
                
                if !chapter.plot_points.is_empty() {
                    export.push_str("**Plot Points:**\n");
                    for point in &chapter.plot_points {
                        export.push_str(&format!("- {}\n", point));
                    }
                    export.push('\n');
                }
                
                export.push_str(&format!("**Word Count:** {}\n\n", chapter.word_count));
                export.push_str(&format!("{}\n\n", chapter.content));
            }
        }

        // Export story notes
        if !self.story.story_notes.is_empty() {
            export.push_str("## Story Notes\n\n");
            for (i, note) in self.story.story_notes.iter().enumerate() {
                export.push_str(&format!("{}. {}\n", i + 1, note));
            }
            export.push('\n');
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(export),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn export_plain_text(&self) -> ToolResult {
        let mut export = format!("{}\n\n", self.story.metadata.title);
        
        for (i, chapter) in self.story.chapters.iter().enumerate() {
            export.push_str(&format!("Chapter {}: {}\n\n", i + 1, chapter.title));
            export.push_str(&format!("{}\n\n", chapter.content));
        }

        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(export),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }

    fn export_structured(&self) -> ToolResult {
        ToolResult {
            content: vec![ToolResultContent {
                r#type: "text".to_string(),
                text: Some(serde_json::to_string_pretty(&self.story).unwrap_or_else(|_| "Export failed".to_string())),
                ..Default::default()
            }],
            is_error: Some(false),
        }
    }
}
