use super::{ArticleError, FeedItem, Scraper};
use crate::frontend::Components;
use chrono::NaiveDateTime;
use scraper::{Html, Selector};

pub struct N1;
impl Scraper for N1 {
    fn get_domain(&self) -> &str {
        "https://n1info.rs/"
    }

    fn get_feed_url(&self, page: usize) -> String {
        format!("{}najnovije/page/{}", self.get_domain(), page + 1)
    }

    fn parse_article(&self, html: Html) -> Result<Vec<Components>, ArticleError> {
        const TITLE_SELCTOR: &str = ".entry-title";
        const CONTENT_SELECTOR: &str = ".entry-content";
        let mut article = Vec::new();

        let title_selector = Selector::parse(TITLE_SELCTOR).unwrap();
        let title = html
            .select(&title_selector)
            .next()
            .ok_or(ArticleError::NoTitle)?
            .text()
            .collect();
        article.push(Components::Title(title));

        let content_selector = Selector::parse(CONTENT_SELECTOR).unwrap();
        let content = html
            .select(&content_selector)
            .next()
            .ok_or(ArticleError::NoContent)?
            .child_elements();
        for elem in content {
            match elem.value().name() {
                "p" => {
                    let text: String = elem.text().collect();
                    if text.is_empty() {
                        continue;
                    }
                    if let Some(inner) = elem.child_elements().next() {
                        if inner.attr("data-attribute-id") == Some("emphasized-text") {
                            article.push(Components::Lead(text));
                            continue;
                        }
                    }
                    article.push(Components::Paragraph(text))
                }
                "section" => {
                    let blockqoute_selector = Selector::parse("blockquote").unwrap();
                    let blockqoute = elem.select(&blockqoute_selector).next();
                    if let Some(blockqoute) = blockqoute {
                        let paragraphs: Vec<String> = blockqoute
                            .child_elements()
                            .filter(|e| e.value().name() == "p")
                            .map(|p| p.text().collect())
                            .collect();
                        article.push(Components::Boxed(paragraphs))
                    }
                }
                "h2" => {
                    let text: String = elem.text().collect();
                    if text.is_empty() {
                        continue;
                    }
                    article.push(Components::Subtitle(text))
                }
                // TODO: Blog
                _ => {}
            }
        }
        if article.len() == 1 {
            return Err(ArticleError::NoContent);
        }

        Ok(article)
    }

    // TODO: Error handling
    fn parse_feed(&self, html: Html) -> Result<Vec<FeedItem>, ArticleError> {
        let article_selector = Selector::parse("article").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let title_selector = Selector::parse("h3").unwrap();
        const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
        let time_selector = Selector::parse("time").unwrap();
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
                let published = a
                    .select(&time_selector)
                    .next()
                    .and_then(|el| el.attr("datetime"))
                    .and_then(|dt| NaiveDateTime::parse_from_str(dt, TIME_FORMAT).ok())?;
                Some(FeedItem {
                    url: Some(url),
                    title,
                    published: Some(published),
                })
            })
            .collect())
    }
}
