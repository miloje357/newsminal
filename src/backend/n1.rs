use std::{error::Error, fmt::Display, rc::Rc};

use super::{BackendError, FeedItem, NewsSite, Parser};
use crate::frontend::Components;
use reqwest::blocking::Client;
use scraper::{CaseSensitivity::CaseSensitive, ElementRef, Html};

pub struct N1;

impl Display for N1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N1")
    }
}

impl NewsSite for N1 {
    fn get_feed_items(&self, client: &Client) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        super::parsers::get_feed_items(
            client,
            Rc::new(Self),
            "https://n1info.rs/feed",
            Some("%a, %d %b %Y %H:%M:%S %:z"),
        )
    }
}

impl Parser for N1 {
    fn parse_article_content(&self, elem: ElementRef) -> Option<Components> {
        let text: String = elem.text().collect::<String>().trim().into();
        if text.is_empty() {
            return None;
        }
        if elem.value().name() == "p" {
            return Some(Components::Lead(text));
        }

        if elem.value().name() != "div" {
            return None;
        }

        if !elem
            .value()
            .has_class("article-content-wrapper", CaseSensitive)
        {
            return None;
        }
        let Some(first_child) = elem.child_elements().next() else {
            return None;
        };

        if first_child
            .value()
            .has_class("related-news-block", CaseSensitive)
            || first_child.value().has_class("aspect-video", CaseSensitive)
        {
            return None;
        }

        let Some(grandchild) = first_child.child_elements().next() else {
            return None;
        };

        if grandchild.value().name() == "h2" {
            Some(Components::Subtitle(text))
        } else if grandchild.value().has_class("twitter-tweet", CaseSensitive) {
            Some(Components::Boxed(
                grandchild.text().map(|line| line.to_string()).collect(),
            ))
        } else {
            Some(Components::Paragraph(text))
        }
    }

    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError> {
        super::parsers::parse_article(Rc::new(Self), html, ".article-wrapper")
    }
}
