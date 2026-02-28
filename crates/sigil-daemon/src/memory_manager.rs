//! Memory Management
//!
//! Stores and retrieves plain markdown notes on the sigil floppy disk.
//! Follows the Claude Code auto-memory pattern with Obsidian-compatible conventions.
//!
//! Disk layout:
//! ```text
//! {disk_mount}/memory/
//! ├── MEMORY.md           # Concise index with [[wikilinks]]
//! ├── architecture.md     # Topic files with optional YAML frontmatter
//! └── ...
//! ```

use crate::disk_watcher::DiskWatcher;
use crate::ipc::{MemoryEntry, MemoryStatusData};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;
use tracing::{debug, info};

/// Memory manager for plain markdown storage on sigil disks
pub struct MemoryManager {
    disk_watcher: Arc<DiskWatcher>,
}

impl MemoryManager {
    pub fn new(disk_watcher: Arc<DiskWatcher>) -> Self {
        Self { disk_watcher }
    }

    /// Store a memory note under a topic key.
    /// Creates or appends to a topic file and updates the MEMORY.md index.
    pub async fn store_memory(
        &self,
        topic_key: String,
        content_description: String,
    ) -> Result<String> {
        let memory_dir = self.get_or_create_memory_dir().await?;
        let file_name = Self::topic_to_filename(&topic_key);
        let file_path = memory_dir.join(&file_name);
        let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        if fs::try_exists(&file_path).await.unwrap_or(false) {
            // Append to existing topic file
            let existing = fs::read_to_string(&file_path).await?;
            let updated = format!(
                "{}\n\n## {} Update\n\n{}\n",
                existing.trim_end(),
                current_date,
                content_description
            );
            fs::write(&file_path, updated).await?;
        } else {
            // Create new topic file with YAML frontmatter
            let content = format!(
                "---\ndate: {}\ntags: []\n---\n\n# {}\n\n{}\n",
                current_date,
                Self::topic_to_title(&topic_key),
                content_description
            );
            fs::write(&file_path, content).await?;
        }

        // Update MEMORY.md index
        self.update_index(&memory_dir, &file_name).await?;

        info!("Memory stored: {}", file_name);
        Ok(format!("Memory stored: {} at {:?}", file_name, file_path))
    }

