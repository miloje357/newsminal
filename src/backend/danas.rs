use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use scraper::{Html, Selector};

use crate::{FeedItem, frontend::Components};

use super::{BackendError, NewsSite};

pub struct Danas;
impl NewsSite for Danas {
    fn get_feed_url(&self, page: usize) -> String {
        format!("https://www.danas.rs/najnovije-vesti/page/{}", page + 1)
    }

    // FIXME: BBC articles don't work
    //        (CONTENT_SELECTOR should be ".content div.flex .w-full div")
    fn parse_article(&self, html: Html) -> Result<Vec<Components>, BackendError> {
        const TITLE_SELCTOR: &str = ".post-title";
        const CONTENT_SELECTOR: &str = ".content div.flex .w-full";
        let mut article = Vec::new();

        let title_selector = Selector::parse(TITLE_SELCTOR).unwrap();
        let title = html
            .select(&title_selector)
            .next()
            .ok_or(BackendError::NoTitle)?
            .text()
            .collect();
        article.push(Components::Title(title));

        let content_selector = Selector::parse(CONTENT_SELECTOR).unwrap();
        let content = html
            .select(&content_selector)
            .next()
            .ok_or(BackendError::NoContent)?
            .child_elements();
        for elem in content {
            match elem.value().name() {
                "p" => {
                    let text: String = elem.text().collect();
                    article.push(Components::Paragraph(text))
                }
                "div" => {
                    if elem.value().classes().any(|c| c == "post-intro-content") {
                        let lead_text = elem.text().collect();
                        article.push(Components::Lead(lead_text));
                    }
                }
                "blockquote" => {
                    let paragraphs: Vec<String> = elem
                        .child_elements()
                        .filter(|e| e.value().name() == "p")
                        .map(|p| p.text().collect())
                        .collect();
                    article.push(Components::Boxed(paragraphs))
                }
                "h2" => {
                    let text: String = elem.text().collect();
                    article.push(Components::Subtitle(text))
                }
                _ => {}
            }
        }
        if article.len() == 1 {
            return Err(BackendError::NoContent);
        }

        Ok(article)
    }

    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, BackendError> {
        let article_selector = Selector::parse("article").unwrap();
        let link_selector = Selector::parse("h3 a").unwrap();
        let title_selector = Selector::parse("h3").unwrap();
        const TIME_FORMAT: &str = "%d.%m.%Y. %H:%M";
        const TODAY: &str = "danas %H:%M";
        let time_selector = Selector::parse(".published").unwrap();
        Ok(html
            .select(&article_selector)
            .filter_map(|a| {
                let url = a.select(&link_selector).next()?.attr("href")?.to_string();
                let title = a
                    .select(&title_selector)
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string();
                let published = a.select(&time_selector).next()?.text().collect::<String>();
                let published = match NaiveDateTime::parse_from_str(&published, TIME_FORMAT) {
                    Ok(dt) => Some(dt),
                    Err(_) => {
                        let time = NaiveTime::parse_from_str(&published, TODAY).ok()?;
                        let today: NaiveDate = Local::now().date_naive();
                        Some(NaiveDateTime::new(today, time))
                    }
                }?;
                Some(FeedItem {
                    url: Some(url),
                    title: format!("(D) {title}"),
                    published: Some(published),
                    at: None,
                    scraper: Box::new(Self),
                })
            })
            .collect())
    }
}
