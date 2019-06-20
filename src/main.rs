#![allow(dead_code)]

//! twocents is a sentiment aggregation tool for keeping track of what people are saying about
//! certain things, according to reputable sources. It crawls for quotes people have said, keeps
//! track of the attribution, and stores it for later indexing.

use scraper::{Html, Selector};
use std::collections::HashSet;

const BBC_SEARCH: &str =
    "https://www.bbc.co.uk/search/more?page={page}&q={query}&filter=news&suggid=";

#[derive(Debug)]
struct Article {
    url: String,
    quotes: Vec<String>,
}

impl Article {
    fn from(url: String) -> Article {
        Article {
            url,
            quotes: vec![],
        }
    }
}

/// Search BBC news for a particular query, up to `depth` times.
fn search_bbc(query: &str, depth: usize) -> Vec<Article> {
    let query_without_duped_spaces: String = query
        .chars()
        .fold(vec![], |mut acc, c| {
            if c == ' ' {
                if let Some(e) = acc.last() {
                    if *e != ' ' {
                        acc.push(' ');
                    }
                }
            } else {
                acc.push(c);
            }

            acc
        })
        .into_iter()
        .collect();

    let mut links = HashSet::new();

    let query = query_without_duped_spaces.replace(" ", "+").to_lowercase();

    for i in 1..=depth {
        let url = BBC_SEARCH
            .replace("{query}", &query)
            .replace("{page}", &i.to_string());

        println!("Searching: {}", url);

        let mut resp = reqwest::get(&url).unwrap();
        let body = resp.text().unwrap();
        let fragment = Html::parse_document(&body);
        let article_headers = Selector::parse(".media-text a").unwrap();

        for article in fragment.select(&article_headers) {
            let element = article.value();
            if let Some(href) = element.attr("href") {
                links.insert(String::from(href));
            }
        }
    }

    links.into_iter().map(|url| Article::from(url)).collect()
}

// extract quotes
// extract date

fn has_quote(fragment: &str) -> bool {
    fragment.contains('"')
}

fn extract_sentences(fragment: &str) -> Vec<&str> {
    let quotes: Vec<usize> = fragment
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == '"')
        .map(|x| x.0)
        .collect();
    let stops: Vec<usize> = fragment
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == '.' || *c == '!' || *c == '?')
        .map(|x| x.0)
        .collect();

    let mut quote_cursor = 0;
    let mut valid_stops = vec![0];

    for stop_i in stops.into_iter() {
        while quotes[quote_cursor..].len() >= 2 && quotes[quote_cursor + 1] < stop_i {
            quote_cursor += 2;
        }

        if quotes[quote_cursor..].len() > 1 {
            let quote_start = quotes[quote_cursor];
            let quote_end = quotes[quote_cursor + 1];

            if stop_i > quote_start && stop_i < quote_end {
                continue;
            }
        }

        valid_stops.push(stop_i + 1);
    }

    valid_stops.push(fragment.len());

    let mut sentences = vec![];

    for slice in valid_stops.windows(2) {
        let sentence = fragment[slice[0]..slice[1]].trim();

        if sentence.len() > 1 {
            sentences.push(sentence);
        }
    }

    sentences
}

fn extract_story_bbc(url: &str) -> Vec<String> {
    let mut resp = reqwest::get(url).unwrap();
    let body = resp.text().unwrap();
    let fragment = Html::parse_document(&body);
    let story = Selector::parse("#page p").unwrap();

    let mut all_text = vec![];

    for body in fragment.select(&story) {
        let mut text: Vec<_> = body.text().collect();
        all_text.append(&mut text);
    }

    all_text.into_iter().map(|s| s.to_string()).collect()
}

fn main() {
    println!("Hello, world!");

    let articles = search_bbc("boris johnson", 1);

    for mut article in articles {
        let paragraphs = extract_story_bbc(&article.url);

        for p in paragraphs {
            let sentences = extract_sentences(&p);
            for s in sentences {
                if has_quote(&s) {
                    article.quotes.push(s.to_string());
                }
            }
        }

        println!("{:#?}", article);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_extraction() {
        assert_eq!(
            vec!["The dog.", "Runs around"],
            extract_sentences("The dog. Runs around")
        );
    }

    #[test]
    fn test_sentence_extraction_hanging_period() {
        assert_eq!(
            vec!["The dog.", "Runs around."],
            extract_sentences("The dog. Runs around.")
        );
    }

    #[test]
    fn test_sentence_extraction_ellipsis() {
        assert_eq!(
            vec!["The dog.", "Runs around"],
            extract_sentences("The dog... Runs around")
        );
    }

    #[test]
    fn test_sentence_extraction_with_quotes() {
        assert_eq!(
            vec!["The \"dog. Runs around\" does it?", "Yes"],
            extract_sentences("The \"dog. Runs around\" does it? Yes")
        );
    }
}
