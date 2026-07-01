//! Compiles and decompiles board code

use regex::Regex;
use serde_json::{
    Map,
    Value::{self},
    json,
};
use std::collections::HashSet;
use std::collections::{HashMap, hash_map::Values};

pub fn test() -> String {
    "this is a test for board-lang dependencie".to_string()
}

pub struct Lexer<'a> {
    input: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub fn tokenize(&self, ignore_whitespace: bool, ignore_comments: bool) -> Vec<String> {
        if ignore_whitespace {
            let mut tokens = Vec::new();
            for line in self.input.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if ignore_comments && line.starts_with("//") {
                    continue;
                }
                tokens.extend(line.split_whitespace().map(String::from));
            }
            tokens
        } else {
            self.input.split('\n').map(String::from).collect()
        }
    }

    pub fn get_json(&self, ignore_whitespace: bool, ignore_comments: bool) -> Value {
        let sections = split_sections(&&self.tokenize(ignore_whitespace, ignore_comments));

        json!({
            "profiles": parse_profiles(sections.get("PROFILES")),
            "tags": {},
            "context": {},
            "boards": parse_boards(sections.get("BOARDS")),
            "tasks": parse_tasks(sections.get("TASKS")),
        })
    }

    /// Checks the current .bd code and clears it
    pub fn clear_code(&self, ignore_comments: bool) -> String {
        let json = self.get_json(true, ignore_comments);
        return create_code_from_json(json, true, ignore_comments);
    }
}

