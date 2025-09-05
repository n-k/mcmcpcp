# Creative Writer MCP Server - Tool Reference

This document outlines the comprehensive set of tools available in the enhanced Creative Writer MCP server, designed specifically for creative story writing by LLMs.

## Overview

The Creative Writer MCP server transforms the basic paragraph-based toolset into a sophisticated creative writing assistant with 18 specialized tools organized into 6 categories:

## Tool Categories

### 📖 Story Structure & Management
Tools for managing the overall story structure and metadata.

#### `update_story_metadata`
Update story metadata including title, genre, themes, target audience, and synopsis.
- **Parameters:** title, genre, themes[], target_audience, synopsis
- **Use Case:** Set up story foundation and track high-level story information

#### `create_chapter`
Create a new chapter with title, content, and metadata.
- **Parameters:** title*, content*, summary, plot_points[]
- **Use Case:** Add structured chapters with automatic word counting and plot tracking

#### `get_story_outline`
Get the complete story structure including chapters, word counts, and summaries.
- **Use Case:** Review story organization and chapter progression

#### `get_story_statistics`
Get comprehensive story statistics including word counts, reading time estimates, and content metrics.
- **Use Case:** Track writing progress and story scope

### 👥 Character Development
Tools for creating and managing character profiles and relationships.

#### `create_character`
Create a new character with detailed profile including traits, backstory, and goals.
- **Parameters:** name*, description*, traits[], backstory, goals
- **Use Case:** Build rich, multi-dimensional characters

#### `update_character`
Update an existing character's details.
- **Parameters:** name*, description, traits[], backstory, goals
- **Use Case:** Evolve characters throughout the writing process

#### `add_character_relationship`
Add or update relationships between characters.
- **Parameters:** character1*, character2*, relationship*
- **Use Case:** Build character dynamics and story connections

#### `get_character_details`
Get detailed information about a specific character.
- **Parameters:** name*
- **Use Case:** Review character information during writing

#### `list_characters`
List all characters with basic information.
- **Use Case:** Get overview of story's cast of characters

### 🌍 World-building
Tools for creating and managing story world elements.

#### `create_world_element`
Create world-building elements such as locations, cultures, historical events, or magic systems.
- **Parameters:** name*, element_type*, description*, properties{}
- **Use Case:** Build rich, consistent story worlds

#### `get_world_element`
Get details about a specific world element.
- **Parameters:** name*
- **Use Case:** Reference world-building details during writing

#### `list_world_elements`
List all world elements, optionally filtered by type.
- **Parameters:** element_type (optional)
- **Use Case:** Browse world-building elements by category

### 📈 Plot & Narrative
Tools for managing plot development and story structure analysis.

#### `add_plot_point`
Add a major plot point or story event.
- **Parameters:** plot_point*
- **Use Case:** Track key story developments and plot progression

#### `analyze_story_structure`
Analyze the current story structure and provide feedback on narrative arc.
- **Use Case:** Get insights into story balance and development areas

### ✍️ Writing Enhancement
Tools for analyzing and improving writing quality.

#### `analyze_chapter_content`
Analyze a specific chapter for pacing, style, and narrative elements.
- **Parameters:** chapter_index*
- **Use Case:** Get detailed metrics and analysis for individual chapters

#### `suggest_character_development`
Suggest character development opportunities based on current story.
- **Parameters:** character_name (optional)
- **Use Case:** Identify areas for character growth and development

### 📝 Notes & Organization
Tools for managing story notes and exporting content.

#### `add_story_note`
Add a note or reminder about the story.
- **Parameters:** note*
- **Use Case:** Track ideas, reminders, and writing notes

#### `get_story_notes`
Get all story notes.
- **Use Case:** Review accumulated notes and ideas

#### `export_story`
Export the complete story in various formats.
- **Parameters:** format ("markdown", "plain_text", "structured")
- **Use Case:** Generate formatted output for different purposes

## Key Improvements Over Original

### ❌ Original Limitations:
- Line-based paragraph model (unrealistic)
- Only 4 basic CRUD operations
- No story structure awareness
- No character or world-building support
- No creative writing assistance
- No analysis or enhancement tools

### ✅ Enhanced Capabilities:
- **Rich Data Models:** Proper story structure with chapters, characters, world elements
- **Comprehensive Toolset:** 18 specialized tools across 6 categories
- **Creative Writing Focus:** Tools designed specifically for story creation
- **Analysis & Insights:** Built-in analysis tools for story improvement
- **Flexible Export:** Multiple export formats for different use cases
- **Relationship Tracking:** Character relationships and world element connections
- **Progress Tracking:** Word counts, statistics, and development metrics

## Usage Examples

### Setting Up a New Story
```
1. update_story_metadata(title="The Last Dragon", genre="Fantasy", themes=["redemption", "friendship"])
2. create_character(name="Aria", description="Young dragon rider with a mysterious past")
3. create_world_element(name="Drakmoor", element_type="location", description="Ancient dragon sanctuary")
4. add_plot_point(plot_point="Aria discovers her true heritage")
```

### Writing and Analysis Workflow
```
1. create_chapter(title="The Awakening", content="...", summary="Aria meets her first dragon")
2. analyze_chapter_content(chapter_index=0)
3. suggest_character_development()
4. analyze_story_structure()
```

### Export and Review
```
1. get_story_outline()
2. get_story_statistics()
3. export_story(format="markdown")
```

## Technical Implementation

- **Language:** Rust with async/await support
- **Architecture:** MCP (Model Context Protocol) server
- **Data Persistence:** In-memory with JSON serialization support
- **Error Handling:** Comprehensive error responses with helpful messages
- **Type Safety:** Strong typing with serde serialization/deserialization

This enhanced MCP server provides LLMs with professional-grade creative writing tools, enabling sophisticated story development workflows that go far beyond simple text manipulation.
