use std::{error::Error, fmt::Display, rc::Rc};

use super::{BackendError, FeedItem, NewsSite, Parser};
use crate::frontend::ComponentKind;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};

pub struct N1;

impl Display for N1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N1")
    }
}

impl NewsSite for N1 {
    fn get_feed_items(&self, client: &Client) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        super::parsers::get_feed_items(client, Rc::new(Self), "https://n1info.rs/feed")
    }
}

impl Parser for N1 {
    fn parse_article_content(&self, elem: ElementRef) -> Option<ComponentKind> {
        match elem.value().name() {
            "p" => {
                let text: String = elem.text().collect();
                if text.is_empty() {
                    return None;
                }
                if let Some(inner) = elem.child_elements().next() {
                    if inner.attr("data-attribute-id") == Some("emphasized-text") {
                        return Some(ComponentKind::Lead(text));
                    }
                }
                Some(ComponentKind::Paragraph(text))
            }
            "section" => {
                let blockqoute_selector = Selector::parse("blockquote").unwrap();
                let blockqoute = elem.select(&blockqoute_selector).next();
                if let Some(blockqoute) = blockqoute {
                    let paragraphs: Vec<String> = blockqoute
                        .child_elements()
                        .filter(|e| e.value().name() == "p")
                        .map(|p| p.text().collect())
                        .collect();
                    return Some(ComponentKind::Boxed(paragraphs));
                }
                None
            }
            "h2" => {
                let text: String = elem.text().collect();
                if text.is_empty() {
                    return None;
                }
                Some(ComponentKind::Subtitle(text))
            }
            // TODO: Blog
            _ => None,
        }
    }

    fn parse_article(&self, html: Html) -> Result<Vec<ComponentKind>, BackendError> {
        super::parsers::parse_article(Rc::new(Self), html, ".entry-content")
    }
}
