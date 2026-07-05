//! Compiles and decompiles board code

use regex::Regex;
use serde_json::{
    Map,
    Value::{self},
    json,
};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write;

pub struct Lexer<'a> {
    input: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn tokenize(&self, ignore_whitespace: bool, ignore_comments: bool) -> Vec<String> {
        let mut tokens: Vec<String> = self.input.split('\n').map(String::from).collect();

        if ignore_whitespace || ignore_comments {
            tokens.retain(|token| {
                let trimmed = token.trim();

                // Remove empty lines
                if ignore_whitespace && trimmed.is_empty() {
                    return false;
                }

                // Remove comment lines
                if ignore_comments && trimmed.starts_with("//") {
                    return false;
                }

                true
            });

            // Normalize whitespace AFTER filtering
            if ignore_whitespace {
                for token in &mut tokens {
                    *token = token.trim().to_string();
                }
            }
        }

        tokens
    }

    pub fn get_json(&self, ignore_whitespace: bool, ignore_comments: bool) -> Value {
        let sections = split_sections(&&self.tokenize(ignore_whitespace, ignore_comments));

        let mut boards = parse_boards(sections.get("BOARDS"));
        let tasks = parse_tasks(sections.get("TASKS"));

        let mut board_labels = HashSet::new();
        for board in &boards {
            if let Some(label) = board.get("label").and_then(|l| l.as_str()) {
                board_labels.insert(label.to_string());
            }
        }

        let mut added_boards = Vec::new();
        for task in &tasks {
            if let Some(board) = task.get("board").and_then(|b| b.as_str()) {
                if !board.is_empty()
                    && !board_labels.contains(board)
                    && !added_boards.iter().any(|b| b == board)
                {
                    added_boards.push(board.to_string());
                }
            }
        }

        let last_index = added_boards.len().saturating_sub(1);
        for (i, board_name) in added_boards.iter().enumerate() {
            let mut entry = Map::new();
            entry.insert("label".to_string(), json!(board_name));
            entry.insert(
                "inline_comment".to_string(),
                json!("[Lexer]: Added this line"),
            );
            boards.push(Value::Object(entry));

            if i == last_index {
                boards.push(empty());
            }
        }

        let mut board_order = HashMap::new();
        let mut index = 0;
        for board in &boards {
            if let Some(label) = board.get("label").and_then(|l| l.as_str()) {
                if !board_order.contains_key(label) {
                    board_order.insert(label.to_string(), index);
                    index += 1;
                }
            }
        }

        let mut sortable_tasks = Vec::new();
        let mut other_tasks = Vec::new();

        for task in tasks {
            if task.is_object() && task.get("board").and_then(|b| b.as_str()).is_some() {
                sortable_tasks.push(task);
            } else {
                other_tasks.push(task);
            }
        }

        sortable_tasks.sort_by_key(|task| {
            let board_name = task.get("board").and_then(|b| b.as_str()).unwrap_or("");
            board_order.get(board_name).cloned().unwrap_or(usize::MAX)
        });

        let mut final_tasks = sortable_tasks;
        final_tasks.extend(other_tasks);

        json!({
            "profiles": parse_profiles(sections.get("PROFILES")),
            "tags": {},
            "context": {},
            "boards": boards,
            "tasks": final_tasks,
        })
    }

    /// Checks the current .bd code and clears it
    pub fn clear_code(&self, ignore_comments: bool) -> String {
        let json = self.get_json(false, false);
        return create_code_from_json(json, true, ignore_comments);
    }
}

