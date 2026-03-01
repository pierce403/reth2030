use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use syn::{Item, UseTree, Visibility};

const TODO_ACCEPTANCE_LINE: &str = "- [x] Public APIs are documented at crate-level.";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed reading {}: {err}", path.display()))
}

fn crate_level_doc_lines(source: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut started = false;

    for line in source.lines() {
        let trimmed_start = line.trim_start();
        if let Some(doc) = trimmed_start.strip_prefix("//!") {
            lines.push(doc.trim_start());
            started = true;
            continue;
        }

        if !started && line.trim().is_empty() {
            continue;
        }

        if started && line.trim().is_empty() {
            lines.push("");
            continue;
        }

        break;
    }

    lines
}

fn public_api_section_lines<'a>(doc_lines: &'a [&'a str]) -> Option<Vec<&'a str>> {
    let mut in_public_api = false;
    let mut lines = Vec::new();

    for line in doc_lines {
        let trimmed = line.trim();
        if trimmed == "## Public API" {
            in_public_api = true;
            continue;
        }

        if in_public_api {
            if trimmed.starts_with("## ") {
                break;
            }
            lines.push(trimmed);
        }
    }

    in_public_api.then_some(lines)
}

fn parse_documented_symbols(section_lines: &[&str]) -> BTreeSet<String> {
    let mut symbols = BTreeSet::new();

    for line in section_lines {
        let trimmed = line.trim();
        if !trimmed.starts_with("- `") {
            continue;
        }

        let rest = &trimmed[3..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let symbol = &rest[..end];
        if symbol.is_empty() {
            continue;
        }

        symbols.insert(symbol.to_owned());
    }

    symbols
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn collect_public_use_tree_symbols(tree: &UseTree, symbols: &mut BTreeSet<String>) {
    match tree {
        UseTree::Name(name) => {
            symbols.insert(name.ident.to_string());
        }
        UseTree::Rename(rename) => {
            symbols.insert(rename.rename.to_string());
        }
        UseTree::Path(path) => collect_public_use_tree_symbols(&path.tree, symbols),
        UseTree::Group(group) => {
            for tree in &group.items {
                collect_public_use_tree_symbols(tree, symbols);
            }
        }
        UseTree::Glob(_) => {
            panic!(
                "public glob re-exports are not supported by this contract; list explicit symbols"
            )
        }
    }
}

fn collect_public_item_symbols(item: Item, symbols: &mut BTreeSet<String>) {
    match item {
        Item::Const(item_const) if is_public(&item_const.vis) => {
            symbols.insert(item_const.ident.to_string());
        }
        Item::Enum(item_enum) if is_public(&item_enum.vis) => {
            symbols.insert(item_enum.ident.to_string());
        }
        Item::ExternCrate(item_extern_crate) if is_public(&item_extern_crate.vis) => {
            let symbol = item_extern_crate
                .rename
                .as_ref()
                .map(|(_, ident)| ident.to_string())
                .unwrap_or_else(|| item_extern_crate.ident.to_string());
            symbols.insert(symbol);
        }
        Item::Fn(item_fn) if is_public(&item_fn.vis) => {
            symbols.insert(item_fn.sig.ident.to_string());
        }
        Item::Mod(item_mod) if is_public(&item_mod.vis) => {
            symbols.insert(item_mod.ident.to_string());
        }
        Item::Static(item_static) if is_public(&item_static.vis) => {
            symbols.insert(item_static.ident.to_string());
        }
        Item::Struct(item_struct) if is_public(&item_struct.vis) => {
            symbols.insert(item_struct.ident.to_string());
        }
        Item::Trait(item_trait) if is_public(&item_trait.vis) => {
            symbols.insert(item_trait.ident.to_string());
        }
        Item::TraitAlias(item_trait_alias) if is_public(&item_trait_alias.vis) => {
            symbols.insert(item_trait_alias.ident.to_string());
        }
        Item::Type(item_type) if is_public(&item_type.vis) => {
            symbols.insert(item_type.ident.to_string());
        }
        Item::Union(item_union) if is_public(&item_union.vis) => {
            symbols.insert(item_union.ident.to_string());
        }
        Item::Use(item_use) if is_public(&item_use.vis) => {
            collect_public_use_tree_symbols(&item_use.tree, symbols);
        }
        _ => {}
    }
}

fn parse_public_symbols_from_source(source: &str) -> BTreeSet<String> {
    let syntax = syn::parse_file(source).expect("crate source must parse as Rust");
    let mut symbols = BTreeSet::new();
    for item in syntax.items {
        collect_public_item_symbols(item, &mut symbols);
    }
    symbols
}

fn assert_crate_public_api_docs(relative_path: &str) {
    let source = read_repo_file(relative_path);
    let doc_lines = crate_level_doc_lines(&source);
    assert!(
        !doc_lines.is_empty(),
        "{relative_path} must contain crate-level inner-doc comments"
    );

    let section = public_api_section_lines(&doc_lines)
        .unwrap_or_else(|| panic!("{relative_path} must contain a `## Public API` section"));
    let documented_symbols = parse_documented_symbols(&section);

    let exported_symbols = parse_public_symbols_from_source(&source);

    assert_eq!(
        documented_symbols, exported_symbols,
        "{relative_path} `## Public API` symbols must match expected public API surface"
    );
}

#[test]
fn todo_marks_public_api_docs_acceptance_criterion_complete() {
    let todo = read_repo_file("TODO.md");
    assert!(
        todo.lines()
            .any(|line| line.trim() == TODO_ACCEPTANCE_LINE),
        "TODO.md must keep the public API docs acceptance criterion checked: {TODO_ACCEPTANCE_LINE}"
    );
}

#[test]
fn crate_level_doc_lines_capture_only_leading_inner_docs() {
    let source = r#"
//! Top-level docs.
//!
//! ## Public API
//! - `Foo`: documented.

pub struct Foo;

//! Not part of crate docs.
"#;

    let lines = crate_level_doc_lines(source);
    assert_eq!(
        lines,
        vec![
            "Top-level docs.",
            "",
            "## Public API",
            "- `Foo`: documented.",
            ""
        ]
    );
}

#[test]
fn public_api_section_lines_stops_at_next_h2_heading() {
    let docs = [
        "Intro",
        "## Public API",
        "- `Foo`: documented.",
        "",
        "## Notes",
        "- `Bar`: not in public-api section",
    ];

    let section = public_api_section_lines(&docs).expect("public API section should exist");
    assert_eq!(section, vec!["- `Foo`: documented.", ""]);
}

#[test]
fn parse_documented_symbols_ignores_non_bullets_and_malformed_entries() {
    let section = [
        "- `Foo`: documented",
        "- `Bar`",
        "- no backticks",
        "- ``: empty symbol",
        "text",
    ];

    let symbols = parse_documented_symbols(&section);
    assert_eq!(
        symbols,
        BTreeSet::from(["Bar".to_owned(), "Foo".to_owned()])
    );
}

#[test]
fn parse_public_symbols_from_source_collects_public_root_items_and_reexports() {
    let source = r#"
pub use crate::alpha::{Foo, bar::Baz as Qux, nested::Zed};
pub use crate::solo::Thing;
pub struct Api;
pub enum Kind {
    A,
}
pub type Count = u64;
pub trait Runner {}
pub const LIMIT: u64 = 10;
pub static ENABLED: bool = true;
pub fn run() {}
"#;

    let symbols = parse_public_symbols_from_source(source);
    assert_eq!(
        symbols,
        BTreeSet::from([
            "Api".to_owned(),
            "Count".to_owned(),
            "ENABLED".to_owned(),
            "Foo".to_owned(),
            "Kind".to_owned(),
            "LIMIT".to_owned(),
            "Qux".to_owned(),
            "Runner".to_owned(),
            "Thing".to_owned(),
            "Zed".to_owned(),
            "run".to_owned(),
        ])
    );
}

#[test]
fn parse_public_symbols_from_source_ignores_non_public_and_nested_items() {
    let source = r#"
pub struct Api;

impl Api {
    pub fn method(&self) {}
}

pub(crate) fn internal() {}
fn private() {}

mod nested {
    pub struct NestedApi;
}
"#;

    let symbols = parse_public_symbols_from_source(source);
    assert_eq!(symbols, BTreeSet::from(["Api".to_owned()]));
}

#[test]
#[should_panic(expected = "public glob re-exports are not supported by this contract")]
fn parse_public_symbols_from_source_rejects_public_glob_reexports() {
    let _ = parse_public_symbols_from_source("pub use crate::foo::*;");
}

#[test]
fn reth2030_core_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs("crates/reth2030-core/src/lib.rs");
}

#[test]
fn reth2030_types_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs("crates/reth2030-types/src/lib.rs");
}

#[test]
fn reth2030_net_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs("crates/reth2030-net/src/lib.rs");
}

#[test]
fn reth2030_rpc_public_api_is_documented_at_crate_level() {
    assert_crate_public_api_docs("crates/reth2030-rpc/src/lib.rs");
}
