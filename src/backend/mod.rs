mod n1;

use crate::{Feed, FeedItem, frontend::Components};
use chrono::Utc;
use n1::N1;
use scraper::Html;
use std::{error::Error, fmt::Display};

#[derive(Debug)]
struct NoScraper;
impl Error for NoScraper {}
impl Display for NoScraper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No support for this site")
    }
}

#[derive(Debug)]
pub enum ArticleError {
    NoTitle,
    NoContent,
    ServerError(String),
}

impl Error for ArticleError {}
impl Display for ArticleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArticleError::NoTitle => write!(f, "No title in the HTML"),
            ArticleError::NoContent => write!(f, "No content in the HTML"),
            ArticleError::ServerError(err) => write!(f, "Server returned an error: {err}"),
        }
    }
}

// TODO: Add scraper: Box<dyn Scraper> to this function
pub fn get_article(url: &str) -> Result<Vec<Components>, Box<dyn Error>> {
    let scrapers: Vec<Box<dyn Scraper>> = vec![Box::new(N1)];

    let scr = scrapers
        .into_iter()
        .find(|scr| url.starts_with(scr.get_domain()))
        .ok_or(NoScraper)?;

    let html = reqwest::blocking::get(url)?;
    match html.error_for_status() {
        Ok(html) => {
            let html = Html::parse_document(&html.text()?);
            Ok(scr.parse_article(html)?)
        }
        Err(err) => Err(Box::new(ArticleError::ServerError(err.to_string()))),
    }
}

trait Scraper {
    fn get_domain(&self) -> &str;
    fn get_feed_url(&self, page: usize) -> String;
    fn parse_article(&self, html: Html) -> Result<Vec<Components>, ArticleError>;
    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, ArticleError>;
}

impl Feed {
    fn get_feed_site(url: &str, scr: Box<dyn Scraper>) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        let html = reqwest::blocking::get(url)?;
        let html = html.error_for_status()?;
        let html = Html::parse_document(&html.text()?);
        Ok(scr.parse_feed(html)?)
    }

    // TODO: Add scraping from multiple sources
    // TODO: Add scraping from multiple pages of the same source
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let scrapers: Vec<Box<dyn Scraper>> = vec![Box::new(N1)];
        let mut feed_items = Vec::new();
        for scr in scrapers {
            let url = scr.get_feed_url(0);
            match Self::get_feed_site(&url, scr) {
                Ok(new_feed_items) => {
                    feed_items.extend(new_feed_items);
                }
                Err(err) => {
                    let title = format!("Couldn't get articles from {}: {err}", url);
                    feed_items.push(FeedItem {
                        url: None,
                        title,
                        published: None,
                        at: None,
                    })
                }
            }
        }

        Ok(Feed {
            time: Utc::now().naive_utc(),
            items: feed_items,
            selected: 0,
        })
    }

    pub fn get_selected_url(&self) -> &str {
        self.items[self.selected].url.as_ref().unwrap()
    }

    // TODO: refresh()
}

// TODO: Add more scrapers
