use std::{error::Error, fmt::Display, rc::Rc};

use scraper::Html;

use crate::{FeedItem, frontend::Components};

use super::{BackendError, NewsSite, Parser};

pub struct Danas;

impl Display for Danas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "D")
    }
}

impl NewsSite for Danas {
    fn get_feed_items(&self) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        super::parsers::get_feed_items(Rc::new(Self), "https://danas.rs/feed")
    }
}

impl Parser for Danas {
    fn parse_article_content(&self, elem: scraper::ElementRef) -> Option<Components> {
        match elem.value().name() {
            "p" => {
                let text: String = elem.text().collect();
                Some(Components::Paragraph(text))
            }
            "div" => {
                if elem.value().classes().any(|c| c == "post-intro-content") {
                    let lead_text = elem.text().collect();
                    return Some(Components::Lead(lead_text));
                }
                None
            }
            "blockquote" => {
                let paragraphs: Vec<String> = elem
                    .child_elements()
                    .filter(|e| e.value().name() == "p")
                    .map(|p| p.text().collect())
                    .collect();
                Some(Components::Boxed(paragraphs))
            }
            "h2" => {
                let text: String = elem.text().collect();
                Some(Components::Subtitle(text))
            }
            _ => None,
        }
    }

    // FIXME: BBC articles don't work
    //        (CONTENT_SELECTOR should be ".content div.flex .w-full div")
    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError> {
        super::parsers::parse_article(
            Rc::new(Self),
            html,
            ".post-title",
            ".content div.flex .w-full",
        )
    }
}
