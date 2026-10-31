//! The lossless KDL engine against generated configs: comments in and
//! between nodes, blank lines, tabs and CRLF, raw and escaped strings,
//! `/-` nodes, blocks on one line and on many.
//!
//! The generator keeps what it meant beside the text it wrote, so each
//! check compares the parser and the splice engine with a second
//! derivation: the tree read from the text against the tree written, and
//! an edited text against the same tree written out again with the new
//! value. What differs is the route (rendering a tree, against parsing and
//! splicing), not the author: one reading of KDL wrote both sides.
//!
//! Swept: niri's one-node-per-line shape and `{ a; b; }` blocks, nesting
//! three deep, arguments, properties, a bare word last on a node. Left
//! out: multi-line strings and block comments across lines, which niri
//! configs do not use, and escapes beyond `\"`, `\\`, `\n` and `\t`.

use std::collections::{BTreeMap, HashMap, HashSet};

use niri_center::kdl::{Node as Parsed, PathSeg, ValueKind, parse_document};
use proptest::prelude::*;

/// Node names: some of niri's, a keybind, and words that start like a
/// raw string or a keyword.
const NODE_NAMES: &[&str] = &[
    "output",
    "layout",
    "window-rule",
    "match",
    "scale",
    "Mod+Shift+T",
    "r",
    "off",
    "_x",
];
const PROP_NAMES: &[&str] = &["x", "y", "app-id", "repeat", "r"];
const BARE_WORDS: &[&str] = &["off", "true", "null", "eDP-1", "r", "on-demand"];
/// What comes between a node's name and its values, and between values.
const GAPS: &[&str] = &[" ", "  ", "\t", " /* c */ "];

/// A value as written in the file, and what the parser should read there.
#[derive(Debug, Clone)]
struct Written {
    text: String,
    read: ValueKind,
}

