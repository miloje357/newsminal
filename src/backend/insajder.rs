use std::{error::Error, fmt::Display, rc::Rc};

use chrono::{Local, NaiveDateTime};
use reqwest::blocking::Client;
use scraper::{ElementRef, Html};
use serde::Deserialize;

use crate::{Body, FeedItem, frontend::Components};

use super::{BackendError, NewsSite, parsers::Parser};

pub struct Insajder;

impl Display for Insajder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Δ")
    }
}

#[derive(Deserialize)]
struct Data {
    data: Items,
}

#[derive(Deserialize)]
struct Items {
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    body: String,
    lead: String,
    published_at: String,
    title: String,
}

impl NewsSite for Insajder {
    fn get_feed_items(&self, client: &Client) -> Result<Vec<FeedItem>, Box<dyn Error>> {
        const URL: &str = "https://insajder2-hasura.superdesk.org/v1/graphql";
        const QUERY: &str = "{\"query\": \"{items:swp_article(limit:50,offset:0,order_by:{published_at:desc}){lead published_at title body}}\"}";
        let data = client.post(URL).body(QUERY).send()?;
        let json = data.json::<Data>()?;
        Ok(json
            .data
            .items
            .into_iter()
            .map(|i| FeedItem {
                title: format!("[{}] {}", Self, i.title),
                published: NaiveDateTime::parse_from_str(&i.published_at, "%Y-%m-%dT%H:%M:%S")
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap(),
                at: None,
                body: Body::Fetched {
                    html: i.body,
                    lead: i.lead,
                },
                parser: Rc::new(Self),
            })
            .collect())
    }
}

impl Parser for Insajder {
    fn parse_article_content(&self, elem: ElementRef) -> Option<Components> {
        let text = elem.text().collect::<String>();
        match elem.value().name() {
            "p" => Some(Components::Paragraph(text)),
            "h2" => Some(Components::Subtitle(text)),
            _ => None,
        }
    }

    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError> {
        let body = html
            .root_element()
            .child_elements()
            .filter_map(|elem| Some(self.parse_article_content(elem)?))
            .collect::<Vec<_>>();
        if body.len() == 0 {
            return Err(BackendError::NoContent);
        }
        Ok(body)
    }
}
