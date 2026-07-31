// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tree to indented text, plus compaction helpers.

use super::super::options::*;

use super::types::*;

pub(crate) fn render_tree(
    nodes: &[TreeNode],
    idx: usize,
    indent: usize,
    output: &mut String,
    options: &SnapshotOptions,
) {
    let node = &nodes[idx];

    // Reduce unnecessary indentation and rendering
    if node.role.is_empty()
        || (node.role == "generic" && !node.has_ref && node.children.len() <= 1)
        || (node.role == "StaticText" && node.name.replace(INVISIBLE_CHARS, "").is_empty())
    {
        // Ignored node -- still render children
        for &child in &node.children {
            render_tree(nodes, child, indent, output, options);
        }
        return;
    }

    if let Some(max_depth) = options.depth {
        if indent > max_depth {
            return;
        }
    }

    let role = &node.role;

    // Skip root WebArea wrapper
    if role == "RootWebArea" || role == "WebArea" {
        for &child in &node.children {
            render_tree(nodes, child, indent, output, options);
        }
        return;
    }

    if options.interactive && !node.has_ref {
        // In interactive mode, skip non-interactive but render children
        for &child in &node.children {
            render_tree(nodes, child, indent, output, options);
        }
        return;
    }

    let prefix = "  ".repeat(indent);
    let mut line = format!("{prefix}- {role}");

    // Use ARIA name if available, only fall back to cursor-interactive textContent in interactive mode since their visible text in child nodes is filtered out
    let unescaped_display_name = if !node.name.is_empty() {
        &node.name
    } else if options.interactive {
        if let Some(ref ci) = node.cursor_info {
            &ci.text
        } else {
            &node.name
        }
    } else {
        &node.name
    };
    if !unescaped_display_name.is_empty() {
        if let Ok(display_name) = serde_json::to_string(&unescaped_display_name) {
            line.push_str(&format!(" {}", display_name.replace(INVISIBLE_CHARS, "")));
        }
    }

    // Properties
    let mut attrs = Vec::new();

    if let Some(level) = node.level {
        attrs.push(format!("level={level}"));
    }
    if let Some(ref checked) = node.checked {
        attrs.push(format!("checked={checked}"));
    }
    if let Some(expanded) = node.expanded {
        attrs.push(format!("expanded={expanded}"));
    }
    if let Some(selected) = node.selected {
        if selected {
            attrs.push("selected".to_string());
        }
    }
    if let Some(disabled) = node.disabled {
        if disabled {
            attrs.push("disabled".to_string());
        }
    }
    if let Some(required) = node.required {
        if required {
            attrs.push("required".to_string());
        }
    }

    if let Some(ref ref_id) = node.ref_id {
        attrs.push(format!("ref={ref_id}"));
    }

    if let Some(ref url) = node.url {
        attrs.push(format!("url={url}"));
    }

    if !attrs.is_empty() {
        line.push_str(&format!(" [{}]", attrs.join(", ")));
    }

    // Add cursor-interactive kind & hints
    if let Some(ref cursor_info) = node.cursor_info {
        line.push_str(&format!(
            " {} [{}]",
            cursor_info.kind,
            cursor_info.hints.join(", ")
        ));
    }

    // Value
    if let Some(ref val) = node.value_text {
        if !val.is_empty() && val != &node.name {
            line.push_str(&format!(": {val}"));
        }
    }

    output.push_str(&line);
    output.push('\n');

    for &child in &node.children {
        render_tree(nodes, child, indent + 1, output, options);
    }
}

pub(crate) fn compact_tree(tree: &str, interactive: bool) -> String {
    let lines: Vec<&str> = tree.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let mut keep = vec![false; lines.len()];

    for (i, line) in lines.iter().enumerate() {
        if line.contains("ref=") || line.contains(": ") {
            keep[i] = true;
            // Mark ancestors
            let my_indent = count_indent(line);
            for j in (0..i).rev() {
                let ancestor_indent = count_indent(lines[j]);
                if ancestor_indent < my_indent {
                    keep[j] = true;
                    if ancestor_indent == 0 {
                        break;
                    }
                }
            }
        }
    }

    let result: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, line)| *line)
        .collect();

    let output = result.join("\n");
    if output.trim().is_empty() && interactive {
        return "(no interactive elements)".to_string();
    }
    output
}

pub(crate) fn count_indent(line: &str) -> usize {
    let trimmed = line.trim_start();
    (line.len() - trimmed.len()) / 2
}