/// Creates code from json,
/// check ../examples/board/example.json, ../examples/board/example.bd and ../examples/board/example_cleaned.bd
/// for examples
pub fn create_code_from_json(
    json: Value,
    ignore_whitespaces: bool,
    ignore_comments: bool,
) -> String {
    // Serialize initial value
    let mut json_str = match serde_json::to_string(&json) {
        Ok(s) => s,
        Err(_) => {
            return r#"{"profiles":[],"tags":{},"context":{},"boards":[],"tasks":[]}"#.to_string();
        }
    };

    // Remove `"inline_comment": <something>,`
    if ignore_comments {
        json_str = remove_inline_comments(json_str);
    }

    // Remove empty placeholder objects
    json_str = json_str.replace(r#"{"__empty__":true},"#, "");

    // Parse back to JSON to normalize / validate
    let json_new_value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => json, // fallback to original value
    };

    // Create code based on json
    let mut code = String::new();

    // Ensure `profiles` is an array
    code.push_str("[PROFILES]\n");
    if let Some(profiles) = json_new_value.get("profiles").and_then(Value::as_array) {
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
                        code.push_str("//");
                        code.push_str(comment);
                        code.push('\n');
                    }
                    continue;
                }

                // Label
                if let Some(label) = obj.get("label").and_then(Value::as_str) {
                    code.push_str(label);
                }

                // Inline comment
                if let Some(inline) = obj.get("inline_comment").and_then(Value::as_str)
                    && !ignore_comments
                {
                    code.push_str("// ");
                    code.push_str(inline);
                }

                code.push('\n');
            }
        }
    }

    code.push_str("\n[BOARDS]\n");
    if let Some(boards) = json_new_value.get("boards").and_then(Value::as_array) {
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
                        code.push_str("// ");
                        code.push_str(comment);
                        code.push('\n');
                    }
                    continue;
                }

                // Priority
                if let Some(priority) = obj.get("priority").and_then(Value::as_str) {
                    if let Ok(value) = priority.parse::<i32>() {
                        code.push_str(&insert_priority(value));
                        code.push_str(" ");
                    }
                }

                // Board
                if let Some(label) = obj.get("label").and_then(Value::as_str) {
                    code.push_str(label);
                    code.push_str(" ");
                }

                // Color
                if let Some(color) = obj.get("label").and_then(Value::as_str) {
                    code.push_str("#");
                    code.push_str(color);
                    code.push_str(" ");
                }

                // Tags
                if let Some(tags) = obj.get("tags").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for tag in tags {
                        if let Some(value) = tag.as_str() {
                            if !value.is_empty() && seen.insert(value) {
                                code.push_str("+");
                                code.push_str(value);
                                code.push_str(" ");
                            }
                        }
                    }
                }

                // Contexts
                if let Some(contexts) = obj.get("tags").and_then(Value::as_array) {
                    let mut seen = HashSet::new();

                    for context in contexts {
                        if let Some(value) = context.as_str() {
                            if !value.is_empty() && seen.insert(value) {
                                code.push_str("@");
                                code.push_str(value);
                                code.push_str(" ");
                            }
                        }
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
                                    code.push_str("(");
                                    code.push_str(name);
                                    code.push_str(")[");
                                    code.push_str(link);
                                    code.push_str("]");
                                }
                            }
                        }
                    }
                }

                // Inline comment
                if let Some(inline) = obj.get("inline_comment").and_then(Value::as_str)
                    && !ignore_comments
                {
                    code.push_str("// ");
                    code.push_str(inline);
                }

                code.push('\n');
            }
        }
    }

    serde_json::to_string(&json_new_value).unwrap_or_default()
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
                        .push(format!("//{comment}"));
                }
            }

            if header.ends_with(']') {
                let name = header.trim_matches(&['[', ']'][..]).to_ascii_uppercase();
                current = Some(name.clone());
                sections.entry(name).or_default();
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

    for line in lines.into_iter().flatten() {
        let line = line.trim();

        if line.is_empty() {
            profiles_object.push(empty());
            continue;
        }

        let mut entry = Map::new();

        // Full-line comment
        if let Some(comment) = full_line_comment(line) {
            entry.insert("comment".to_string(), json!(comment));
            profiles_object.push(Value::Object(entry));
            continue;
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

    for line in lines.into_iter().flatten() {
        let line = line.trim();
        if line.is_empty() {
            boards_object.push(empty());
            continue;
        }

        let mut entry = Map::new();
        if let Some(comment) = full_line_comment(line) {
            entry.insert("comment".to_string(), json!(comment));
            boards_object.push(Value::Object(entry));
            continue;
        }

        let links = extract_links(line);
        entry.insert("links".to_string(), json!(links));

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
            continue; // already has the same label
        }

        entry.insert("label".to_string(), json!(label));
        entry.insert(
            "tags".to_string(),
            json!(extract_prefixed_words(&line, '+')),
        );
        entry.insert(
            "contexts".to_string(),
            json!(extract_prefixed_words(&line, '@')),
        );
        entry.insert("due".to_string(), extract_due(&line));
        entry.insert("color".to_string(), json!(extract_color(&line)));
        entry.insert("finish place".to_string(), json!(false)); // TODO

        boards_object.push(Value::Object(entry));
    }

    boards_object
}

fn parse_tasks(lines: Option<&Vec<String>>) -> Vec<Value> {
    let mut tasks = Vec::new();

    for line in lines.into_iter().flatten() {
        let line = line.trim();
        if line.is_empty() {
            tasks.push(empty());
            continue;
        }

        let mut entry = Map::new();
        if let Some(comment) = full_line_comment(line) {
            entry.insert("comment".to_string(), json!(comment));
            tasks.push(Value::Object(entry));
            continue;
        }

        let links = extract_links(line);
        entry.insert("links".to_string(), json!(links));

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
        entry.insert(
            "tags".to_string(),
            json!(extract_prefixed_words(&line, '+')),
        );
        entry.insert(
            "contexts".to_string(),
            json!(extract_prefixed_words(&line, '@')),
        );
        entry.insert("due".to_string(), extract_due(&line));
        entry.insert("color".to_string(), json!(extract_color(&line)));

        tasks.push(Value::Object(entry));
    }

    tasks
}

fn full_line_comment(line: &str) -> Option<String> {
    line.strip_prefix("//")
        .map(|comment| comment.trim().to_string())
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

fn insert_priority(value: i32) -> String {
    if value > 3 {
        value.to_string()
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
    let line = remove_links(&line);
    let line = remove_rgb_functions(&line);
    let mut output = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            '+' | '@' => {
                index += 1;
                while index < chars.len() && is_word_char(chars[index]) {
                    index += 1;
                }
            }
            '!' => {
                index += 1;
                while index < chars.len() && is_due_char(chars[index]) {
                    index += 1;
                }
            }
            '#' => {
                index += 1;
                while index < chars.len() && chars[index].is_ascii_hexdigit() {
                    index += 1;
                }
            }
            ch => {
                output.push(ch);
                index += 1;
            }
        }
    }

    output.trim().to_string()
}

fn extract_prefixed_words(line: &str, prefix: char) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut values = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == prefix {
            index += 1;
            let start = index;
            while index < chars.len() && is_word_char(chars[index]) {
                index += 1;
            }
            if start < index {
                values.push(chars[start..index].iter().collect());
            }
        } else {
            index += 1;
        }
    }

    values
}

