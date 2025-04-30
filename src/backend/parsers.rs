use std::{error::Error, rc::Rc};

use chrono::DateTime;
use rss::Channel;
use scraper::{ElementRef, Html, Selector};

use crate::{FeedItem, frontend::Components};

use super::{BackendError, NewsSite};

pub trait Parser {
    fn parse_article_title(
        &self,
        html: &Html,
        selector: &Selector,
    ) -> Result<String, BackendError> {
        Ok(html
            .select(selector)
            .next()
            .ok_or(BackendError::NoTitle)?
            .text()
            .collect::<String>())
    }

    fn parse_article_content(&self, elem: ElementRef) -> Option<Components>;
    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError>;
}

pub fn get_feed_items(
    parser: Rc<dyn NewsSite>,
    url: &'static str,
) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let rss = reqwest::blocking::get(url)?;
    let rss = rss.error_for_status()?.bytes()?;
    let rss = Channel::read_from(&rss[..])?;
    Ok(rss
        .items
        .iter()
        .filter_map(|item| {
            Some(FeedItem {
                url: item
                    .link
                    .clone()
                    .and_then(|url| (!url.contains("english")).then_some(url))?,
                title: format!("({}) {}", parser, item.title.clone()?),
                published: DateTime::parse_from_rfc2822(&item.pub_date.clone()?)
                    .ok()?
                    .into(),
                at: None,
                parser: parser.clone(),
            })
        })
        .collect())
}

pub fn parse_article(
    parser: Rc<dyn Parser>,
    html: Html,
    title_selector: &str,
    content_selector: &str,
) -> Result<Vec<Components>, BackendError> {
    let title_selector = Selector::parse(title_selector).unwrap();
    let content_selector = Selector::parse(content_selector).unwrap();
    let mut article = Vec::new();

    let title = parser.parse_article_title(&html, &title_selector)?;
    article.push(Components::Title(title));

    article.extend(
        html.select(&content_selector)
            .next()
            .ok_or(BackendError::NoContent)?
            .child_elements()
            .filter_map(|e| parser.parse_article_content(e)),
    );
    if article.len() == 1 {
        return Err(BackendError::NoContent);
    }

    Ok(article)
}

// TODO: Add more scrapers