/// Creates code from json
pub fn create_code_from_json(
    json: Value,
    ignore_whitespaces: bool,
    ignore_comments: bool,
) -> String {
    // Create code based on json
    let mut code = String::new();
    let mut new_line;

    // Ensure `profiles` is an array
    code.push_str("[PROFILES]\n");
    if let Some(profiles) = json.get("profiles").and_then(Value::as_array) {
        for line in profiles {
            // Each line must be an object
            if let Some(obj) = line.as_object() {
                // Whitespace
                if obj.contains_key("__empty__") {
                    if !ignore_whitespaces {
                        code.push('\n');
                    }
                    continue;
                }
                // Comment
                else if let Some(comment) = obj.get("comment").and_then(Value::as_str) {
                    if !ignore_comments {
                        write!(code, "// {}\n", comment).unwrap();
                    }
                    continue;
                }

                new_line = String::new();

                // Description
                if let Some(description) = obj.get("description").and_then(Value::as_str) {
                    let lines: Vec<&str> = description.split('\n').collect();
                    let last_i = lines.len().saturating_sub(1);

                    for (i, line) in lines.iter().enumerate() {
                        if i == 0 {
                            if i == last_i {
                                write!(code, "\n/// {}", line).unwrap();
                            } else {
                                write!(code, "\n/// {}\n", line).unwrap();
                            }
                        } else if i == last_i {
                            write!(code, "/// {}", line).unwrap();
                        } else if !line.trim().is_empty() {
                            write!(code, "/// {}\n", line).unwrap();
                        }
                    }
                }

                // Label
                if let Some(label) = obj.get("label").and_then(Value::as_str) {
                    new_line.push_str(label);
                }

                // Inline comment
                if let Some(inline) = obj.get("inline_comment").and_then(Value::as_str)
                    && !ignore_comments
                {
                    if inline == "[Lexer]: Added this line"
                        && !code.ends_with("// [Lexer]: Added this line\n")
                    {
                        code.push_str("\n");
                    }
                    write!(new_line, " // {}", inline).unwrap();
                }

                write!(code, "{}\n", new_line.trim()).unwrap();
            }
        }
    }

    code.push_str("\n[BOARDS]\n");
    if let Some(boards) = json.get("boards").and_then(Value::as_array) {
        for line in boards {
            // Each line must be an object
            if let Some(obj) = line.as_object() {
                // Whitespace
                if obj.contains_key("__empty__") {
                    if !ignore_whitespaces {
                        code.push('\n');
                    }
                    continue;
                }
                // Comment
                else if let Some(comment) = obj.get("comment").and_then(Value::as_str) {
                    if !ignore_comments {
                        write!(code, "// {}\n", comment).unwrap();
                    }
                    continue;
                }

                new_line = String::new();

                // Description
                if let Some(description) = obj.get("description").and_then(Value::as_str) {
                    let lines: Vec<&str> = description.split('\n').collect();
                    let last_i = lines.len().saturating_sub(1);

                    for (i, line) in lines.iter().enumerate() {
                        if i == 0 {
                            if i == last_i {
                                write!(code, "\n/// {}", line).unwrap();
                            } else {
                                write!(code, "\n/// {}\n", line).unwrap();
                            }
                        } else if i == last_i {
                            write!(code, "/// {}", line).unwrap();
                        } else if !line.trim().is_empty() {
                            write!(code, "/// {}\n", line).unwrap();
                        }
                    }
                }

                // Priority
                if let Some(priority) = obj.get("priority").and_then(Value::as_u64) {
                    let priority_str = insert_priority(priority);
                    if !priority_str.is_empty() {
                        write!(new_line, "{} ", priority_str).unwrap();
                    }
                }

                // Board
                if let Some(label) = obj.get("label").and_then(Value::as_str) {
                    if let Some(finish_place) = obj.get("finish place").and_then(Value::as_bool) {
                        if finish_place {
                            write!(new_line, "<{}> ", label).unwrap();
                        } else {
                            write!(new_line, "{} ", label).unwrap();
                        }
                    } else {
                        write!(new_line, "{} ", label).unwrap();
                    }
                }

                // Color
                if let Some(color) = obj.get("color").and_then(Value::as_str) {
                    write!(new_line, "#{} ", color).unwrap();
                }

                // Tags
                if let Some(tags) = obj.get("tags").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for tag in tags {
                        if let Some(value) = tag.as_str() {
                            if !value.is_empty() && seen.insert(value) {
                                write!(new_line, "+{} ", value).unwrap();
                            }
                        }
                    }
                }

                // Contexts
                if let Some(contexts) = obj.get("contexts").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for context in contexts {
                        if let Some(value) = context.as_str() {
                            if !value.is_empty() && seen.insert(value) {
                                write!(new_line, "@{} ", value).unwrap();
                            }
                        }
                    }
                }

                // due
                if let Some(due) = obj.get("due").and_then(Value::as_object) {
                    let mut day: u8 = 0;
                    let mut month: u8 = 0;
                    let mut year: i32 = 0;
                    if let Some(d) = due.get("day").and_then(Value::as_u64) {
                        day = d as u8;
                    }
                    if let Some(m) = due.get("month").and_then(Value::as_u64) {
                        month = m as u8;
                    }
                    if let Some(y) = due.get("year").and_then(Value::as_i64) {
                        year = y as i32;
                    }

                    if day != 0 || month != 0 || year != 0 {
                        write!(
                            new_line,
                            "!{} ",
                            &board_settings::format_date(day, month, year)
                        )
                        .unwrap();
                    }
                }

                // links
                if let Some(links) = obj.get("links").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for link_item in links {
                        if let Some(arr) = link_item.as_array() {
                            if arr.len() >= 2 {
                                let name = arr[0].as_str().unwrap_or("");
                                let link = arr[1].as_str().unwrap_or("");

                                if !name.is_empty() && seen.insert(name.to_string()) {
                                    write!(new_line, "[{}]({}) ", name, link).unwrap();
                                }
                            }
                        }
                    }
                }

                // Inline comment
                if let Some(inline) = obj.get("inline_comment").and_then(Value::as_str)
                    && !ignore_comments
                {
                    if inline == "[Lexer]: Added this line"
                        && !code.ends_with("// [Lexer]: Added this line\n")
                    {
                        code.push_str("\n");
                    }
                    write!(new_line, "// {}", inline).unwrap();
                }

                write!(code, "{}\n", new_line.trim()).unwrap();
            }
        }
    }

    code.push_str("\n[TASKS]\n");
    let mut curr_board = String::new();
    if let Some(tasks) = json.get("tasks").and_then(Value::as_array) {
        for line in tasks {
            // Each line must be an object
            if let Some(obj) = line.as_object() {
                // Whitespace
                if obj.contains_key("__empty__") {
                    if !ignore_whitespaces {
                        code.push('\n');
                    }
                    continue;
                }
                // Comment
                else if let Some(comment) = obj.get("comment").and_then(Value::as_str) {
                    if !ignore_comments {
                        write!(code, "// {}\n", comment).unwrap();
                    }
                    continue;
                }

                new_line = String::new();

                // Description
                if let Some(description) = obj.get("description").and_then(Value::as_str) {
                    let lines: Vec<&str> = description.split('\n').collect();
                    let last_i = lines.len().saturating_sub(1);

                    for (i, line) in lines.iter().enumerate() {
                        if i == 0 {
                            if i == last_i {
                                write!(code, "\n/// {}", line).unwrap();
                            } else {
                                write!(code, "\n/// {}\n", line).unwrap();
                            }
                        } else if i == last_i {
                            write!(code, "/// {}", line).unwrap();
                        } else if !line.trim().is_empty() {
                            write!(code, "/// {}\n", line).unwrap();
                        }
                    }
                }

                // Priority
                if let Some(priority) = obj.get("priority").and_then(Value::as_u64) {
                    let priority_str = insert_priority(priority);
                    if !priority_str.is_empty() {
                        write!(new_line, "{} ", &priority_str).unwrap();
                    }
                }

                // Board
                if let Some(board) = obj.get("board").and_then(Value::as_str) {
                    if curr_board.is_empty() {
                        curr_board = board.to_string();
                    } else if curr_board != board {
                        curr_board = board.to_string();
                        code.push_str("\n");
                    }
                    write!(new_line, "{} - ", board).unwrap();
                }

                // Label
                if let Some(label) = obj.get("label").and_then(Value::as_str) {
                    write!(new_line, "{} ", label).unwrap();
                }

                // Color
                if let Some(color) = obj.get("color").and_then(Value::as_str) {
                    write!(new_line, "#{} ", color).unwrap();
                }

                // Tags
                if let Some(tags) = obj.get("tags").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for tag in tags {
                        if let Some(value) = tag.as_str() {
                            if !value.is_empty() && seen.insert(value) {
                                write!(new_line, "+{} ", value).unwrap();
                            }
                        }
                    }
                }

                // Contexts
                if let Some(contexts) = obj.get("contexts").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for context in contexts {
                        if let Some(value) = context.as_str() {
                            if !value.is_empty() && seen.insert(value) {
                                write!(new_line, "@{} ", value).unwrap();
                            }
                        }
                    }
                }

                // due
                if let Some(due) = obj.get("due").and_then(Value::as_object) {
                    let mut day: u8 = 0;
                    let mut month: u8 = 0;
                    let mut year: i32 = 0;
                    if let Some(d) = due.get("day").and_then(Value::as_u64) {
                        day = d as u8;
                    }
                    if let Some(m) = due.get("month").and_then(Value::as_u64) {
                        month = m as u8;
                    }
                    if let Some(y) = due.get("year").and_then(Value::as_i64) {
                        year = y as i32;
                    }

                    if day != 0 || month != 0 || year != 0 {
                        write!(
                            new_line,
                            "!{} ",
                            &board_settings::format_date(day, month, year)
                        )
                        .unwrap();
                    }
                }

                // links
                if let Some(links) = obj.get("links").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for link_item in links {
                        if let Some(arr) = link_item.as_array() {
                            if arr.len() >= 2 {
                                let name = arr[0].as_str().unwrap_or("");
                                let link = arr[1].as_str().unwrap_or("");

                                if !name.is_empty() && seen.insert(name.to_string()) {
                                    write!(new_line, "[{}]({}) ", name, link).unwrap();
                                }
                            }
                        }
                    }
                }

                // Inline comment
                if let Some(inline) = obj.get("inline_comment").and_then(Value::as_str)
                    && !ignore_comments
                {
                    if inline == "[Lexer]: Added this line"
                        && !code.ends_with("// [Lexer]: Added this line\n")
                    {
                        code.push_str("\n");
                    }
                    write!(new_line, "// {}", inline).unwrap();
                }

                write!(code, "{}\n", new_line.trim()).unwrap();
            }
        }
    }

    code
}

