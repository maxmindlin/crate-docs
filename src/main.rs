mod content;

use rustyline::Editor;
use std::process::Command;

use content::*;

struct DocState {
    page: DocPage,
    available_docs: Vec<DocListing>,
}

impl From<DocPage> for DocState {
    fn from(page: DocPage) -> Self {
        let mut listings: Vec<DocListing> = Vec::new();
        for block in page.doc_blocks.clone() {
            listings.extend(block.docs.clone());
        }

        Self {
            page: page,
            available_docs: listings
        }
    }
}

impl DocState {
    fn search_doc_listings(&self, target: &str) -> Option<DocListing> {
        // First search for exact matches
        for doc in &self.available_docs {
            if &doc.name == target {
                return Some(doc.clone())
            }
        }

        // No exact matches - search for anything
        // that ends with target. If you
        // do this first you could fail to
        // return an exact match if it comes later.
        for doc in &self.available_docs {
            if doc.name.ends_with(target) {
                return Some(doc.clone())
            }
        }

        None
    }
}

enum Cmd {
    Doc(String),
    Lookup(String),
    Unknown(String),
    RefreshCache,
    Empty,
    InvalidUsage(String),
}

enum Allow {
    Yes,
    No,
}

impl From<&str> for Allow {
    fn from(s: &str) -> Self {
        match s {
            "y" | 
            "Y" |
            "yes" |
            "Yes" |
            "YES" => Self::Yes,
            _ => Self::No
        }
    }
}

impl From<String> for Allow {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<String> for Cmd {
    fn from(s: String) -> Self {
        let cmds: Vec<&str> = s.split(" ").collect();

        if cmds == vec![""] {
            return Self::Empty
        }

        match cmds[0] {
            "lup" => {
                if cmds.len() != 2 {
                    Self::InvalidUsage("lookup command must be length 2".to_owned())
                } else {
                    Self::Lookup(cmds[1].to_owned())
                }
            }
            "rc" => Self::RefreshCache,
            "doc" => {
                if cmds.len() != 2 {
                    Self::InvalidUsage("doc command must be length 2".to_owned())
                } else {
                    Self::Doc(cmds[1].to_owned())
                }
            }
            s => Self::Unknown(s.to_owned()),
        }
    }
}

fn wait_for_input<T: From<String>>(prompt: &str, editor: &mut Editor<()>) -> T {
    match editor.readline(prompt) {
        Ok(line) => {
            editor.add_history_entry(line.as_str());
            T::from(line)
        }
        Err(_) => std::process::exit(1)
    }
}

fn wait_for_cmd(prompt: &str, editor: &mut Editor<()>) -> Cmd {
    let p = format!("{} ", prompt);
    wait_for_input::<Cmd>(&p, editor)
}

fn wait_for_permission(prompt: &str, editor: &mut Editor<()>) -> Allow {
    let p = format!("{} (y/n) ", prompt);
    wait_for_input::<Allow>(&p, editor)
}

fn main() {
    let mut e = Editor::<()>::new();
    loop {
        let cmd = wait_for_cmd(">>", &mut e);
        process_cmd(cmd, &mut e);
    }
}

fn process_cmd(cmd: Cmd, editor: &mut Editor<()>) {
    match cmd {
        Cmd::Empty => {},
        Cmd::Unknown(s) => println!("Unknown command `{}`", s),
        Cmd::Lookup(name) => process_crate_fetch_cmds(false, &name, editor),
        Cmd::InvalidUsage(s) => println!("Invalid command usage: {}", s),
        Cmd::RefreshCache => process_refresh_cmd(editor),
        _ => println!("Must lookup a crate before that cmd can be used")
    }
}

fn process_crate_fetch_cmds(online: bool, name: &str, editor: &mut Editor<()>) {
    loop {
        let page = DocPage::fetch(online, &name);
        match page {
            Err(ContentError::DoesNotExist) if !online => {
                let confirm = wait_for_permission("Docs are missing - fetch live page? >>", editor);
                match Allow::from(confirm) {
                    Allow::Yes => process_crate_fetch_cmds(true, name, editor),
                    Allow::No => break
                }
            }
            Err(ContentError::DoesNotExist) => {
                println!("Docs for {} cannot be found", &name);
                break;
            }
            Err(ContentError::LoadFailure) if online => {
                println!("Failed to fetch html, 
                make sure you are connected to the internet.");
                break;
            },
            Err(e) => println!("{:?}", e),
            Ok(p) => process_opened_crate_cmds(p, &name, editor),
        }
    }
}

fn process_opened_crate_cmds(p: DocPage, name: &str, editor: &mut Editor<()>) {
    // Here we have a valid doc page open.
    // Enter a new state where we are looping
    // cmds onto this doc page.
    let state = DocState::from(p);
    state.page.print_tableview();
    loop {
        let cmd_prmpt = format!("( {} ) >>", &name);
        let cmd = Cmd::from(wait_for_cmd(&cmd_prmpt, editor));
        match cmd {
            Cmd::Lookup(_) => process_cmd(cmd, editor),
            Cmd::Empty => continue,
            Cmd::RefreshCache => process_refresh_cmd(editor),
            Cmd::Doc(s) => {
                match state.search_doc_listings(&s) {
                    None => println!("Did not match any docs"),
                    Some(d) => println!("{} {}", &d.name, &d.url),
                }
            }
            _ => continue,
        }
    }
}

fn process_refresh_cmd(editor: &mut Editor<()>) {
    let confirm = wait_for_permission("Refresh the doc cache? >>", editor);
    match Allow::from(confirm) {
        Allow::Yes => {
            Command::new("cargo").arg("doc").spawn().expect("Failed to refresh docs");
        },
        Allow::No => {
            println!("Skipping refresh.");
        }
    }
}
