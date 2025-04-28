use std::{fmt::Display, rc::Rc};

use super::{BackendError, FeedItem, NewsSite, Parser, parsers::FeedSelectors};
use crate::frontend::Components;
use chrono::NaiveDateTime;
use scraper::{ElementRef, Html, Selector};

pub struct N1;

impl Display for N1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N1")
    }
}

impl NewsSite for N1 {
    fn get_feed_url(&self, page: usize) -> String {
        format!("https://n1info.rs/najnovije/page/{}", page + 1)
    }
}

impl Parser for N1 {
    fn parse_feed_published(
        &self,
        article: ElementRef,
        selector: &Selector,
    ) -> Option<NaiveDateTime> {
        const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
        Some(
            article
                .select(&selector)
                .next()
                .and_then(|el| el.attr("datetime"))
                .and_then(|dt| NaiveDateTime::parse_from_str(dt, TIME_FORMAT).ok())?,
        )
    }

    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, BackendError> {
        super::parsers::parse_feed(
            Rc::new(Self),
            html,
            FeedSelectors {
                article: "article",
                url: "a",
                title: "h3",
                time: "time",
            },
        )
    }

    fn parse_article_content(&self, elem: ElementRef) -> Option<Components> {
        match elem.value().name() {
            "p" => {
                let text: String = elem.text().collect();
                if text.is_empty() {
                    return None;
                }
                if let Some(inner) = elem.child_elements().next() {
                    if inner.attr("data-attribute-id") == Some("emphasized-text") {
                        return Some(Components::Lead(text));
                    }
                }
                Some(Components::Paragraph(text))
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
                    return Some(Components::Boxed(paragraphs));
                }
                None
            }
            "h2" => {
                let text: String = elem.text().collect();
                if text.is_empty() {
                    return None;
                }
                Some(Components::Subtitle(text))
            }
            // TODO: Blog
            _ => None,
        }
    }

    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError> {
        super::parsers::parse_article(Rc::new(Self), html, ".entry-title", ".entry-content")
    }
}
