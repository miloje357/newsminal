mod danas;
mod insajder;
mod n1;
mod parsers;

use crate::{Body, Feed, FeedItem, frontend::ComponentKind};
use chrono::{DateTime, Local};
use danas::Danas;
use insajder::Insajder;
use n1::N1;
use parsers::Parser;
use reqwest::blocking::Client;
use scraper::Html;
use std::{cmp, error::Error, fmt::Display, time::Instant};

#[derive(Debug)]
pub enum BackendError {
    NoContent,
    FeedError,
}

impl Error for BackendError {}
impl Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::NoContent => write!(f, "Couldn't scrape content from article HTML"),
            BackendError::FeedError => {
                write!(f, "Couldn't get any articles from feed (check logs)")
            }
        }
    }
}

pub trait NewsSite: Display + Parser {
    fn get_feed_items(&self, clinet: &Client) -> Result<Vec<FeedItem>, Box<dyn Error>>;
}

impl FeedItem {
    pub fn get_article(&self) -> Result<Vec<ComponentKind>, Box<dyn Error>> {
        match &self.body {
            Body::Fetched { html, lead } => {
                let mut body = vec![
                    ComponentKind::Title(self.title.clone()),
                    ComponentKind::Lead(lead.to_string()),
                ];
                let html = Html::parse_fragment(&html);
                body.extend(self.parser.parse_article(html)?);
                Ok(body)
            }
            Body::ToFetch { url } => {
                let mut body = vec![ComponentKind::Title(self.title.clone())];
                let html = reqwest::blocking::get(url)?;
                let html = html.error_for_status()?.text()?;
                let html = Html::parse_document(&html);
                body.extend(self.parser.parse_article(html)?);
                Ok(body)
            }
        }
    }
}

impl Feed {
    pub fn selected(&self) -> &FeedItem {
        &self.items[self.selected]
    }

    fn get_new_items(client: &Client) -> Vec<FeedItem> {
        let news_sites: &[Box<dyn NewsSite>] = &[Box::new(N1), Box::new(Danas), Box::new(Insajder)];
        let mut feed_items = Vec::new();
        let mut last_published = DateTime::<Local>::MIN_UTC.into();
        for scr in news_sites {
            match scr.get_feed_items(client) {
                Ok(new_feed_items) => {
                    if let Some(last) = new_feed_items.last() {
                        last_published = cmp::max(last_published, last.published);
                    }
                    feed_items.extend(new_feed_items)
                }
                Err(err) => eprintln!("Couldn't get articles from {}: {err}", scr),
            }
        }
        let mut feed_items = feed_items
            .into_iter()
            .filter(|item| item.published > last_published)
            .collect::<Vec<_>>();
        feed_items.sort_by(|a, b| b.published.cmp(&a.published));
        feed_items
    }

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let client = Client::new();
        let feed_items = Self::get_new_items(&client);
        if feed_items.is_empty() {
            return Err(Box::new(BackendError::FeedError));
        }
        Ok(Feed {
            time: Instant::now(),
            items: feed_items.into(),
            selected: 0,
            client,
        })
    }

    pub fn refresh(&mut self) -> Option<usize> {
        let all_articles = Self::get_new_items(&self.client);
        let first = self.items.get(0)?;
        let new_articles: Vec<FeedItem> = all_articles
            .into_iter()
            .take_while(|i| i.published > first.published)
            .collect();
        let num_new = new_articles.len();
        self.time = Instant::now();
        for new_article in new_articles.into_iter().rev() {
            self.items.push_front(new_article);
        }
        Some(num_new)
    }
}