fn empty() -> Value {
    json!({ "__empty__": true })
}

fn split_sections(lines: &[String]) -> HashMap<String, Vec<String>> {
    let mut sections: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim();

        if line.is_empty() {
            if let Some(section) = &current {
                sections
                    .entry(section.clone())
                    .or_default()
                    .push(String::new());
            }
            continue;
        }

        if line.starts_with('[') {
            let (header, comment) = extract_inline_comment(line);
            if !comment.is_empty() {
                if let Some(section) = &current {
                    sections
                        .entry(section.clone())
                        .or_default()
                        .push(format!("// {comment}"));
                }
            }

            if header.ends_with(']') {
                let mut name = header.trim_matches(&['[', ']'][..]).to_ascii_uppercase();

                if !name.is_empty() {
                    if name == "PROFILE" {
                        name = "PROFILES".to_string();
                    } else if name == "BOARD" {
                        name = "BOARDS".to_string();
                    } else if name == "TASK" {
                        name = "TASKS".to_string();
                    }

                    if name == "PROFILES" || name == "BOARDS" || name == "TASKS" {
                        current = Some(name.clone());
                        sections.entry(name).or_default();
                    } else if let Some(section) = &current {
                        sections
                            .entry(section.clone())
                            .or_default()
                            .push(line.to_string());
                    }
                } else if let Some(section) = &current {
                    sections
                        .entry(section.clone())
                        .or_default()
                        .push(line.to_string());
                }
            }
        } else if let Some(section) = &current {
            sections
                .entry(section.clone())
                .or_default()
                .push(line.to_string());
        }
    }

    sections
}

