mod n1;

use crate::frontend::Components;
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
            Ok(scr.get_article(html)?)
        }
        Err(err) => Err(Box::new(ArticleError::ServerError(err.to_string()))),
    }
}

trait Scraper {
    fn get_domain(&self) -> &str;
    fn get_article(&self, html: Html) -> Result<Vec<Components>, ArticleError>;
}

// TODO: Add more scrapers
