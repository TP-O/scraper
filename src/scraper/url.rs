use std::{collections::HashMap, fmt::Display, str::FromStr};

use async_trait::async_trait;
use regex::Regex;
use thirtyfour::By;
use tokio::sync::mpsc::Sender;
use url::Url;

use crate::error::ScrapeResult;

use super::{new_driver, Scrape};

#[derive(Clone, Copy, PartialEq, Debug)]
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

impl FromStr for UrlTag {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "img" => Ok(Self::Img),
            "iframe" => Ok(Self::Iframe),
            "a" => Ok(Self::A),
            "link" => Ok(Self::Link),
            "script" => Ok(Self::Script),
            "source" => Ok(Self::Source),
            _ => Err("Unsupported URL tag"),
        }
    }
}

impl UrlTag {
    fn source_attr(&self) -> String {
        let attr = match self {
            UrlTag::Img => "src",
            UrlTag::Iframe => "src",
            UrlTag::A => "href",
            UrlTag::Link => "href",
            UrlTag::Script => "src",
            UrlTag::Source => "src",
        };

        String::from(attr)
    }
}

#[derive(Clone)]
pub struct ScrapeUrlFilter {
    tags: Vec<UrlTag>,
    regex: Regex,
}

impl Default for ScrapeUrlFilter {
    fn default() -> Self {
        Self {
            tags: vec![UrlTag::A],
            regex: Regex::new("(.*?)").unwrap(),
        }
    }
}

impl ScrapeUrlFilter {
    pub fn replace_tags(&mut self, tags: Vec<UrlTag>) -> &mut Self {
        self.tags = tags;

        self
    }

    pub fn add_tag(&mut self, tag: UrlTag) -> &mut Self {
        if !self.tags.contains(&tag) {
            self.tags.push(tag)
        }

        self
    }

    pub fn remove_tag(&mut self, tag: UrlTag) -> &mut Self {
        match self.tags.iter().position(|&t| t == tag) {
            Some(removed_index) => {
                self.tags.remove(removed_index);
            }
            None => {}
        };

        self
    }

    pub fn set_regex(&mut self, rule: String) -> &mut Self {
        match Regex::new(&rule) {
            Ok(regex) => self.regex = regex,
            Err(_) => {}
        }

        self
    }
}

pub struct UrlScraper {
    tx: Sender<String>,
    filter: ScrapeUrlFilter,
    url_counter: HashMap<String, usize>,
}

impl UrlScraper {
    pub fn new(tx: Sender<String>, filter: ScrapeUrlFilter) -> Self {
        Self {
            tx,
            filter,
            url_counter: HashMap::new(),
        }
    }

    pub fn is_url(url: &str) -> bool {
        let regex = Regex::new(r"^(?:http(s)?://)[\w.-]+(?:\.[\w\.-]+)+(.?)*$").unwrap();

        regex.is_match(&url)
    }

    pub fn path_to_url(&self, url: &Url, path: String) -> String {
        match path.starts_with("/") {
            true => url.join(format!("{path}").as_str()).unwrap().to_string(),
            false => format!("{}/{}", url.host_str().to_owned().unwrap(), path),
        }
    }

    fn is_matched(&self, url_str: &str) -> bool {
        self.filter.regex.is_match(&url_str)
    }

    fn is_duplicate(&self, url_str: &str) -> bool {
        self.url_counter.get(url_str).is_some()
    }

    fn count_scraped_url(&mut self, url_str: &str) {
        let old_counter = self.url_counter.get(url_str).unwrap_or(&0);
        self.url_counter
            .insert(String::from(url_str), old_counter + 1);
    }

    fn is_valid(&self, url_str: &str) -> bool {
        self.is_matched(url_str) && !self.is_duplicate(url_str)
    }
}

#[async_trait]
impl Scrape for UrlScraper {
    async fn scrape(&mut self, urls: &Vec<String>) -> ScrapeResult<()> {
        let driver = new_driver().await?;

        for url in urls {
            driver.goto(url).await.unwrap();

            if let Ok(parsed_url) = Url::parse(url) {
                for tag_name in self.filter.tags.clone() {
                    let tags = driver
                        .find_all(By::Tag(&tag_name.to_string()))
                        .await
                        .unwrap();

                    for tag in tags {
                        if let Some(attr_value) = tag.attr(&tag_name.source_attr()).await.unwrap() {
                            let scraped_url = if Self::is_url(&attr_value) {
                                attr_value
                            } else {
                                self.path_to_url(&parsed_url, attr_value)
                            };

                            if self.is_valid(&scraped_url) {
                                self.count_scraped_url(&scraped_url);
                                self.tx.send(scraped_url).await.unwrap();
                            }
                        }
                    }
                }
            }
        }

        Ok(driver.quit().await.unwrap())
    }
}
