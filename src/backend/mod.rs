mod n1;

use crate::{Feed, FeedItem, frontend::Components};
use chrono::Utc;
use n1::N1;
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

// TODO: Add scraper: Box<dyn Scraper> to this function
pub fn get_article(url: &str) -> Result<Vec<Components>, Box<dyn Error>> {
    let scrapers: Vec<Box<dyn Scraper>> = vec![Box::new(N1)];

    let scr = scrapers
        .into_iter()
        .find(|scr| url.starts_with(scr.get_domain()))
        .ok_or(BackendError::NoScraper)?;

    let html = reqwest::blocking::get(url)?;
    match html.error_for_status() {
        Ok(html) => {
            let html = Html::parse_document(&html.text()?);
            Ok(scr.parse_article(html)?)
        }
        Err(err) => Err(Box::new(BackendError::ServerError(err.to_string()))),
    }
}

trait Scraper {
    fn get_domain(&self) -> &str;
    fn get_feed_url(&self, page: usize) -> String;
    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError>;
    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, BackendError>;
}

impl Feed {
    fn get_feed_site(url: &str, scr: Box<dyn Scraper>) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        let html = reqwest::blocking::get(url)?;
        let html = html.error_for_status()?;
        let html = Html::parse_document(&html.text()?);
        Ok(scr.parse_feed(html)?)
    }

    // TODO: Add scraping from multiple pages of the same source
    fn get_new_items() -> Vec<FeedItem> {
        let scrapers = vec![Box::new(N1)];
        let mut feed_items = Vec::new();
        for scr in scrapers {
            let url = scr.get_feed_url(0);
            match Self::get_feed_site(&url, scr) {
                Ok(new_feed_items) => feed_items.extend(new_feed_items),
                Err(err) => eprintln!("Couldn't get articles from: {err}"),
            }
        }
        // TODO: sort
        feed_items
    }

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let feed_items = Self::get_new_items();
        if feed_items.is_empty() {
            return Err(Box::new(BackendError::FeedError));
        }
        Ok(Feed {
            time: Utc::now().naive_utc(),
            items: feed_items.into(),
            selected: 0,
        })
    }

    pub fn get_selected_url(&self) -> &str {
        self.items[self.selected].url.as_ref().unwrap()
    }

    pub fn refresh(&mut self, is_manual: bool) -> Option<usize> {
        let now = Utc::now().naive_utc();
        if !is_manual && (now - self.time).num_minutes() < 1 {
            return None;
        }

        let all_articles = Self::get_new_items();
        let first = self.items.get(0)?;
        let new_articles: Vec<FeedItem> = all_articles
            .into_iter()
            .take_while(|i| {
                i.published
                    .map_or(false, |ip| first.published.map_or(false, |fp| ip > fp))
            })
            .collect();
        let num_new = new_articles.len();

        self.time = now;
        for new_article in new_articles.into_iter().rev() {
            self.items.push_front(new_article);
        }
        Some(num_new)
    }
}

// TODO: Add more scrapers
