use std::fs::File;
use std::io::prelude::*;
use std::env;

use reqwest;
use scraper::{Html, Selector};
use prettytable::{Table, Row, Cell};
use prettytable::format;

#[derive(Debug)]
pub enum ContentError {
    DoesNotExist,
    LoadFailure,
    InvalidPage,
}

#[derive(Debug)]
pub enum PageType {
    All(Html),
    Index(Html),
}

#[derive(Debug)]
pub enum DocType {
    Module,
    Struct,
    Type,
    Trait,
}

impl From<&DocType> for String {
    fn from(dt: &DocType) -> Self {
        match dt {
            DocType::Module => "Modules".to_owned(),
            DocType::Struct => "Structs".to_owned(),
            DocType::Type => "Types".to_owned(),
            DocType::Trait => "Traits".to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct DocTypeListing {
    doc_type: DocType,
    docs: Vec<DocListing>,
}

#[derive(Debug)]
pub struct DocListing {
    name: String,
    url: String,
}

#[derive(Debug)]
pub struct DocPage {
    page_type: PageType,
    doc_blocks: Vec<DocTypeListing>,
}

impl DocPage {
    pub fn fetch(online: bool, crate_name: &str) -> Result<Self, ContentError> {
        let page = fetch_html(crate_name, online);
        match page {
            Err(e) => Err(e),
            Ok(p) => {
                let mut blocks: Vec<DocTypeListing> = Vec::new();
                for dt in vec![
                    DocType::Module,
                    DocType::Struct,
                    DocType::Type,
                    DocType::Trait
                ] {
                    let listing = gen_doc_listing(&p, dt);
                    match listing {
                        Ok(Some(l)) => blocks.push(l),
                        _ => continue
                    }
                }

                Ok(Self {
                    page_type: p,
                    doc_blocks: blocks,
                })
            }
        }
    }

    pub fn print_tableview(&self) {
        for block in &self.doc_blocks {
            let mut tbl = Table::new();
            tbl.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            tbl.set_titles(Row::new(vec![
                Cell::new(&String::from(&block.doc_type))
                    .style_spec("Fgb")
            ]));

            for listing in &block.docs {
                tbl.add_row(Row::new(vec![
                    Cell::new(&listing.name).style_spec("Fyi")
                ]));
            }
            tbl.printstd();
        }
    }
}

pub fn gen_doc_listing(page: &PageType, doc_type: DocType) -> Result<Option<DocTypeListing>, ContentError> {
    match page {
        PageType::All(html) => {
            let selector_str = match doc_type {
                DocType::Module => ".modules.docblock",
                DocType::Struct => ".structs.docblock",
                DocType::Type => ".typedefs.docblock",
                DocType::Trait => ".traits.docblock",
            };
            let selector = Selector::parse(selector_str).unwrap();

            let content = html.select(&selector).next();
            match content {
                // We didnt find anything that matches the 
                // docblock selector. This _will_ happen pretty often
                // as every crate does not export every type.
                None => Ok(None),
                Some(c) => {
                    // Every doc component is listed as an anchor
                    // within their given docblock. Iterate through
                    // these and collect them into listing structs to 
                    // be attached to the set of all doc listings
                    // within the DocTypeListing struct.
                    let mut listings: Vec<DocListing> = Vec::new();
                    let entry_selector = Selector::parse("a").unwrap();
                    for entry in c.select(&entry_selector) {
                        let name = entry.text().collect::<String>();
                        let url = match entry.value().attr("href") {
                            Some(u) => u.to_owned(),
                            None => "".to_owned(),
                        };
                        listings.push(DocListing {
                            name: name,
                            url: url,
                        })
                    }

                    Ok(Some(DocTypeListing {
                        doc_type: doc_type,
                        docs: listings,
                    }))
                }
            }

        },
        // We cannot fetch a doc listing page
        // from any pages that do not match the
        // previous types.
        _ => Err(ContentError::InvalidPage),
    }
}

pub fn fetch_live_html(crate_name: &str) -> Result<PageType, ContentError> {
    let url = format!("https://docs.rs/{}", crate_name);
    let resp = reqwest::blocking::get(&url);
    match resp {
        Ok(r) => {
            // We cannot know the exact url up front
            // since the url includes the version. However,
            // we can hit the base url and check what it resolves
            // to and then use that resolved url to get the 
            // all.html page.
            let url = format!("{}all.html", r.url());
            let resp = reqwest::blocking::get(&url);
            match resp {
                Err(_) => Err(ContentError::LoadFailure),
                Ok(r) => {
                    let body = r.text();
                    match body {
                        Err(_) => Err(ContentError::LoadFailure),
                        Ok(b) => Ok(PageType::All(Html::parse_document(&b)))
                    }
                }
            }
        },
        Err(_) => Err(ContentError::LoadFailure),
    }
}

pub fn fetch_html(crate_name: &str, online: bool) -> Result<PageType, ContentError> {
    if online {
        return fetch_live_html(crate_name)
    }                                            

    // We are going to grab the html pages from the 
    // cached `docs` directory created by cargo. We need
    // the working directory to get there.
    let path = env::current_dir().unwrap();
    let path = path.display();

    // Format the absolute path for the html page
    // that lists all the exports. Attempt to
    // load it into an html page from the file content string.
    let index_path = format!("{}/target/doc/{}/all.html", path, crate_name);
    let file = File::open(index_path);
    match file {
        Err(_) => Err(ContentError::DoesNotExist),
        Ok(mut f) => {
            let mut content = String::new();
            match f.read_to_string(&mut content) {
                Err(_) => Err(ContentError::LoadFailure),
                Ok(_) => Ok(PageType::All(Html::parse_document(&content)))
            }
        }
    }
}