fn parse_profiles(lines: Option<&Vec<String>>) -> Vec<Value> {
    let mut profiles_object = Vec::new();
    let mut seen_labels: HashSet<String> = HashSet::new();
    let mut description = String::new();

    for line in lines.into_iter().flatten() {
        let line = line.trim();

        if line.is_empty() {
            profiles_object.push(empty());
            continue;
        }

        let mut entry = Map::new();

        if is_doc_comment(line) {
            description.push_str(&line[3..]);
            continue;
        } else if full_line_comment(line) {
            entry.insert("comment".to_string(), json!(line[2..].trim()));
            profiles_object.push(Value::Object(entry));
            continue;
        }

        if !description.is_empty() {
            description = description
                .replace("\\\\n", "\n")
                .replace("\\n", "\n")
                .lines()
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n");
            entry.insert("description".to_string(), json!(description));
            description.clear();
        }

        // Inline comment
        let (label, comment) = extract_inline_comment(line);

        if !comment.is_empty() {
            entry.insert("inline_comment".to_string(), json!(comment));
        }

        entry.insert("label".to_string(), json!(label));

        // Deduplicate profiles by label
        if seen_labels.insert(label.clone()) {
            profiles_object.push(Value::Object(entry));
        }
    }

    profiles_object
}

fn parse_boards(lines: Option<&Vec<String>>) -> Vec<Value> {
    let mut boards_object = Vec::new();
    let mut seen_labels: HashSet<String> = HashSet::new();
    let mut needs_finish_place = true;
    let mut description = String::new();

    for line in lines.into_iter().flatten() {
        let line = line.trim();

        // empty line → empty value (still counts as something)
        if line.is_empty() {
            boards_object.push(empty());
            continue;
        }

        let mut entry = Map::new();

        if is_doc_comment(line) {
            description.push_str(&line[3..]);
            continue;
        }
        // full-line comment
        else if full_line_comment(line) {
            entry.insert("comment".to_string(), json!(line[2..].trim()));
            boards_object.push(Value::Object(entry));
            continue;
        }

        if !description.is_empty() {
            description = description
                .replace("\\\\n", "\n")
                .replace("\\n", "\n")
                .lines()
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n");
            entry.insert("description".to_string(), json!(description));
            description.clear();
        }

        let links = extract_links(line);
        if !links.is_empty() {
            entry.insert("links".to_string(), json!(links));
        }

        let line = remove_links(line);
        let (line, comment) = extract_inline_comment(&line);
        if !comment.is_empty() {
            entry.insert("inline_comment".to_string(), json!(comment));
        }

        let (priority, line) = extract_priority(&line);
        entry.insert("priority".to_string(), json!(priority));

        if !is_board_line(&line) {
            continue;
        }

        let label = board_label(&line);
        if !seen_labels.insert(label.clone()) {
            continue; // duplicate label
        }

        if needs_finish_place && label.starts_with("<") && label.ends_with(">") {
            entry.insert("label".to_string(), json!(label[1..label.len() - 1].trim()));
            entry.insert("finish place".to_string(), json!(true));
            needs_finish_place = false;
        } else {
            entry.insert("label".to_string(), json!(label));
        }

        let tags = extract_prefixed_words(&line, '+');
        if !tags.is_empty() {
            entry.insert("tags".to_string(), json!(tags));
        }

        let contexts = extract_prefixed_words(&line, '@');
        if !contexts.is_empty() {
            entry.insert("contexts".to_string(), json!(contexts));
        }

        let due = extract_due(&line);
        if !due.is_null() {
            entry.insert("due".to_string(), due);
        }

        let color = extract_color(&line);
        if !color.is_empty() {
            entry.insert("color".to_string(), json!(color));
        }

        // only insert if there's something besides priority / label
        let meaningful = entry.keys().any(|k| k != "priority" && k != "label");

        if meaningful {
            boards_object.push(Value::Object(entry));
        }
    }

    if needs_finish_place {
        // check the last object with a "label" and set finish place if found
        for board in boards_object.iter_mut().rev() {
            if board.as_object().and_then(|o| o.get("label")).is_some() {
                board
                    .as_object_mut()
                    .unwrap()
                    .insert("finish place".to_string(), json!(true));
                break;
            }
        }
    }

    if boards_object.last() != Some(&empty()) {
        boards_object.push(empty());
    }

    boards_object
}

