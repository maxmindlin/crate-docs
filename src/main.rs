#[macro_use] extern crate prettytable;
mod content;

use std::io;
use std::io::*;

use content::*;

enum Command {
    Lookup(String),
    Unknown
}

impl From<String> for Command {
    fn from(s: String) -> Self {
        let cmds: Vec<&str> = s.split(" ").collect();
        match cmds.len() {
            2 => {
                if cmds[0] == "lookup" {
                    Self::Lookup(cmds[1].to_owned())
                } else {
                    Self::Unknown
                }
            }
            _ => Self::Unknown
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

        match Command::from(cmd.to_owned()) {
            Command::Unknown => println!("Unknown command"),
            Command::Lookup(name) => {
                let page = DocPage::fetch(false, &name);
                match page {
                    Err(e) => println!("{:?}", e),
                    Ok(p) => p.print_tableview(),
                }
            }
        }
    }
}

