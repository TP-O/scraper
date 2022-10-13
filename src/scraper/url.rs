use std::{collections::HashMap, fmt::Display};

use regex::Regex;
use thirtyfour::{prelude::WebDriverResult, By};
use tokio::sync::mpsc::Sender;
use url::Url;

use super::new_driver;

#[derive(Clone, Copy, PartialEq)]
pub enum UrlTag {
    Img,
    Iframe,
    A,
    Link,
    Script,
    Source,
}

impl Display for UrlTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlTag::Img => write!(f, "img"),
            UrlTag::Iframe => write!(f, "iframe"),
            UrlTag::A => write!(f, "a"),
            UrlTag::Link => write!(f, "link"),
            UrlTag::Script => write!(f, "script"),
            UrlTag::Source => write!(f, "source"),
        }
    }
}

impl UrlTag {
    fn source_attr(&self) -> &str {
        match self {
            UrlTag::Img => "src",
            UrlTag::Iframe => "src",
            UrlTag::A => "href",
            UrlTag::Link => "href",
            UrlTag::Script => "src",
            UrlTag::Source => "src",
        }
    }
}

#[derive(Clone)]
pub struct ScrapeUrlOptions {
    tags: Vec<UrlTag>,
    regex: Regex,
}

impl Default for ScrapeUrlOptions {
    fn default() -> Self {
        Self {
            tags: vec![UrlTag::A],
            regex: Regex::new("(.*?)").unwrap(),
        }
    }
}

impl ScrapeUrlOptions {
    pub fn add_tag(mut self, tag: UrlTag) -> Self {
        if !self.tags.contains(&tag) {
            self.tags.push(tag)
        }

        self
    }

    pub fn remove_tag(mut self, tag: UrlTag) -> Self {
        match self.tags.iter().position(|&t| t == tag) {
            Some(removed_index) => {
                self.tags.remove(removed_index);
            }
            None => {}
        };

        self
    }

    pub fn set_regex(mut self, rule: &str) -> Self {
        match Regex::new(rule) {
            Ok(regex) => self.regex = regex,
            Err(_) => {}
        }

        self
    }
}

pub struct UrlScraper {
    tx: Sender<String>,
    options: ScrapeUrlOptions,
    url_counter: HashMap<String, usize>,
}

impl UrlScraper {
    pub fn new(tx: Sender<String>, options: ScrapeUrlOptions) -> Self {
        Self {
            tx,
            options,
            url_counter: HashMap::new(),
        }
    }

    pub fn is_url(url: &str) -> bool {
        let regex = Regex::new(r"^(?:http(s)?://)[\w.-]+(?:\.[\w\.-]+)+(.?)*$").unwrap();

        regex.is_match(url)
    }

    pub fn path_to_url(&self, url: &Url, path: &str) -> String {
        match path.starts_with("/") {
            true => url.join(format!("{path}").as_str()).unwrap().to_string(),
            false => format!("{}/{}", url.host_str().to_owned().unwrap(), path),
        }
    }

    fn is_matched(&self, url_str: &str) -> bool {
        self.options.regex.is_match(&url_str)
    }

    fn is_duplicate(&self, url_str: &str) -> bool {
        self.url_counter.get(url_str).is_some()
    }

    fn count_scraped_url(&mut self, url_str: String) {
        let old_counter = self.url_counter.get(&url_str).unwrap_or(&0);
        self.url_counter.insert(url_str, old_counter + 1);
    }

    fn is_valid(&self, url_str: &str) -> bool {
        self.is_matched(url_str) && !self.is_duplicate(url_str)
    }

    pub async fn scrape(mut self, urls: &Vec<&str>) -> WebDriverResult<()> {
        let driver = new_driver().await?;

        for url in urls {
            driver.goto(url).await?;

            match Url::parse(url) {
                Ok(parsed_url) => {
                    for tag_name in self.options.tags.clone() {
                        let tags = driver
                            .find_all(By::Tag(tag_name.to_string().as_str()))
                            .await?;

                        for tag in tags {
                            match tag.attr(tag_name.source_attr()).await? {
                                Some(src_value) => {
                                    let scraped_url = if Self::is_url(&src_value) {
                                        src_value
                                    } else {
                                        self.path_to_url(&parsed_url, &src_value).to_owned()
                                    };

                                    if self.is_valid(&scraped_url) {
                                        self.tx.send(scraped_url.clone()).await.unwrap();
                                        self.count_scraped_url(scraped_url);
                                    }
                                }
                                None => continue,
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(driver.quit().await?)
    }
}
