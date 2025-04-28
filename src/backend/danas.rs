use std::{fmt::Display, rc::Rc};

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use scraper::{Html, Selector};

use crate::{FeedItem, frontend::Components};

use super::{BackendError, NewsSite, Parser, parsers::FeedSelectors};

pub struct Danas;

impl Display for Danas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "D")
    }
}

impl NewsSite for Danas {
    fn get_feed_url(&self, page: usize) -> String {
        format!("https://www.danas.rs/najnovije-vesti/page/{}", page + 1)
    }
}

impl Parser for Danas {
    fn parse_feed_published(
        &self,
        article: scraper::ElementRef,
        selector: &Selector,
    ) -> Option<NaiveDateTime> {
        const TIME_FORMAT: &str = "%d.%m.%Y. %H:%M";
        const TODAY: &str = "danas %H:%M";
        let published = article.select(&selector).next()?.text().collect::<String>();
        match NaiveDateTime::parse_from_str(&published, TIME_FORMAT) {
            Ok(dt) => Some(dt),
            Err(_) => {
                let time = NaiveTime::parse_from_str(&published, TODAY).ok()?;
                let today: NaiveDate = Local::now().date_naive();
                Some(NaiveDateTime::new(today, time))
            }
        }
    }

    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, BackendError> {
        super::parsers::parse_feed(
            Rc::new(Self),
            html,
            FeedSelectors {
                article: "article",
                url: "h3 a",
                title: "h3",
                time: ".published",
            },
        )
    }

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