fn extract_due(line: &str) -> Value {
    let chars: Vec<char> = line.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '!' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && is_due_char(chars[end]) {
                end += 1;
            }

            let date: String = chars[start..end].iter().collect();
            let parts: Vec<&str> = date.split('/').collect();
            if parts.len() == 3 {
                if let (Ok(day), Ok(month), Ok(year)) = (
                    parts[0].parse::<u64>(),
                    parts[1].parse::<u64>(),
                    parts[2].parse::<u64>(),
                ) {
                    return json!({
                        "day": day,
                        "month": month,
                        "year": year
                    });
                }
            }
        }
        index += 1;
    }

    json!({})
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
    let chars: Vec<char> = line.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '#' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && chars[end].is_ascii_hexdigit() {
                end += 1;
            }

            if end - start >= 6 {
                let mut color: String = chars[start..start + 6].iter().collect();
                if end - start >= 8 {
                    color.push_str(&chars[start + 6..start + 8].iter().collect::<String>());
                }
                return Some(color.to_ascii_lowercase());
            }
        }
        index += 1;
    }

    None
}

fn extract_rgb_like(line: &str, name: &str) -> Option<String> {
    let start = line.find(&format!("{name}("))?;
    let args_start = start + name.len() + 1;
    let args_end = line[args_start..].find(')')? + args_start;
    let args: Vec<&str> = line[args_start..args_end]
        .split(',')
        .map(str::trim)
        .collect();

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
    let mut links = Vec::new();
    let mut rest = line;

    while let Some(title_start) = rest.find('[') {
        let after_title_start = &rest[title_start + 1..];
        let Some(title_end) = after_title_start.find(']') else {
            break;
        };
        let title = &after_title_start[..title_end];
        let after_title = &after_title_start[title_end + 1..];

        if !after_title.starts_with('(') {
            rest = after_title;
            continue;
        }

        let Some(url_end) = after_title[1..].find(')') else {
            break;
        };
        let url = &after_title[1..url_end + 1];
        links.push(vec![title.to_string(), url.to_string()]);
        rest = &after_title[url_end + 2..];
    }

    links
}

fn remove_links(line: &str) -> String {
    let mut output = String::new();
    let mut rest = line;

    while let Some(title_start) = rest.find('[') {
        let before = &rest[..title_start];
        let after_title_start = &rest[title_start + 1..];
        let Some(title_end) = after_title_start.find(']') else {
            break;
        };
        let after_title = &after_title_start[title_end + 1..];

        if !after_title.starts_with('(') {
            output.push_str(before);
            output.push('[');
            rest = after_title_start;
            continue;
        }

        let Some(url_end) = after_title[1..].find(')') else {
            break;
        };

        output.push_str(before);
        rest = &after_title[url_end + 2..];
    }

    output.push_str(rest);
    output
}

fn remove_rgb_functions(line: &str) -> String {
    let mut output = line.to_string();
    for name in ["rgba", "rgb"] {
        while let Some(start) = output.find(&format!("{name}(")) {
            let Some(end) = output[start..].find(')') else {
                break;
            };
            output.replace_range(start..start + end + 1, "");
        }
    }
    output
}

fn is_word_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_due_char(ch: char) -> bool {
    ch.is_ascii_digit() || ch == '/'
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

    #[test]
    fn parses_board_example_correctly() {
        let source = include_str!("../../../examples/board/example.bd");
        let expected: Value =
            serde_json::from_str(include_str!("../../../examples/board/example.json")).unwrap();

        let parsed = Lexer::new(source).get_json(false, false).unwrap();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn creates_clear_code_correctly() {
        let source = include_str!("../../../examples/board/example.bd");
        let expected: Value =
            serde_json::from_str(include_str!("../../../examples/board/example_cleaned.bd"))
                .unwrap();

        let parsed = Lexer::new(source).clear_code(false).unwrap();

        assert_eq!(parsed, expected);
    }
}

fn remove_inline_comments(mut s: String) -> String {
    let re = Regex::new(r#""inline_comment"\s*:\s*"[^"]*",?"#).unwrap();
    s = re.replace_all(&s, "").to_string();
    s
}