#[derive(Debug, Clone)]
enum Entry {
    Arg(Written),
    Prop(&'static str, Written),
}

#[derive(Debug, Clone)]
struct Node {
    slashdash: Option<&'static str>,
    name: &'static str,
    entries: Vec<(&'static str, Entry)>,
    /// A bare word as the last argument: only on a node without a block,
    /// where a word before `{` would name a new node.
    last_bare: Option<(&'static str, Written)>,
    before_block: &'static str,
    block: Option<Block>,
    /// Space or a comment after the node on its line; not written inside a
    /// one-line block, where a line comment would swallow the `}`.
    tail: String,
}

#[derive(Debug, Clone)]
struct Block {
    one_line: bool,
    /// On one line, whether the last node ends with `;` or meets the `}`.
    last_semicolon: bool,
    children: Vec<Item>,
}

#[derive(Debug, Clone)]
enum Item {
    Node(Node),
    Comment(String),
    Blank(&'static str),
}

#[derive(Debug, Clone)]
struct Generated {
    items: Vec<Item>,
    indent_unit: &'static str,
    line_break: &'static str,
    final_line_break: bool,
}

fn number() -> impl Strategy<Value = Written> {
    "[-+]?[0-9]{1,4}(\\.[0-9]{1,3})?([eE][-+]?[0-9]{1,2})?".prop_map(|text| Written {
        read: ValueKind::Number(text.clone()),
        text,
    })
}

/// A quoted string, escapes as the KDL spec reads them.
fn quoted() -> impl Strategy<Value = Written> {
    let piece = prop_oneof![
        "[a-z0-9 {};=/*#é—]".prop_map(|text| (text.clone(), text)),
        Just(("\\\"".to_string(), "\"".to_string())),
        Just(("\\\\".to_string(), "\\".to_string())),
        Just(("\\n".to_string(), "\n".to_string())),
        Just(("\\t".to_string(), "\t".to_string())),
    ];
    prop::collection::vec(piece, 0..8).prop_map(|pieces| {
        let (written, read): (String, String) = pieces.into_iter().unzip();
        Written {
            text: format!("\"{written}\""),
            read: ValueKind::Quoted(read),
        }
    })
}

/// A raw string: no escapes, and a `"` inside once it has a `#`.
fn raw() -> impl Strategy<Value = Written> {
    (0..3usize)
        .prop_flat_map(|hashes| {
            let content = if hashes == 0 {
                "[a-z0-9 {};=/*\\\\é]{0,8}"
            } else {
                "[a-z0-9 {};=/*\\\\é\"]{0,8}"
            };
            (Just(hashes), content)
        })
        .prop_map(|(hashes, content)| {
            let fence = "#".repeat(hashes);
            Written {
                text: format!("r{fence}\"{content}\"{fence}"),
                read: ValueKind::Quoted(content),
            }
        })
}

fn bare() -> impl Strategy<Value = Written> {
    prop::sample::select(BARE_WORDS).prop_map(|word| Written {
        text: word.to_string(),
        read: ValueKind::Bare(word.to_string()),
    })
}

/// A value that may stand anywhere among a node's arguments.
fn argument() -> impl Strategy<Value = Written> {
    prop_oneof![number(), quoted(), raw()]
}

fn entry() -> impl Strategy<Value = Entry> {
    prop_oneof![
        argument().prop_map(Entry::Arg),
        (
            prop::sample::select(PROP_NAMES),
            prop_oneof![number(), quoted(), raw(), bare()]
        )
            .prop_map(|(name, value)| Entry::Prop(name, value)),
    ]
}

fn tail() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("  ".to_string()),
        " //[ a-z{};\"*/]{0,10}",
        " /\\*[ a-z{};\"]{0,6}\\*/",
    ]
}

fn comment() -> impl Strategy<Value = Item> {
    prop_oneof!["//[ a-z{};\"*/]{0,12}", "/\\*[ a-z{};\"]{0,12}\\*/"].prop_map(Item::Comment)
}

fn blank() -> impl Strategy<Value = Item> {
    prop::sample::select(&["", "  ", "\t"][..]).prop_map(Item::Blank)
}

fn node(block: BoxedStrategy<Option<Block>>) -> impl Strategy<Value = Item> {
    (
        prop::option::of(prop::sample::select(&["/-", "/- "][..])),
        prop::sample::select(NODE_NAMES),
        prop::collection::vec((prop::sample::select(GAPS), entry()), 0..4),
        prop::option::of((prop::sample::select(GAPS), bare())),
        prop::sample::select(&["", " "][..]),
        block,
        tail(),
    )
        .prop_map(
            |(slashdash, name, mut entries, last_bare, before_block, block, tail)| {
                // a property is edited where it is first written: keep each name once
                let mut seen_props = HashSet::new();
                entries.retain(|(_, entry)| match entry {
                    Entry::Arg(_) => true,
                    Entry::Prop(prop_name, _) => seen_props.insert(*prop_name),
                });
                let last_bare = if block.is_some() { None } else { last_bare };
                Item::Node(Node {
                    slashdash,
                    name,
                    entries,
                    last_bare,
                    before_block,
                    block,
                    tail,
                })
            },
        )
}

fn generated() -> impl Strategy<Value = Generated> {
    let leaf = prop_oneof![
        3 => node(Just(None).boxed()),
        1 => comment(),
        1 => blank(),
    ];
    let item = leaf.prop_recursive(3, 32, 5, |inner| {
        let block = (
            any::<bool>(),
            any::<bool>(),
            prop::collection::vec(inner, 0..5),
        )
            .prop_map(|(one_line, last_semicolon, children)| {
                Some(Block {
                    one_line,
                    last_semicolon,
                    children,
                })
            })
            .boxed();
        prop_oneof![
            2 => node(block),
            1 => node(Just(None).boxed()),
            1 => comment(),
            1 => blank(),
        ]
    });
    (
        prop::collection::vec(item, 0..8),
        prop::sample::select(&["    ", "\t", "  "][..]),
        prop::sample::select(&["\n", "\r\n"][..]),
        any::<bool>(),
    )
        .prop_map(
            |(items, indent_unit, line_break, final_line_break)| Generated {
                items,
                indent_unit,
                line_break,
                final_line_break,
            },
        )
}

/// The nodes among `items`, without the comments and blank lines.
fn nodes(items: &[Item]) -> impl Iterator<Item = &Node> {
    items.iter().filter_map(|item| match item {
        Item::Node(node) => Some(node),
        Item::Comment(_) | Item::Blank(_) => None,
    })
}

fn render(config: &Generated) -> String {
    let mut out = String::new();
    render_lines(&config.items, "", config, &mut out);
    if !config.final_line_break
        && let Some(kept) = out.strip_suffix(config.line_break)
    {
        out.truncate(kept.len());
    }
    out
}

fn render_lines(items: &[Item], indent: &str, config: &Generated, out: &mut String) {
    for item in items {
        match item {
            Item::Blank(space) => out.push_str(space),
            Item::Comment(text) => {
                out.push_str(indent);
                out.push_str(text);
            }
            Item::Node(node) => {
                out.push_str(indent);
                render_head(node, out);
                if let Some(block) = &node.block {
                    out.push_str(node.before_block);
                    if block.one_line {
                        render_one_line_block(block, out);
                    } else {
                        out.push('{');
                        out.push_str(config.line_break);
                        let inner = format!("{indent}{}", config.indent_unit);
                        render_lines(&block.children, &inner, config, out);
                        out.push_str(indent);
                        out.push('}');
                    }
                }
                out.push_str(&node.tail);
            }
        }
        out.push_str(config.line_break);
    }
}

/// `{ a; b 1 { c; }; }` or `{ a; b off }`: the nodes only, between `;`.
fn render_one_line_block(block: &Block, out: &mut String) {
    out.push('{');
    let nodes: Vec<&Node> = nodes(&block.children).collect();
    for (position, node) in nodes.iter().enumerate() {
        out.push(' ');
        render_head(node, out);
        if let Some(inner) = &node.block {
            out.push_str(node.before_block);
            render_one_line_block(inner, out);
        }
        if block.last_semicolon || position + 1 < nodes.len() {
            out.push(';');
        }
    }
    out.push_str(" }");
}

fn render_head(node: &Node, out: &mut String) {
    if let Some(marker) = node.slashdash {
        out.push_str(marker);
    }
    out.push_str(node.name);
    for (gap, entry) in &node.entries {
        out.push_str(gap);
        match entry {
            Entry::Arg(value) => out.push_str(&value.text),
            Entry::Prop(name, value) => {
                out.push_str(name);
                out.push('=');
                out.push_str(&value.text);
            }
        }
    }
    if let Some((gap, value)) = &node.last_bare {
        out.push_str(gap);
        out.push_str(&value.text);
    }
}

/// A node as read, without where it was written.
#[derive(Debug, PartialEq)]
struct Shape {
    name: String,
    disabled: bool,
    args: Vec<ValueKind>,
    props: Vec<(String, ValueKind)>,
    children: Option<Vec<Self>>,
}

fn meant(items: &[Item]) -> Vec<Shape> {
    nodes(items)
        .map(|node| {
            let mut args = Vec::new();
            let mut props = Vec::new();
            for (_, entry) in &node.entries {
                match entry {
                    Entry::Arg(value) => args.push(value.read.clone()),
                    Entry::Prop(name, value) => props.push((name.to_string(), value.read.clone())),
                }
            }
            args.extend(node.last_bare.iter().map(|(_, value)| value.read.clone()));
            Shape {
                name: node.name.to_string(),
                disabled: node.slashdash.is_some(),
                args,
                props,
                children: node.block.as_ref().map(|block| meant(&block.children)),
            }
        })
        .collect()
}

fn read(nodes: &[Parsed]) -> Vec<Shape> {
    nodes
        .iter()
        .map(|node| Shape {
            name: node.name.clone(),
            disabled: node.disabled,
            args: node.args.iter().map(|value| value.kind.clone()).collect(),
            props: node
                .props
                .iter()
                .map(|prop| (prop.name.clone(), prop.value.kind.clone()))
                .collect(),
            children: node.block.map(|_| read(&node.children)),
        })
        .collect()
}

/// Where an edit lands: a node's argument by position, or its property.
#[derive(Debug, Clone)]
enum Target {
    Arg(usize),
    Prop(&'static str),
}

/// A value in the generated tree, with the path and target that edit it.
struct Site<'tree> {
    path: Vec<PathSeg>,
    target: Target,
    value: &'tree mut Written,
}

/// Every value among `items`, in document order.
fn value_sites<'tree>(
    items: &'tree mut [Item],
    path: &mut Vec<PathSeg>,
    sites: &mut Vec<Site<'tree>>,
) {
    let mut same_named_before: HashMap<&'static str, usize> = HashMap::new();
    for item in items {
        let Item::Node(node) = item else { continue };
        let count = same_named_before.entry(node.name).or_default();
        path.push(PathSeg::Name(node.name.to_string()));
        path.push(PathSeg::Index(*count));
        *count += 1;
        let mut arg_index = 0;
        for (_, entry) in &mut node.entries {
            let (target, value) = match entry {
                Entry::Arg(value) => {
                    arg_index += 1;
                    (Target::Arg(arg_index - 1), value)
                }
                Entry::Prop(name, value) => (Target::Prop(name), value),
            };
            sites.push(Site {
                path: path.clone(),
                target,
                value,
            });
        }
        if let Some((_, value)) = &mut node.last_bare {
            sites.push(Site {
                path: path.clone(),
                target: Target::Arg(arg_index),
                value,
            });
        }
        if let Some(block) = &mut node.block {
            value_sites(&mut block.children, path, sites);
        }
        path.truncate(path.len() - 2);
    }
}

proptest! {
    #[test]
    fn an_unedited_config_comes_back_byte_for_byte(config in generated()) {
        let text = render(&config);
        let doc = parse_document(&text).unwrap();
        prop_assert_eq!(doc.to_string(), text.as_str());
        prop_assert_eq!(doc.text(), text.as_str());
    }

    #[test]
    fn the_tree_read_is_the_tree_written(config in generated()) {
        let text = render(&config);
        let doc = parse_document(&text).unwrap();
        prop_assert_eq!(read(&doc.roots), meant(&config.items));
    }

    /// Edits change their values and no other byte: the edited text is the
    /// tree written again with the new values, and reads as that tree.
    #[test]
    fn edits_change_their_values_and_nothing_else(
        config in generated(),
        picks in prop::collection::vec((any::<prop::sample::Index>(), argument()), 1..4),
    ) {
        let text = render(&config);
        let mut doc = parse_document(&text).unwrap();
        let mut expected = config;
        let mut sites = Vec::new();
        value_sites(&mut expected.items, &mut Vec::new(), &mut sites);
        prop_assume!(!sites.is_empty());
        // each site edited once, with the first replacement picked for it
        let mut chosen = BTreeMap::new();
        for (pick, replacement) in picks {
            chosen.entry(pick.index(sites.len())).or_insert(replacement);
        }
        for (index, replacement) in chosen {
            let site = &mut sites[index];
            match site.target {
                Target::Arg(arg) => doc.edit_arg(&site.path, arg, &replacement.text),
                Target::Prop(name) => doc.edit_prop(&site.path, name, &replacement.text),
            }
            .unwrap();
            *site.value = replacement;
        }
        let edited = doc.to_string();
        prop_assert_eq!(&edited, &render(&expected));
        let reread = parse_document(&edited).unwrap();
        prop_assert_eq!(read(&reread.roots), meant(&expected.items));
    }
}
