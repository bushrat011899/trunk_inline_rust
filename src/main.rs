use std::{
    cell::RefCell, fs::{remove_dir_all, DirBuilder, File, OpenOptions}, io::Write, ops::Deref, path::Path
};

use cargo_toml::{Manifest, Package};
use html5ever::{tendril::TendrilSink, tree_builder::TreeBuilderOpts, Attribute, LocalName, Namespace, ParseOpts, QualName};
use markup5ever_rcdom::{Node, NodeData, RcDom, SerializableHandle};

fn main() {
    let _args = std::env::args();

    let env = std::env::vars();

    let mut staging = None;
    let mut html_file = None;

    for (name, value) in env {
        match name.as_str() {
            "TRUNK_STAGING_DIR" => staging = Some(value),
            "TRUNK_HTML_FILE" => html_file = Some(value),
            _ => {}
        }
    }

    let staging = staging.unwrap();
    let html_file = html_file.unwrap();
    
    let staging_path = Path::new(&staging);
    let html_file_path = Path::new(&html_file);
    let staging_html_file_path = staging_path.join(html_file_path.file_name().unwrap());

    let mut staging_html_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&staging_html_file_path)
        .unwrap();

    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let dom = html5ever::parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut staging_html_file)
        .unwrap();

    let mut id = 0;

    let _ = walk(dom.document.as_ref(), |node| {
        let mut new_nodes = vec![];

        for (index, node) in node.children.borrow_mut().iter_mut().enumerate() {
            let path = {
                let NodeData::Element {
                    ref name,
                    ref attrs,
                    ..
                } = node.data
                else {
                    continue;
                };
        
                if name.prefix.is_some() {
                    continue;
                }
        
                if &name.local != "script" {
                    continue;
                }
        
                let Some(_) = attrs.borrow().iter().find(|attr| {
                    attr.name.prefix.is_none() && &attr.name.local == "type" && &*attr.value == "rust"
                }) else {
                    continue;
                };
        
                let children = node.children.borrow_mut();
        
                let Some(text_node) = children.get(0) else {
                    continue;
                };
        
                let NodeData::Text { ref contents } = text_node.data else {
                    continue;
                };
        
                let content = contents.borrow();
        
                let Some((_, rest)) = content.split_once("//! ```cargo") else {
                    continue;
                };
        
                let Some((toml, body)) = rest.split_once("//! ```") else {
                    continue;
                };
        
                let toml = toml
                    .lines()
                    .flat_map(|line| line.trim().strip_prefix("//!"))
                    .map(|line| line.trim())
                    .map(|line| format!("{line}\n"))
                    .collect::<String>();
        
                let Ok(mut toml) = Manifest::from_str(&toml) else {
                    continue;
                };
        
                let mut lib = toml.lib.unwrap_or_default();
        
                if !lib.crate_type.contains(&"cdylib".to_string()) {
                    lib.crate_type.push("cdylib".to_string());
                }
        
                toml.lib = Some(lib);
        
                let package = toml.package.unwrap_or(Package::new("generated", "0.1.0"));
        
                toml.package = Some(package);
        
                let toml = toml::to_string_pretty(&toml).unwrap();
        
                let path = staging_path.join(format!("temp_{}", id));
        
                let path_src = path.join(format!("src"));

                let path_cargo = path.join(format!("Cargo.toml"));

                let path_lib = path_src.join(format!("lib.rs"));
        
                DirBuilder::new().recursive(true).create(&path_src).unwrap();
        
                let mut file = File::create(path_cargo.to_str().unwrap()).unwrap();
                file.write_all(toml.as_bytes()).unwrap();
        
                let mut file = File::create(path_lib.to_str().unwrap()).unwrap();
                file.write_all(body.as_bytes()).unwrap();

                path
            };

            let new_node = Node::new(NodeData::Element {
                name: QualName::new(None, Namespace::from("http://www.w3.org/1999/xhtml"), LocalName::from("script")),
                attrs: RefCell::new(vec![
                    Attribute {
                        name: QualName::new(None, Namespace::from(""), LocalName::from("type")),
                        value: "module".into()
                    },
                ]),
                template_contents: Default::default(),
                mathml_annotation_xml_integration_point: Default::default(),
            });

            let hoist = format!("\nimport init, * as pub from './rust_modules/{id}/index.js';\nfor (const key of Object.keys(pub)) \n{{\nif (key == 'default') {{\ncontinue;\n}}\nwindow[key] = pub[key];\n}}\nawait init();\n");

            new_node.children.borrow_mut().push(Node::new(NodeData::Text {
                contents: RefCell::new(hoist.into())
            }));

            new_nodes.push((index, new_node));

            let _ = std::process::Command::new("wasm-pack")
                .args([
                    "build",
                    "--no-typescript",
                    "--out-dir",
                    staging_path.join("rust_modules").join(format!("{}", id)).to_str().unwrap(),
                    "--out-name",
                    "index",
                    "--target",
                    "web",
                    path.to_str().unwrap(),
                ])
                .output()
                .expect("failed to execute process");

            remove_dir_all(path).unwrap();
        
            id += 1;
        }

        for (index, new_node) in new_nodes {
            node.children.borrow_mut()[index] = new_node;
        }
    });

    let _ = staging_html_file;

    let mut staging_html_file = File::create(&staging_html_file_path).unwrap();

    // The validator.nu HTML2HTML always prints a doctype at the very beginning.
    staging_html_file
        .write_all(b"<!DOCTYPE html>\n")
        .expect("writing DOCTYPE failed");
    let document: SerializableHandle = dom.document.clone().into();
    html5ever::serialize(&mut staging_html_file, &document, Default::default()).expect("serialization failed");
}

fn walk<F: FnMut(&Node)>(node: impl Deref<Target = Node>, mut action: F) -> F {
    for node in node.children.borrow().iter() {
        action(node);
        action = walk(node.as_ref(), action);
    }

    action
}