fn parse_tasks(lines: Option<&Vec<String>>) -> Vec<Value> {
    let mut tasks = Vec::new();
    let mut description = String::new();

    for line in lines.into_iter().flatten() {
        let line = line.trim();

        if line.is_empty() {
            tasks.push(empty());
            continue;
        }

        let mut entry = Map::new();

        if is_doc_comment(line) {
            description.push_str(&line[3..]);
            continue;
        }
        // full-line comment always counts
        else if full_line_comment(line) {
            entry.insert("comment".to_string(), json!(line[2..].trim()));
            tasks.push(Value::Object(entry));
            continue;
        }

        let links = extract_links(line);
        if !links.is_empty() {
            entry.insert("links".to_string(), json!(links));
        }

        if !description.is_empty() {
            description = description
                .replace("\\\\n", "\n")
                .replace("\\n", "\n")
                .lines()
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n");
            entry.insert("description".to_string(), json!(description));
            description.clear();
        }

        let line = remove_links(line);
        let (line, comment) = extract_inline_comment(&line);
        if !comment.is_empty() {
            entry.insert("inline_comment".to_string(), json!(comment));
        }

        let (priority, line) = extract_priority(&line);
        entry.insert("priority".to_string(), json!(priority));

        let (board, line) = if let Some((board, rest)) = line.split_once('-') {
            (board.trim().to_string(), rest.trim().to_string())
        } else {
            (String::new(), line)
        };

        entry.insert("board".to_string(), json!(board));
        entry.insert("label".to_string(), json!(extract_label(&line)));

        let tags = extract_prefixed_words(&line, '+');
        if !tags.is_empty() {
            entry.insert("tags".to_string(), json!(tags));
        }

        let contexts = extract_prefixed_words(&line, '@');
        if !contexts.is_empty() {
            entry.insert("contexts".to_string(), json!(contexts));
        }

        let due = extract_due(&line);
        if !due.is_null() {
            entry.insert("due".to_string(), due);
        }

        let color = extract_color(&line);
        if !color.is_empty() {
            entry.insert("color".to_string(), json!(color));
        }

        // only insert if there's something besides priority / label / board
        let meaningful = entry
            .keys()
            .any(|k| k != "priority" && k != "label" && k != "board");

        if meaningful {
            tasks.push(Value::Object(entry));
        }
    }

    tasks
}

