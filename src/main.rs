#[macro_use] extern crate prettytable;
mod content;

use std::io;
use std::io::*;
use std::process::Command;

use content::*;

enum Cmd {
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

        if cmds.len() == 0 {
            return Self::Empty
        }

        match cmds[0] {
            "lookup" => {
                if cmds.len() != 2 {
                    Self::InvalidUsage("lookup command must be length 2".to_owned())
                } else {
                    Self::Lookup(cmds[1].to_owned())
                }
            }
            "refresh" => Self::RefreshCache,
            s => Self::Unknown(s.to_owned()),
        }
    }
}

fn wait_for_cmd() -> String {
    print!(">> ");
    io::stdout().flush().unwrap();

    let mut cmd = String::new();

    std::io::stdin().read_line(&mut cmd).unwrap();
    cmd.trim().to_owned()
}

fn main() {
    loop {
        let cmd = wait_for_cmd();
        process_cmd(Cmd::from(cmd));
    }
}

fn process_cmd(cmd: Cmd) {
    match cmd {
        Cmd::Empty => {},
        Cmd::Unknown(s) => println!("Unknown command `{}`", s),
        Cmd::Lookup(name) => {
            let page = DocPage::fetch(false, &name);
            match page {
                Err(e) => println!("{:?}", e),
                Ok(p) => p.print_tableview(),
            }
        },
        Cmd::InvalidUsage(s) => println!("Invalid command usage: {}", s),
        Cmd::RefreshCache => {
            println!("Refresh the doc cache? (y/n)");
            println!("note: this may take a couple minutes depending on number of crates.");
            let confirm = wait_for_cmd();
            match Allow::from(confirm) {
                Allow::Yes => {
                    Command::new("cargo").arg("doc").spawn().expect("Failed to refresh docs");
                },
                Allow::No => return
            }
        },
    }
}

