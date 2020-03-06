use std::fs::File;
use std::io::prelude::*;
use std::env;

use url::Url;
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

#[derive(Debug, Clone, Copy)]
pub enum DocType {
    Module,
    Struct,
    Type,
    Trait,
    Enum,
    Function,
    Constant,
    Other,
}

impl From<&DocType> for String {
    fn from(dt: &DocType) -> Self {
        match dt {
            DocType::Module => "Modules".to_owned(),
            DocType::Struct => "Structs".to_owned(),
            DocType::Type => "Types".to_owned(),
            DocType::Trait => "Traits".to_owned(),
            DocType::Enum => "Enums".to_owned(),
            DocType::Function => "Functions".to_owned(),
            DocType::Constant => "Constants".to_owned(),
            DocType::Other => "Others".to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct DocTypeListing {
    pub doc_type: DocType,
    pub docs: Vec<DocListing>,
}

impl Clone for DocTypeListing {
    fn clone(&self) -> Self {
        DocTypeListing {
            doc_type: self.doc_type,
            docs: self.docs.clone()
        }
    }
}

#[derive(Debug)]
pub struct DocListing {
    pub name: String,
    pub url: String,
}

impl Clone for DocListing {
    fn clone(&self) -> Self {
        DocListing {
            name: self.name.to_owned(),
            url: self.url.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct DocPage {
    pub page_type: PageType,
    pub doc_blocks: Vec<DocTypeListing>,
}

impl DocPage {
    pub fn fetch(online: bool, crate_name: &str) -> Result<Self, ContentError> {
        let page = fetch_html(crate_name, online);
        match page {
            Err(e) => Err(e),
            Ok((p, u)) => {
                let docs = gen_doc_listings(&p, &u);
                match docs {
                    Err(e) => Err(e),
                    Ok(d) => Ok(Self {
                        page_type: p,
                        doc_blocks: d,
                    })
                }
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

pub fn gen_doc_listings(page: &PageType, base_url: &str) -> Result<Vec<DocTypeListing>, ContentError> {
    match page {
        PageType::All(html) => {
            // On the All page the only ul nodes are those
            // that belong to a grouping of listings, grouped
            // under the same type.
            let selector = Selector::parse("ul").unwrap();
            let mut docs: Vec<DocTypeListing> = Vec::new();
            for c in html.select(&selector) {
                // Every doc component is listed as an anchor
                // within their given docblock. Iterate through
                // these and collect them into listing structs to 
                // be attached to the set of all doc listings
                // within the DocTypeListing struct.

                let classes: Vec<&str> = match c.value().attr("class") {
                    Some(c) => c.split(" ").collect(),
                    None => Vec::new()
                };

                // This node either doesnt have a class attr or 
                // is a menu item, either way - dont want it.
                if classes.len() == 0 || classes[0].starts_with("pure") {
                    continue
                }

                let dtype = match classes[0] {
                    "modules" => DocType::Module,
                    "structs" => DocType::Struct,
                    "typedefs" => DocType::Type,
                    "traits" => DocType::Trait,
                    "enums" => DocType::Enum,
                    "functions" => DocType::Function,
                    "constants" => DocType::Constant,
                    _ => DocType::Other
                };

                let mut listings: Vec<DocListing> = Vec::new();
                let entry_selector = Selector::parse("a").unwrap();
                for entry in c.select(&entry_selector) {
                    let name = entry.text().collect::<String>();
                    let url = match entry.value().attr("href") {
                        Some(u) => {
                            match Url::parse(base_url) {
                                Ok(base) => {
                                    // These are easy, we have real URLs
                                    base.join(u)
                                        .unwrap()
                                        .as_str()
                                        .to_owned()
                                },
                                Err(url::ParseError::RelativeUrlWithoutBase) => {
                                    // We should probably do this better,
                                    // but this works for now.
                                    // In here is when we have file locations,
                                    // not urls. If we could parse file locations
                                    // as urls this would be better.
                                    base_url.replace("all.html", u).to_owned()
                                },
                                Err(_) => continue
                            }
                        },
                        // if this anchor doesnt have an href (which should never happen?)
                        // then we dont want it.
                        None => continue,
                    };
                    listings.push(DocListing {
                        name: name,
                        url: url,
                    })
                }

                // if its empty then we dont want it in the master set. That would
                // result in empty tables.
                if listings.len() > 0 {
                    docs.push(DocTypeListing {
                        doc_type: dtype,
                        docs: listings,
                    });
                }
            }

            Ok(docs)
        },
        // We cannot fetch a doc listing page
        // from any pages that do not match the
        // previous types.
        _ => Err(ContentError::InvalidPage),
    }
}

pub fn fetch_live_html(crate_name: &str) -> Result<(PageType, String), ContentError> {
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
                    match r.status() {
                        reqwest::StatusCode::OK => {
                            let body = r.text();
                            match body {
                                Err(_) => Err(ContentError::LoadFailure),
                                Ok(b) => Ok((PageType::All(Html::parse_document(&b)), url))
                            }
                        },
                        _ =>Err(ContentError::DoesNotExist),
                    }
                }
            }
        },
        Err(_) => Err(ContentError::LoadFailure),
    }
}

pub fn fetch_html(crate_name: &str, online: bool) -> Result<(PageType, String), ContentError> {
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
    let file = File::open(index_path.clone());
    match file {
        Err(_) => Err(ContentError::DoesNotExist),
        Ok(mut f) => {
            let mut content = String::new();
            match f.read_to_string(&mut content) {
                Err(_) => Err(ContentError::LoadFailure),
                Ok(_) => Ok((PageType::All(Html::parse_document(&content)), index_path))
            }
        }
    }
}