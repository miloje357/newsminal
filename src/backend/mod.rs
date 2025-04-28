mod danas;
mod n1;
mod parsers;

use crate::{Feed, FeedItem, frontend::Components};
use chrono::Utc;
use danas::Danas;
use n1::N1;
use parsers::Parser;
use scraper::Html;
use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum BackendError {
    NoScraper,
    NoTitle,
    NoContent,
    ServerError(String),
    FeedError,
}

impl Error for BackendError {}
impl Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::NoTitle => write!(f, "Couldn't scrape title from article HTML"),
            BackendError::NoContent => write!(f, "Couldn't scrape paragraphs from article HTML"),
            BackendError::ServerError(err) => write!(f, "Server returned an error: {err}"),
            BackendError::FeedError => {
                write!(f, "Couldn't get any articles from feed (check logs)")
            }
            BackendError::NoScraper => write!(f, "No support for this site"),
        }
    }
}

pub trait NewsSite: Parser + Display {
    fn get_feed_url(&self, page: usize) -> String;
    fn get_feed_items(&self, page: usize) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        let url = self.get_feed_url(page);
        let html = reqwest::blocking::get(url)?;
        let html = html.error_for_status()?;
        let html = Html::parse_document(&html.text()?);
        Ok(self.parse_feed(html)?)
    }
}

impl FeedItem {
    pub fn get_article(&self) -> Result<Vec<Components>, Box<dyn Error>> {
        let html = reqwest::blocking::get(&self.url)?;
        match html.error_for_status() {
            Ok(html) => {
                let html = Html::parse_document(&html.text()?);
                Ok(self.parser.parse_article(html)?)
            }
            Err(err) => Err(Box::new(BackendError::ServerError(err.to_string()))),
        }
    }
}

impl Feed {
    pub fn selected(&self) -> &FeedItem {
        &self.items[self.selected]
    }

    // TODO: async
    fn get_new_items(page: usize) -> Vec<FeedItem> {
        let news_sites: &[Box<dyn NewsSite>] = &[Box::new(N1), Box::new(Danas)];
        let mut feed_items = Vec::new();
        for scr in news_sites {
            match scr.get_feed_items(page) {
                Ok(new_feed_items) => feed_items.extend(new_feed_items),
                Err(err) => eprintln!("Couldn't get articles from: {err}"),
            }
        }
        feed_items.sort_by(|a, b| b.published.cmp(&a.published));
        feed_items
    }

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let feed_items = Self::get_new_items(0);
        if feed_items.is_empty() {
            return Err(Box::new(BackendError::FeedError));
        }
        Ok(Feed {
            time: Utc::now().naive_utc(),
            items: feed_items.into(),
            selected: 0,
            page: 0,
        })
    }

    pub fn refresh(&mut self, is_manual: bool) -> Option<usize> {
        let now = Utc::now().naive_utc();
        if !is_manual && (now - self.time).num_minutes() < 1 {
            return None;
        }

        // FIXME: Scrape not just the first page but all up until the self.items[0]
        let all_articles = Self::get_new_items(0);
        let first = self.items.get(0)?;
        let new_articles: Vec<FeedItem> = all_articles
            .into_iter()
            .take_while(|i| i.published > first.published)
            .collect();
        let num_new = new_articles.len();

        self.time = now;
        for new_article in new_articles {
            self.items.push_front(new_article);
        }
        Some(num_new)
    }

    // FIXME: Figure out a better way always have articles in order (perhaps hardcode say 20
    //        articles per append and then cache the unused articles)
    pub fn next_page(&mut self) -> Result<Vec<FeedItem>, BackendError> {
        self.page += 1;
        let new_feed_items = Self::get_new_items(self.page);
        if new_feed_items.is_empty() {
            return Err(BackendError::FeedError);
        }
        Ok(new_feed_items)
    }
}

// TODO: Add more scrapers