fn is_doc_comment(line: &str) -> bool {
    if line.trim_start().starts_with("///") {
        return true;
    }
    false
}

fn full_line_comment(line: &str) -> bool {
    if line.trim_start().starts_with("///") {
        return false;
    } else if line.trim_start().starts_with("//") {
        return true;
    }
    false
}

fn extract_inline_comment(line: &str) -> (String, String) {
    if let Some(index) = line.find("//") {
        (
            line[..index].trim_end().to_string(),
            line[index + 2..].trim().to_string(),
        )
    } else {
        (line.to_string(), String::new())
    }
}

fn extract_priority(line: &str) -> (usize, String) {
    let trimmed = line.trim_start();
    let star_count = trimmed.chars().take_while(|ch| *ch == '*').count();
    if star_count > 0 {
        return (star_count, trimmed[star_count..].trim_start().to_string());
    }

    let digit_count = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count > 0 && trimmed[digit_count..].starts_with('*') {
        let priority = trimmed[..digit_count].parse().unwrap_or(0);
        return (
            priority,
            trimmed[digit_count + 1..].trim_start().to_string(),
        );
    }

    (0, line.to_string())
}

fn insert_priority(value: u64) -> String {
    if value == 0 {
        return String::new();
    } else if value > 3 {
        value.to_string() + "*"
    } else {
        "*".repeat(value.max(0) as usize)
    }
}

