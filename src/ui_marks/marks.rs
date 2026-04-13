use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkType {
    Global,
    Bookmark,
    Search,
    Command,
    Jump,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mark {
    pub id: String,
    pub mark_type: MarkType,
    pub name: String,
    pub color: [u8; 3],
    pub row: usize,
    pub col: usize,
    pub tab_id: usize,
    pub line_content: String,
    pub created_at: std::time::SystemTime,
    pub description: Option<String>,
}

impl Mark {
    pub fn new(
        id: String,
        mark_type: MarkType,
        name: String,
        row: usize,
        col: usize,
        tab_id: usize,
        line_content: String,
    ) -> Self {
        let color = match mark_type {
            MarkType::Global => [255, 215, 0],
            MarkType::Bookmark => [0, 191, 255],
            MarkType::Search => [50, 205, 50],
            MarkType::Command => [255, 105, 180],
            MarkType::Jump => [147, 112, 219],
        };

        Self {
            id,
            mark_type,
            name,
            color,
            row,
            col,
            tab_id,
            line_content,
            created_at: std::time::SystemTime::now(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

pub struct MarkManager {
    marks: HashMap<String, Mark>,
    next_mark_id: usize,
    enabled: bool,
    auto_mark_commands: Vec<String>,
}

impl MarkManager {
    pub fn new() -> Self {
        Self {
            marks: HashMap::new(),
            next_mark_id: 1,
            enabled: true,
            auto_mark_commands: vec![
                "git".to_string(),
                "make".to_string(),
                "cargo".to_string(),
                "npm".to_string(),
                "pytest".to_string(),
            ],
        }
    }

    pub fn create_mark(
        &mut self,
        mark_type: MarkType,
        name: String,
        row: usize,
        col: usize,
        tab_id: usize,
        line_content: String,
    ) -> &Mark {
        let id = format!("mark_{}", self.next_mark_id);
        self.next_mark_id += 1;

        let mark = Mark::new(id.clone(), mark_type, name, row, col, tab_id, line_content);
        self.marks.insert(id, mark);

        self.marks
            .get(&format!("mark_{}", self.next_mark_id - 1))
            .unwrap()
    }

    pub fn create_global_mark(
        &mut self,
        name: String,
        row: usize,
        col: usize,
        tab_id: usize,
        line_content: String,
    ) -> &Mark {
        self.create_mark(MarkType::Global, name, row, col, tab_id, line_content)
    }

    pub fn create_bookmark(
        &mut self,
        name: String,
        row: usize,
        col: usize,
        tab_id: usize,
        line_content: String,
    ) -> &Mark {
        self.create_mark(MarkType::Bookmark, name, row, col, tab_id, line_content)
    }

    pub fn get_mark(&self, id: &str) -> Option<&Mark> {
        self.marks.get(id)
    }

    pub fn get_mark_mut(&mut self, id: &str) -> Option<&mut Mark> {
        self.marks.get_mut(id)
    }

    pub fn remove_mark(&mut self, id: &str) -> bool {
        self.marks.remove(id).is_some()
    }

    pub fn get_marks_for_tab(&self, tab_id: usize) -> Vec<&Mark> {
        self.marks.values().filter(|m| m.tab_id == tab_id).collect()
    }

    pub fn get_marks_in_range(
        &self,
        tab_id: usize,
        row_start: usize,
        row_end: usize,
    ) -> Vec<&Mark> {
        self.marks
            .values()
            .filter(|m| m.tab_id == tab_id && m.row >= row_start && m.row <= row_end)
            .collect()
    }

    pub fn find_mark_at(&self, tab_id: usize, row: usize, col: usize) -> Option<&Mark> {
        self.marks
            .values()
            .find(|m| m.tab_id == tab_id && m.row == row && m.col == col)
            .map(|m| m.mark_type)
            .and_then(|_| {
                self.marks
                    .values()
                    .find(|m| m.tab_id == tab_id && m.row == row && m.col == col)
            })
    }

    pub fn get_all_marks(&self) -> Vec<&Mark> {
        self.marks.values().collect()
    }

    pub fn clear_marks_for_tab(&mut self, tab_id: usize) {
        self.marks.retain(|_, m| m.tab_id != tab_id);
    }

    pub fn clear_all_marks(&mut self) {
        self.marks.clear();
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn should_auto_mark(&self, command: &str) -> bool {
        let cmd = command.split_whitespace().next().unwrap_or("");
        self.auto_mark_commands.iter().any(|c| cmd.contains(c))
    }

    pub fn auto_mark_output_line(
        &mut self,
        line: &str,
        row: usize,
        col: usize,
        tab_id: usize,
    ) -> Option<&Mark> {
        if !self.enabled || !self.should_auto_mark(line) {
            return None;
        }

        let mark_type =
            if line.starts_with("error") || line.starts_with("ERROR") || line.contains("failed") {
                MarkType::Global
            } else if line.starts_with("warning") || line.starts_with("WARNING") {
                MarkType::Search
            } else {
                MarkType::Command
            };

        let name = format!("auto_{}_{}", row, col);
        Some(self.create_mark(mark_type, name, row, col, tab_id, line.to_string()))
    }

    pub fn jump_to_next_mark(&self, tab_id: usize, current_row: usize) -> Option<(usize, usize)> {
        self.marks
            .values()
            .filter(|m| m.tab_id == tab_id && m.row > current_row)
            .min_by_key(|m| m.row)
            .map(|m| (m.row, m.col))
    }

    pub fn jump_to_prev_mark(&self, tab_id: usize, current_row: usize) -> Option<(usize, usize)> {
        self.marks
            .values()
            .filter(|m| m.tab_id == tab_id && m.row < current_row)
            .max_by_key(|m| m.row)
            .map(|m| (m.row, m.col))
    }
}

impl Default for MarkManager {
    fn default() -> Self {
        Self::new()
    }
}
