//! Build script for `browser-automation-cli`.
//!
//! Responsibilities (offline only — no network, no absolute host paths as inputs):
//! 1. Embed `GIT_SHA` and `BUILD_TIMESTAMP` for `version --json`.
//! 2. Generate CDP domain stubs from `cdp-protocol/*.json` into `OUT_DIR`.
//!
//! Rerun triggers use `cargo:rerun-if-changed` / `cargo:rerun-if-env-changed` only.
//!
//! # Macro / codegen policy (`rules_rust_macros`)
//!
//! Prefer **`build.rs` + `include!`** over a custom `proc-macro` crate when the
//! input is external data (CDP protocol JSON). Reasons:
//! - generator is ordinary Rust (debuggable, unit-testable without trybuild);
//! - no `syn`/`quote` dependency surface for consumers;
//! - expansion is deterministic for a fixed protocol snapshot;
//! - the library crate only emits events/types — it never installs macros globally.
//!
//! String concatenation is intentional here (build scripts are not proc-macro
//! crates). Do **not** introduce `macro_rules!` or a workspace proc-macro just to
//! emit these stubs.

// A build script's stdout IS its API: Cargo parses `cargo:` directives from it,
// and there is no other channel. The package-wide `print_stdout` / `print_stderr`
// lints exist to keep agent-consumable output funnelled through `src/output.rs`,
// which has no bearing on a process Cargo runs and reads itself.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Build metadata for `version` / agent diagnostics (rules_rust_cli_com_clap).
    emit_git_build_meta();
    emit_source_hash();

    let protocol_dir = Path::new("cdp-protocol");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set by cargo for every build script");
    let out_path = Path::new(&out_dir).join("cdp_generated.rs");

    let browser_path = protocol_dir.join("browser_protocol.json");
    let js_path = protocol_dir.join("js_protocol.json");

    if !browser_path.exists() && !js_path.exists() {
        fs::write(
            &out_path,
            "// No protocol JSON files found in cdp-protocol/\n",
        )
        .unwrap_or_else(|e| panic!("write empty CDP stub to {}: {e}", out_path.display()));
        return;
    }

    let mut all_domains: Vec<Domain> = Vec::new();

    for path in [&browser_path, &js_path] {
        if !path.exists() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read CDP protocol {}: {e}", path.display()));
        let protocol: ProtocolSpec = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(e) => {
                // `println!`, not `eprintln!`: Cargo parses `cargo:` directives
                // from the build script's STDOUT. Emitted on stderr this warning
                // reached nobody, so a malformed protocol snapshot degraded the
                // generated stubs in silence.
                println!("cargo:warning=Failed to parse {}: {}", path.display(), e);
                continue;
            }
        };
        all_domains.extend(protocol.domains);
    }

    // Collect all known type IDs per domain for cross-domain resolution
    let mut domain_types: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
    for domain in &all_domains {
        let mut types = HashSet::new();
        for td in &domain.types {
            types.insert(td.id.clone());
        }
        domain_types.insert(domain.domain.clone(), types);
    }

    // Known recursive struct fields that need Box wrapping
    let recursive_fields: HashSet<(&str, &str, &str)> = [
        ("DOM", "Node", "contentDocument"),
        ("DOM", "Node", "templateContent"),
        ("DOM", "Node", "importedDocument"),
        ("Accessibility", "AXNode", "sources"),
        ("Runtime", "StackTrace", "parent"),
    ]
    .into_iter()
    .collect();

    let mut output = String::new();
    // GAP-046: the generated wire surface carries its own documentation policy.
    // Emitting the header here (instead of a comment in build.rs) keeps the
    // justification attached to the artifact a reader actually opens.
    output.push_str(concat!(
        "// CDP wire types generated from `cdp-protocol/*.json` by `build.rs`.\n",
        "//\n",
        "// DO NOT EDIT: this file lives in `OUT_DIR` and is rewritten on every\n",
        "// build. Change `build.rs` or the protocol JSON instead.\n",
        "//\n",
        "// Each item mirrors a Chrome DevTools Protocol type 1:1; the\n",
        "// authoritative documentation is the protocol itself:\n",
        "// https://chromedevtools.github.io/devtools-protocol/\n",
        "//\n",
        "// `missing_docs` is allowed for these items by the including module\n",
        "// (`src/native/cdp/types/mod.rs`), which carries the justification.\n",
        "// Inner attributes cannot live here: `include!` splices items only.\n",
        "\n",
        "use serde::{Deserialize, Serialize};\n\n",
    ));

    for domain in &all_domains {
        generate_domain(domain, &domain_types, &recursive_fields, &mut output);
    }

    fs::write(&out_path, &output)
        .unwrap_or_else(|e| panic!("write generated CDP stubs to {}: {e}", out_path.display()));
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct ProtocolSpec {
    domains: Vec<Domain>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
struct Domain {
    domain: String,
    #[serde(default)]
    types: Vec<TypeDef>,
    #[serde(default)]
    commands: Vec<Command>,
    #[serde(default)]
    events: Vec<Event>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
struct TypeDef {
    id: String,
    #[serde(rename = "type", default)]
    type_kind: String,
    #[serde(default)]
    properties: Vec<Property>,
    #[serde(rename = "enum", default)]
    enum_values: Vec<String>,
    #[serde(default)]
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
struct Command {
    name: String,
    #[serde(default)]
    parameters: Vec<Property>,
    #[serde(default)]
    returns: Vec<Property>,
    #[serde(default)]
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
struct Event {
    name: String,
    #[serde(default)]
    parameters: Vec<Property>,
    #[serde(default)]
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
struct Property {
    name: String,
    #[serde(rename = "type", default)]
    type_kind: Option<String>,
    #[serde(rename = "$ref", default)]
    ref_type: Option<String>,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    items: Option<Box<ItemType>>,
    #[serde(rename = "enum", default)]
    enum_values: Vec<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
struct ItemType {
    #[serde(rename = "type", default)]
    type_kind: Option<String>,
    #[serde(rename = "$ref", default)]
    ref_type: Option<String>,
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '_' || c == '-' || c == '.' {
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            // Only insert underscore at transitions from lowercase to uppercase,
            // or when an uppercase sequence ends (e.g. "DOM" -> "dom", not "d_o_m")
            let prev_upper = chars[i - 1].is_uppercase();
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if !prev_upper || next_lower {
                result.push('_');
            }
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

/// Resolve a $ref type reference. Cross-domain refs like "Page.FrameId" become
/// `super::cdp_page::FrameId`. Same-domain refs are used directly.
fn resolve_ref(
    r: &str,
    current_domain: &str,
    domain_types: &std::collections::HashMap<String, HashSet<String>>,
) -> String {
    let parts: Vec<&str> = r.split('.').collect();
    if parts.len() == 2 {
        let ref_domain = parts[0];
        let ref_type = parts[1];
        if ref_domain == current_domain {
            to_pascal_case(ref_type)
        } else {
            // Check if this type actually exists in the referenced domain
            if domain_types
                .get(ref_domain)
                .is_some_and(|t| t.contains(ref_type))
            {
                format!(
                    "super::cdp_{}::{}",
                    to_snake_case(ref_domain),
                    to_pascal_case(ref_type)
                )
            } else {
                // Fall back to serde_json::Value for unknown cross-domain refs
                "serde_json::Value".to_string()
            }
        }
    } else {
        to_pascal_case(r)
    }
}

fn map_type_in_domain(
    prop: &Property,
    current_domain: &str,
    domain_types: &std::collections::HashMap<String, HashSet<String>>,
) -> String {
    if let Some(ref r) = prop.ref_type {
        let type_name = resolve_ref(r, current_domain, domain_types);
        if prop.optional {
            format!("Option<{type_name}>")
        } else {
            type_name
        }
    } else if let Some(ref t) = prop.type_kind {
        let base = match t.as_str() {
            "string" => "String".to_string(),
            "integer" => "i64".to_string(),
            "number" => "f64".to_string(),
            "boolean" => "bool".to_string(),
            "object" => "serde_json::Value".to_string(),
            "any" => "serde_json::Value".to_string(),
            "array" => {
                if let Some(ref items) = prop.items {
                    let inner = if let Some(ref r) = items.ref_type {
                        resolve_ref(r, current_domain, domain_types)
                    } else {
                        match items.type_kind.as_deref().unwrap_or("any") {
                            "string" => "String".to_string(),
                            "integer" => "i64".to_string(),
                            "number" => "f64".to_string(),
                            "boolean" => "bool".to_string(),
                            _ => "serde_json::Value".to_string(),
                        }
                    };
                    format!("Vec<{inner}>")
                } else {
                    "Vec<serde_json::Value>".to_string()
                }
            }
            _ => "serde_json::Value".to_string(),
        };
        if prop.optional {
            format!("Option<{base}>")
        } else {
            base
        }
    } else if prop.optional {
        "Option<serde_json::Value>".to_string()
    } else {
        "serde_json::Value".to_string()
    }
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "type"
            | "self"
            | "Self"
            | "super"
            | "move"
            | "ref"
            | "fn"
            | "mod"
            | "use"
            | "pub"
            | "let"
            | "mut"
            | "const"
            | "static"
            | "if"
            | "else"
            | "for"
            | "while"
            | "loop"
            | "match"
            | "return"
            | "break"
            | "continue"
            | "as"
            | "in"
            | "impl"
            | "trait"
            | "struct"
            | "enum"
            | "where"
            | "async"
            | "await"
            | "dyn"
            | "box"
            | "yield"
            | "override"
            | "crate"
            | "extern"
    )
}

fn generate_domain(
    domain: &Domain,
    domain_types: &std::collections::HashMap<String, HashSet<String>>,
    recursive_fields: &HashSet<(&str, &str, &str)>,
    output: &mut String,
) {
    let mod_name = to_snake_case(&domain.domain);
    output.push_str(&format!(
        "#[allow(dead_code, non_snake_case, non_camel_case_types, clippy::enum_variant_names)]\npub mod cdp_{mod_name} {{\n"
    ));
    output.push_str("    use super::*;\n\n");

    for type_def in &domain.types {
        if !type_def.enum_values.is_empty() {
            // Deduplicate enum variants (some CDP enums have duplicated PascalCase forms)
            let mut seen_variants = HashSet::new();
            output.push_str("    #[derive(Debug, Clone, Serialize, Deserialize)]\n");
            output.push_str(&format!("    pub enum {} {{\n", type_def.id));
            for val in &type_def.enum_values {
                let mut variant = to_pascal_case(val);
                if variant == "Self" {
                    variant = "SelfValue".to_string();
                }
                if variant.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    variant = format!("V{variant}");
                }
                if seen_variants.insert(variant.clone()) {
                    output.push_str(&format!(
                        "        #[serde(rename = \"{val}\")]\n        {variant},\n"
                    ));
                }
            }
            output.push_str("    }\n\n");
        } else if type_def.type_kind == "object" && !type_def.properties.is_empty() {
            output.push_str(
                "    #[derive(Debug, Clone, Serialize, Deserialize)]\n    #[serde(rename_all = \"camelCase\")]\n",
            );
            output.push_str(&format!("    pub struct {} {{\n", type_def.id));
            for prop in &type_def.properties {
                let field_name = to_snake_case(&prop.name);
                let field_name = if is_rust_keyword(&field_name) {
                    format!("r#{field_name}")
                } else {
                    field_name
                };
                let mut rust_type = map_type_in_domain(prop, &domain.domain, domain_types);

                // Wrap recursive fields in Box
                if recursive_fields.contains(&(
                    domain.domain.as_str(),
                    type_def.id.as_str(),
                    prop.name.as_str(),
                )) {
                    if rust_type.starts_with("Option<") {
                        let inner = &rust_type[7..rust_type.len() - 1];
                        rust_type = format!("Option<Box<{inner}>>");
                    } else {
                        rust_type = format!("Box<{rust_type}>");
                    }
                }

                if prop.optional {
                    output
                        .push_str("        #[serde(skip_serializing_if = \"Option::is_none\")]\n");
                }
                output.push_str(&format!("        pub {field_name}: {rust_type},\n"));
            }
            output.push_str("    }\n\n");
        } else if type_def.type_kind == "object" && type_def.properties.is_empty() {
            output.push_str(&format!(
                "    pub type {} = serde_json::Value;\n\n",
                type_def.id
            ));
        } else if type_def.type_kind == "array" {
            output.push_str(&format!(
                "    pub type {} = Vec<serde_json::Value>;\n\n",
                type_def.id
            ));
        } else if type_def.type_kind == "string" && type_def.enum_values.is_empty() {
            output.push_str(&format!("    pub type {} = String;\n\n", type_def.id));
        } else if type_def.type_kind == "integer" {
            output.push_str(&format!("    pub type {} = i64;\n\n", type_def.id));
        } else if type_def.type_kind == "number" {
            output.push_str(&format!("    pub type {} = f64;\n\n", type_def.id));
        }
    }

    for cmd in &domain.commands {
        let pascal_name = to_pascal_case(&cmd.name);

        if !cmd.parameters.is_empty() {
            output.push_str(
                "    #[derive(Debug, Clone, Serialize, Deserialize)]\n    #[serde(rename_all = \"camelCase\")]\n",
            );
            output.push_str(&format!("    pub struct {pascal_name}Params {{\n"));
            for param in &cmd.parameters {
                let field_name = to_snake_case(&param.name);
                let field_name = if is_rust_keyword(&field_name) {
                    format!("r#{field_name}")
                } else {
                    field_name
                };
                let rust_type = map_type_in_domain(param, &domain.domain, domain_types);
                if param.optional {
                    output
                        .push_str("        #[serde(skip_serializing_if = \"Option::is_none\")]\n");
                }
                output.push_str(&format!("        pub {field_name}: {rust_type},\n"));
            }
            output.push_str("    }\n\n");
        }

        if !cmd.returns.is_empty() {
            output.push_str(
                "    #[derive(Debug, Clone, Serialize, Deserialize)]\n    #[serde(rename_all = \"camelCase\")]\n",
            );
            output.push_str(&format!("    pub struct {pascal_name}Result {{\n"));
            for ret in &cmd.returns {
                let field_name = to_snake_case(&ret.name);
                let field_name = if is_rust_keyword(&field_name) {
                    format!("r#{field_name}")
                } else {
                    field_name
                };
                let rust_type = map_type_in_domain(ret, &domain.domain, domain_types);
                if ret.optional {
                    output
                        .push_str("        #[serde(skip_serializing_if = \"Option::is_none\")]\n");
                }
                output.push_str(&format!("        pub {field_name}: {rust_type},\n"));
            }
            output.push_str("    }\n\n");
        }
    }

    for event in &domain.events {
        if !event.parameters.is_empty() {
            let pascal_name = to_pascal_case(&event.name);
            output.push_str(
                "    #[derive(Debug, Clone, Serialize, Deserialize)]\n    #[serde(rename_all = \"camelCase\")]\n",
            );
            output.push_str(&format!("    pub struct {pascal_name}Event {{\n"));
            for param in &event.parameters {
                let field_name = to_snake_case(&param.name);
                let field_name = if is_rust_keyword(&field_name) {
                    format!("r#{field_name}")
                } else {
                    field_name
                };
                let rust_type = map_type_in_domain(param, &domain.domain, domain_types);
                if param.optional {
                    output
                        .push_str("        #[serde(skip_serializing_if = \"Option::is_none\")]\n");
                }
                output.push_str(&format!("        pub {field_name}: {rust_type},\n"));
            }
            output.push_str("    }\n\n");
        }
    }

    output.push_str("}\n\n");
}

/// Embed short git SHA / UTC timestamp as `cargo:rustc-env` keys.
///
/// **Native-only** (rules_rust crates nativas): never shells out to `git` or `date`.
/// SHA is resolved by reading `.git/HEAD` (+ loose ref or `packed-refs`). Missing
/// `.git` is non-fatal (`unknown`). Rebuild when HEAD / refs change.
///
/// A `dirty` flag is intentionally NOT emitted, and `SOURCE_HASH` replaces it.
/// Detecting an uncommitted worktree without the git CLI needs a full index
/// walk or a heavy `gix` build-dep, and every cheap heuristic (comparing source
/// mtimes against `.git/index`) can prove DIRTY but never prove CLEAN — a
/// `dirty: false` derived from one would assert more than it knows. See
/// [`emit_source_hash`] for the field that carries the same intent honestly.
fn emit_git_build_meta() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/packed-refs");
    // Reproducible builds when the environment provides an epoch (OS concern).
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let sha = read_git_head_sha().unwrap_or_else(|| "unknown".into());
    let ts = utc_timestamp_now();

    println!("cargo:rustc-env=GIT_SHA={sha}");
    println!("cargo:rustc-env=BUILD_TIMESTAMP={ts}");
}

/// Embed `SOURCE_HASH`: a content fingerprint of everything that shapes the
/// binary, so `version --json` identifies the build even off a commit.
///
/// # Why this exists
///
/// `git_sha` names the last commit, not the code that was compiled. Building
/// from a modified worktree yields a binary whose `git_sha` points at source it
/// does not contain, and an agent that checks out that SHA to reproduce the run
/// gets different code with no signal that anything diverged.
///
/// `SOURCE_HASH` is derived from the bytes actually compiled, so it is exactly
/// what it claims: equal hashes mean equal inputs, different hashes mean the
/// tree moved. Reproducing is `checkout <git_sha>`, rebuild, compare the field.
///
/// # Determinism
///
/// Inputs are sorted by their repo-relative path with `/` separators, and CRLF
/// is folded to LF before hashing, so a checkout on Windows and one on Linux
/// agree. Backup artefacts (`*.bak.*`) are excluded: they are never compiled,
/// and letting them in would make the hash depend on editor noise.
///
/// # Algorithm
///
/// FNV-1a over 128 bits, folded to 64 for a readable field. This is an identity
/// token, not a security boundary — nothing here defends against a chosen
/// collision, and nothing needs to.
fn emit_source_hash() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=build.rs");

    let mut inputs: Vec<String> = Vec::new();
    collect_source_files(Path::new("src"), Path::new("src"), &mut inputs);
    for extra in ["Cargo.toml", "Cargo.lock", "build.rs"] {
        if Path::new(extra).is_file() {
            inputs.push(extra.to_string());
        }
    }
    inputs.sort_unstable();

    // FNV-1a 128-bit offset basis / prime.
    let mut hash: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

    let mix = |bytes: &[u8], hash: &mut u128| {
        for &b in bytes {
            *hash ^= u128::from(b);
            *hash = hash.wrapping_mul(PRIME);
        }
    };

    for rel in &inputs {
        // Path first: renaming a file must move the hash even if bytes are equal.
        mix(rel.as_bytes(), &mut hash);
        mix(b"\0", &mut hash);
        match fs::read(rel) {
            Ok(bytes) => {
                let normalized: Vec<u8> = bytes.into_iter().filter(|&b| b != b'\r').collect();
                mix(&normalized, &mut hash);
            }
            // An unreadable input still perturbs the hash; silently skipping it
            // would let two different trees claim the same fingerprint.
            Err(_) => mix(b"<unreadable>", &mut hash),
        }
        mix(b"\0", &mut hash);
    }

    let folded = ((hash >> 64) as u64) ^ (hash as u64);
    println!("cargo:rustc-env=SOURCE_HASH={folded:016x}");
}

/// Push every compiled input under `dir` as a `/`-separated path relative to
/// `root`, depth-first. Order is irrelevant: the caller sorts.
fn collect_source_files(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Backups and editor noise are not build inputs.
        if name.starts_with('.') || name.contains(".bak.") {
            continue;
        }
        if path.is_dir() {
            collect_source_files(&path, root, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push(format!("src/{rel}"));
        }
    }
}

/// Short (12-hex) SHA from `.git` filesystem — no `Command::new("git")`.
fn read_git_head_sha() -> Option<String> {
    let head = fs::read_to_string(".git/HEAD").ok()?;
    let head = head.trim();
    if let Some(refname) = head.strip_prefix("ref: ") {
        let refname = refname.trim();
        let loose = Path::new(".git").join(refname);
        if let Ok(sha) = fs::read_to_string(&loose) {
            return short_sha(sha.trim());
        }
        // Fall back to packed-refs (common after `git gc` / shallow clones).
        if let Ok(packed) = fs::read_to_string(".git/packed-refs") {
            for line in packed.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('^') {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let Some(sha) = parts.next() else {
                    continue;
                };
                let Some(name) = parts.next() else {
                    continue;
                };
                if name == refname {
                    return short_sha(sha);
                }
            }
        }
        None
    } else {
        // Detached HEAD: raw SHA in HEAD.
        short_sha(head)
    }
}

fn short_sha(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() < 7 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = s.len().min(12);
    Some(s[..n].to_string())
}

/// UTC `YYYY-MM-DDTHH:MM:SSZ` without the `date` CLI (rules: chrono/`std::time`).
///
/// Prefers `SOURCE_DATE_EPOCH` (seconds) for reproducible builds; otherwise
/// wall-clock via [`SystemTime`].
fn utc_timestamp_now() -> String {
    let secs = env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs())
        })
        .unwrap_or(0);
    format_unix_secs_utc(secs)
}

/// Civil date from Unix epoch seconds (Howard Hinnant algorithm) — pure `std`.
fn format_unix_secs_utc(mut secs: u64) -> String {
    const SECS_PER_DAY: u64 = 86_400;
    let days = secs / SECS_PER_DAY;
    secs %= SECS_PER_DAY;
    let hour = secs / 3_600;
    secs %= 3_600;
    let min = secs / 60;
    let sec = secs % 60;

    // days since Unix epoch → proleptic Gregorian (civil_from_days).
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}
