use std::{error::Error, rc::Rc};

use chrono::DateTime;
use reqwest::blocking::Client;
use rss::Channel;
use scraper::{ElementRef, Html, Selector};

use crate::{FeedItem, frontend::ComponentKind};

use super::{BackendError, NewsSite};

pub trait Parser {
    fn parse_article_content(&self, elem: ElementRef) -> Option<ComponentKind>;
    fn parse_article(&self, html: Html) -> Result<Vec<ComponentKind>, BackendError>;
}

pub fn get_feed_items(
    client: &Client,
    parser: Rc<dyn NewsSite>,
    url: &'static str,
) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let rss = client.get(url).send()?;
    let rss = rss.error_for_status()?.bytes()?;
    let rss = Channel::read_from(&rss[..])?;
    Ok(rss
        .items
        .iter()
        .filter_map(|item| {
            Some(FeedItem {
                title: format!("[{}] {}", parser, item.title()?),
                published: DateTime::parse_from_rfc2822(&item.pub_date()?).ok()?.into(),
                body: crate::Body::ToFetch {
                    url: item
                        .link()
                        .and_then(|url| (!url.contains("english")).then_some(url))?
                        .into(),
                },
                parser: parser.clone(),
            })
        })
        .collect())
}

pub fn parse_article(
    parser: Rc<dyn Parser>,
    html: Html,
    content_selector: &str,
) -> Result<Vec<ComponentKind>, BackendError> {
    let content_selector = Selector::parse(content_selector).unwrap();

    let article = html
        .select(&content_selector)
        .next()
        .ok_or(BackendError::NoContent)?
        .child_elements()
        .filter_map(|e| parser.parse_article_content(e))
        .collect::<Vec<_>>();
    if article.len() == 1 {
        return Err(BackendError::NoContent);
    }

    Ok(article)
}