fn is_board_line(line: &str) -> bool {
    line.rsplit_once('#')
        .map(|(_, color_and_more)| {
            color_and_more
                .chars()
                .take(6)
                .all(|ch| ch.is_ascii_hexdigit())
                && color_and_more.chars().count() >= 6
        })
        .unwrap_or(false)
}

fn board_label(line: &str) -> String {
    line.rsplit_once('#')
        .map(|(label, _)| label.trim().to_string())
        .unwrap_or_default()
}

fn extract_label(line: &str) -> String {
    let line = extract_inline_comment(line).0;

    let re_links = Regex::new(r"\[[^\]]+\]\([^)]+\)").unwrap();
    let line = re_links.replace_all(&line, "");

    let re_rgb = Regex::new(r"rgba?\([^)]+\)").unwrap();
    let line = re_rgb.replace_all(&line, "");

    let re_hex = Regex::new(r"#[0-9a-fA-F]{6,8}").unwrap();
    let line = re_hex.replace_all(&line, "");

    let re_tags = Regex::new(r"\+[a-zA-Z0-9_]+").unwrap();
    let line = re_tags.replace_all(&line, "");

    let re_ctx = Regex::new(r"@[a-zA-Z0-9_]+").unwrap();
    let line = re_ctx.replace_all(&line, "");

    let re_due = Regex::new(r"![0-9/]+").unwrap();
    let line = re_due.replace_all(&line, "");

    line.trim().to_string()
}

fn extract_prefixed_words(line: &str, prefix: char) -> Vec<String> {
    let re_str = format!(r"{}([a-zA-Z0-9_]+)", regex::escape(&prefix.to_string()));
    let re = Regex::new(&re_str).unwrap();
    re.captures_iter(line)
        .map(|cap| cap[1].to_string())
        .collect()
}

fn extract_due(line: &str) -> Value {
    let re = Regex::new(r"!(\d{1,2})[/-](\d{1,2})[/-](\d{2,4})").unwrap();
    if let Some(cap) = re.captures(line) {
        if let (Ok(day), Ok(month), Ok(year)) = (
            cap[1].parse::<u8>(),
            cap[2].parse::<u8>(),
            cap[3].parse::<i32>(),
        ) {
            return json!({
                "day": day,
                "month": month,
                "year": year
            });
        }
    }
    Value::Null
}

fn extract_color(line: &str) -> String {
    if let Some(color) = extract_hex_color(line) {
        return color;
    }

    if let Some(color) = extract_rgb_like(line, "rgba") {
        return color;
    }

    extract_rgb_like(line, "rgb").unwrap_or_default()
}

fn extract_hex_color(line: &str) -> Option<String> {
    let re = Regex::new(r"#([0-9a-fA-F]{6,8})").unwrap();
    re.captures(line).map(|cap| cap[1].to_ascii_lowercase())
}

