use std::rc::Rc;

use chrono::NaiveDateTime;
use scraper::{ElementRef, Html, Selector};

use crate::{FeedItem, frontend::Components};

use super::{BackendError, NewsSite};

pub trait Parser {
    fn parse_feed_title(&self, article: ElementRef, selector: &Selector) -> Option<String> {
        Some(
            article
                .select(&selector)
                .next()?
                .text()
                .collect::<String>()
                .trim()
                .to_string(),
        )
    }

    fn parse_feed_url(&self, article: ElementRef, selector: &Selector) -> Option<String> {
        Some(article.select(&selector).next()?.attr("href")?.to_string())
    }

    fn parse_feed_published(
        &self,
        article: ElementRef,
        selector: &Selector,
    ) -> Option<NaiveDateTime>;

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

    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, BackendError>;
    fn parse_article_content(&self, elem: ElementRef) -> Option<Components>;
    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError>;
}

pub struct FeedSelectors<'a> {
    pub article: &'a str,
    pub url: &'a str,
    pub title: &'a str,
    pub time: &'a str,
}

// TODO: Add rss feed parsing as well
pub fn parse_feed(
    parser: Rc<dyn NewsSite>,
    html: Html,
    selectors: FeedSelectors,
) -> Result<Vec<FeedItem>, BackendError> {
    let article_selector = Selector::parse(&selectors.article).unwrap();
    let link_selector = Selector::parse(&selectors.url).unwrap();
    let title_selector = Selector::parse(&selectors.title).unwrap();
    let time_selector = Selector::parse(&selectors.time).unwrap();
    Ok(html
        .select(&article_selector)
        .filter_map(|a| {
            // TODO: Write to stderr if any of the parser funcitons errored
            Some(FeedItem {
                url: parser.parse_feed_url(a, &link_selector)?,
                title: format!(
                    "({parser}) {}",
                    parser.parse_feed_title(a, &title_selector)?
                ),
                published: parser.parse_feed_published(a, &time_selector)?,
                at: None,
                parser: Rc::clone(&parser),
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