    /// Search memory notes by text terms
    pub async fn query_memory(&self, search_terms: String) -> Result<Vec<MemoryEntry>> {
        let memory_dir = self.get_memory_dir().await?;
        let mut results = Vec::new();

        let mut entries = ReadDirStream::new(fs::read_dir(&memory_dir).await?);
        while let Some(entry_result) = entries.next().await {
            let entry = entry_result?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if Self::matches_search(&content, &search_terms) {
                        results.push(Self::parse_entry(&path, &content));
                    }
                }
            }
        }

        // Sort by date descending (most recent first)
        results.sort_by(|a, b| b.date.cmp(&a.date));
        Ok(results)
    }

    /// Get memory status and disk usage
    pub async fn get_memory_status(&self) -> Result<MemoryStatusData> {
        let disk_path = self.disk_watcher.get_current_disk_path().await;
        let disk_path_str = disk_path.as_ref().map(|p| p.to_string_lossy().to_string());

        let (total_files, usage_bytes, usage_display, has_index) = match &disk_path {
            Some(_) => match self.get_memory_dir().await {
                Ok(memory_dir) => {
                    let count = Self::count_files(&memory_dir).await.unwrap_or(0);
                    let bytes = Self::calculate_dir_size(&memory_dir).await.unwrap_or(0);
                    let display = Self::format_bytes(bytes);
                    let index_exists = fs::try_exists(memory_dir.join("MEMORY.md"))
                        .await
                        .unwrap_or(false);
                    (count, bytes, display, index_exists)
                }
                Err(_) => (0, 0, "No memory directory".to_string(), false),
            },
            None => (0, 0, "No disk detected".to_string(), false),
        };

        Ok(MemoryStatusData {
            disk_path: disk_path_str,
            total_files,
            memory_usage_bytes: usage_bytes,
            memory_usage_display: usage_display,
            has_index,
        })
    }

    // --- Directory helpers ---

    /// Get or create the memory directory on the sigil disk
    async fn get_or_create_memory_dir(&self) -> Result<PathBuf> {
        let disk_path = self
            .disk_watcher
            .get_current_disk_path()
            .await
            .ok_or_else(|| anyhow!("No sigil disk detected"))?;

        let mount_dir = disk_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid disk path: no parent directory"))?;
        let memory_dir = mount_dir.join("memory");

        fs::create_dir_all(&memory_dir).await?;
        Ok(memory_dir)
    }

    /// Get existing memory directory (no creation)
    async fn get_memory_dir(&self) -> Result<PathBuf> {
        let disk_path = self
            .disk_watcher
            .get_current_disk_path()
            .await
            .ok_or_else(|| anyhow!("No sigil disk detected"))?;

        let mount_dir = disk_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid disk path: no parent directory"))?;
        let memory_dir = mount_dir.join("memory");

        if !fs::try_exists(&memory_dir).await.unwrap_or(false) {
            return Err(anyhow!("Memory directory not initialized"));
        }
        Ok(memory_dir)
    }

    // --- Index management ---

    /// Update MEMORY.md index with a wikilink to the topic file
    async fn update_index(&self, memory_dir: &Path, file_name: &str) -> Result<()> {
        let index_path = memory_dir.join("MEMORY.md");
        let topic_link = format!("[[{}]]", file_name.trim_end_matches(".md"));

        if fs::try_exists(&index_path).await.unwrap_or(false) {
            let content = fs::read_to_string(&index_path).await?;
            if !content.contains(&topic_link) {
                let updated = format!("{}\n- {}\n", content.trim_end(), topic_link);
                fs::write(&index_path, updated).await?;
                debug!("Updated MEMORY.md with: {}", topic_link);
            }
        } else {
            let content = format!(
                "# Sigil Memory\n\nTopics stored on this disk.\n\n## Topics\n\n- {}\n",
                topic_link
            );
            fs::write(&index_path, content).await?;
            debug!("Created MEMORY.md with: {}", topic_link);
        }
        Ok(())
    }

    // --- Parsing helpers ---

    /// Parse a memory entry from a markdown file
    fn parse_entry(path: &Path, content: &str) -> MemoryEntry {
        let title = Self::extract_heading(content).unwrap_or_else(|| {
            path.file_stem()
                .map(|s| Self::topic_to_title(&s.to_string_lossy()))
                .unwrap_or_default()
        });
        let date = Self::extract_frontmatter_value(content, "date")
            .unwrap_or_else(|| "unknown".to_string());
        let tags = Self::extract_frontmatter_list(content, "tags");
        let summary = Self::extract_first_paragraph(content);

        MemoryEntry {
            title,
            path: path.to_string_lossy().to_string(),
            date,
            tags,
            summary,
        }
    }

    /// Extract the first `# ` heading from markdown content
    fn extract_heading(content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(heading) = trimmed.strip_prefix("# ") {
                return Some(heading.trim().to_string());
            }
        }
        None
    }

    /// Extract a single value from YAML frontmatter
    fn extract_frontmatter_value(content: &str, key: &str) -> Option<String> {
        let frontmatter = Self::extract_frontmatter(content)?;
        let pattern = format!("{}:", key);
        for line in frontmatter.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(&pattern) {
                let value = rest.trim();
                if !value.is_empty() && !value.starts_with('[') {
                    return Some(value.to_string());
                }
            }
        }
        None
    }

    /// Extract a list value from YAML frontmatter (e.g., `tags: [a, b, c]`)
    fn extract_frontmatter_list(content: &str, key: &str) -> Vec<String> {
        let frontmatter = match Self::extract_frontmatter(content) {
            Some(fm) => fm,
            None => return Vec::new(),
        };
        let pattern = format!("{}:", key);
        for line in frontmatter.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(&pattern) {
                let value = rest.trim();
                if let Some(inner) = value.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    return inner
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
        Vec::new()
    }

    /// Extract YAML frontmatter block (content between `---` delimiters)
    fn extract_frontmatter(content: &str) -> Option<String> {
        let content = content.trim_start();
        if !content.starts_with("---") {
            return None;
        }
        let after_first = &content[3..];
        let end = after_first.find("---")?;
        Some(after_first[..end].to_string())
    }

    /// Extract first non-empty paragraph after frontmatter and heading
    fn extract_first_paragraph(content: &str) -> String {
        let mut past_frontmatter = !content.trim_start().starts_with("---");
        let mut past_heading = false;
        let mut in_frontmatter = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if !past_frontmatter {
                if trimmed == "---" {
                    if in_frontmatter {
                        past_frontmatter = true;
                    } else {
                        in_frontmatter = true;
                    }
                }
                continue;
            }

            if !past_heading {
                if trimmed.starts_with("# ") {
                    past_heading = true;
                    continue;
                }
                if trimmed.is_empty() {
                    continue;
                }
                // No heading found, treat content as starting here
                past_heading = true;
            }

            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                return trimmed.to_string();
            }
        }
        "No summary available".to_string()
    }

    /// Check if content matches any of the search terms (case-insensitive)
    fn matches_search(content: &str, search_terms: &str) -> bool {
        let content_lower = content.to_lowercase();
        search_terms
            .split_whitespace()
            .any(|term| content_lower.contains(&term.to_lowercase()))
    }

    // --- Naming helpers ---

    /// Convert a topic key to a filename: "signing patterns" → "signing-patterns.md"
    fn topic_to_filename(topic: &str) -> String {
        let slug: String = topic
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        // Collapse multiple hyphens and trim
        let mut result = String::new();
        let mut prev_hyphen = false;
        for c in slug.chars() {
            if c == '-' {
                if !prev_hyphen && !result.is_empty() {
                    result.push('-');
                }
                prev_hyphen = true;
            } else {
                result.push(c);
                prev_hyphen = false;
            }
        }
        let trimmed = result.trim_end_matches('-');
        format!("{}.md", trimmed)
    }

    /// Convert a topic key to a display title: "signing-patterns" → "Signing Patterns"
    fn topic_to_title(topic: &str) -> String {
        topic
            .replace(['-', '_'], " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    // --- Filesystem helpers ---

    /// Count markdown files in a directory
    async fn count_files(dir: &Path) -> Result<u32> {
        let mut count = 0;
        let mut entries = ReadDirStream::new(fs::read_dir(dir).await?);
        while let Some(entry_result) = entries.next().await {
            let entry = entry_result?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Recursively calculate directory size in bytes
    fn calculate_dir_size(
        dir: &Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + Send + '_>> {
        Box::pin(async move {
            let mut size = 0u64;
            let mut entries = ReadDirStream::new(fs::read_dir(dir).await?);
            while let Some(entry_result) = entries.next().await {
                let entry = entry_result?;
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    size += Self::calculate_dir_size(&entry.path()).await?;
                } else {
                    size += metadata.len();
                }
            }
            Ok(size)
        })
    }

    /// Format bytes in human-readable format
    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }
}