fn extract_rgb_like(line: &str, name: &str) -> Option<String> {
    let re_str = format!(r"{}\(([^)]+)\)", name);
    let re = Regex::new(&re_str).unwrap();
    let cap = re.captures(line)?;
    let args: Vec<&str> = cap[1].split(',').map(str::trim).collect();

    if (name == "rgb" && args.len() != 3) || (name == "rgba" && args.len() != 4) {
        return None;
    }

    let red = clamp(args[0].parse::<i64>().ok()?, 0, 255) as u8;
    let green = clamp(args[1].parse::<i64>().ok()?, 0, 255) as u8;
    let blue = clamp(args[2].parse::<i64>().ok()?, 0, 255) as u8;
    let alpha = if name == "rgba" {
        clamp_float(args[3].parse::<f64>().ok()?, 0.0, 1.0)
    } else {
        1.0
    };

    let mut color = format!("{red:02x}{green:02x}{blue:02x}");
    if alpha < 1.0 {
        color.push_str(&format!("{:02x}", (alpha * 255.0).round() as u8));
    }

    Some(color)
}

fn extract_links(line: &str) -> Vec<Vec<String>> {
    let re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    re.captures_iter(line)
        .map(|cap| vec![cap[1].to_string(), cap[2].to_string()])
        .collect()
}

fn remove_links(line: &str) -> String {
    let re = Regex::new(r"\[[^\]]+\]\([^)]+\)").unwrap();
    re.replace_all(line, "").into_owned()
}

fn clamp(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn clamp_float(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use serde_json::Value;

    macro_rules! assert_eq_code {
        ($left:expr, $right:expr, $lang:expr) => {
            if $left != $right {
                let (result, expected) = if $lang == "json" {
                    (
                        serde_json::to_string_pretty(&$left).unwrap(),
                        serde_json::to_string_pretty(&$right).unwrap(),
                    )
                } else {
                    (format!("{}", $left), format!("{}", $right))
                };

                let diff = diff_lines(&result, &expected);

                panic!(
                    "\n====================\n\
                     Result (with diff):\n\n\
                     ```{lang}\n{diff}\n```\n\
                     --------------------\n\
                     Expected:\n\n\
                     ```{lang}\n{expected}\n```\n\
                     ====================\n",
                    lang = $lang,
                    diff = diff,
                    expected = expected
                );
            }
        };
    }

    fn diff_lines(left: &str, right: &str) -> String {
        let left_lines: Vec<&str> = left.lines().collect();
        let right_lines: Vec<&str> = right.lines().collect();

        let mut out = String::new();
        let max = left_lines.len().max(right_lines.len());

        for i in 0..max {
            match (left_lines.get(i), right_lines.get(i)) {
                (Some(l), Some(r)) if l == r => {
                    out.push_str(l);
                }
                (Some(l), Some(_r)) => {
                    out.push_str(&format!("\x1b[31m{}  -> diff here\x1b[0m", l));
                }
                (None, Some(r)) => {
                    out.push_str(&format!("\x1b[31m-- line should be here --\n{}\x1b[0m", r));
                    continue;
                }
                (Some(l), None) => {
                    out.push_str(&format!("\x1b[31m{}  -> line shouldn't exist\x1b[0m", l));
                }
                (None, None) => {}
            }
            out.push('\n');
        }

        out
    }

    #[test]
    fn parses_board_example_correctly() {
        let source = include_str!("../../../test/board/example.bd");
        let expected: Value =
            serde_json::from_str(include_str!("../../../test/board/example.bdj")).unwrap();

        let parsed = Lexer::new(source).get_json(false, false);

        assert_eq_code!(parsed, expected, "json");
    }

    #[test]
    fn creates_clear_code_correctly() {
        let source = include_str!("../../../test/board/example.bd");
        let expected = include_str!("../../../test/board/example_cleaned.bd");

        let parsed = Lexer::new(source).clear_code(false);

        assert_eq_code!(parsed, expected, "bd");
    }
}
