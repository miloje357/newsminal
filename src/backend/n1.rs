use super::{ArticleError, Scraper};
use crate::frontend::Components;
use scraper::{Html, Selector};

pub struct N1;
impl Scraper for N1 {
    fn get_domain(&self) -> &str {
        "https://n1info.rs/"
    }

    fn get_article(&self, html: Html) -> Result<Vec<Components>, ArticleError> {
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
}
