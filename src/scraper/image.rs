use std::{
    fmt::Display,
    fs,
    io::{Error, Result as IoResult},
    str::FromStr,
    time::SystemTime,
};

use async_trait::async_trait;
use base64::decode;
use chrono::{DateTime, Utc};
use dataurl::DataUrl;
use derive_getters::Getters;
use thirtyfour::{
    fantoccini::error::CmdError,
    prelude::{WebDriverError, WebDriverResult},
    By, WebDriver, WebElement,
};
use tokio::sync::mpsc::Sender;

use super::{new_driver, Scrape};

#[derive(Clone, Copy, PartialEq)]
pub enum ImageMimeType {
    Jpeg,
    Png,
}

impl Display for ImageMimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageMimeType::Jpeg => write!(f, "image/jpeg"),
            ImageMimeType::Png => write!(f, "image/png"),
        }
    }
}

impl FromStr for ImageMimeType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image/jpeg" => Ok(Self::Jpeg),
            "image/png" => Ok(Self::Png),
            _ => Err("Unsupported MIME type"),
        }
    }
}

#[derive(Debug, Getters)]
pub struct ScrapedImage {
    title: String,
    mime_type: String,
    encoded_content: String,
}

impl ScrapedImage {
    pub fn save(&self, path: &str) -> IoResult<()> {
        match decode(self.encoded_content.clone()) {
            Ok(content) => {
                let slash_index = self.mime_type.find("/").unwrap_or(0);
                let extension = self
                    .mime_type
                    .chars()
                    .skip(slash_index + 1)
                    .collect::<String>();
                let now: DateTime<Utc> = SystemTime::now().into();
                let name = now.timestamp_millis();

                fs::create_dir_all(format!("{}/{}", path, self.title))?;
                fs::write(
                    format!("{}/{}/{}.{}", path, self.title, name, extension),
                    content,
                )?;

                Ok(())
            }
            Err(_) => Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "Base64 decode failed",
            )),
        }
    }
}

#[derive(Clone)]
pub struct ScrapeImageOptions {
    min_width: usize,
    min_height: usize,
    mime_types: Vec<ImageMimeType>,
}

impl Default for ScrapeImageOptions {
    fn default() -> Self {
        Self {
            min_width: 300,
            min_height: 300,
            mime_types: vec![ImageMimeType::Jpeg],
        }
    }
}

impl ScrapeImageOptions {
    pub fn set_min_width(mut self, width: usize) -> Self {
        self.min_width = width;

        self
    }

    pub fn set_min_height(mut self, height: usize) -> Self {
        self.min_height = height;

        self
    }

    pub fn add_mime_type(mut self, mime_type: ImageMimeType) -> Self {
        if !self.mime_types.contains(&mime_type) {
            self.mime_types.push(mime_type)
        }

        self
    }

    pub fn remove_mime_type(mut self, mime_type: ImageMimeType) -> Self {
        match self.mime_types.iter().position(|&t| t == mime_type) {
            Some(removed_index) => {
                self.mime_types.remove(removed_index);
            }
            None => {}
        };

        self
    }
}

pub struct ImageScraper {
    tx: Sender<ScrapedImage>,
    options: ScrapeImageOptions,
}

impl ImageScraper {
    pub fn new(tx: Sender<ScrapedImage>, options: ScrapeImageOptions) -> Self {
        Self { tx, options }
    }

    async fn is_valid_size(&self, img: &WebElement, width: usize, height: usize) -> bool {
        // Property value is only returned in the error =))
        match img.prop("width").await.err() {
            Some(WebDriverError::CmdError(CmdError::NotW3C(value))) => {
                if value.as_u64().unwrap_or(0) < width as u64 {
                    return false;
                }

                match img.prop("height").await.err() {
                    Some(WebDriverError::CmdError(CmdError::NotW3C(value))) => {
                        value.as_u64().unwrap_or(0) >= height as u64
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn is_valid_mime_type(&self, accepted_types: &Vec<ImageMimeType>, media_type: &str) -> bool {
        match ImageMimeType::from_str(media_type) {
            Ok(mime_type) => accepted_types.contains(&mime_type),
            Err(_) => false,
        }
    }

    // Format of data url: data:[<mediatype>][;base64],<data>
    fn get_data(
        &self,
        data_url: String,
        mime_types: &Vec<ImageMimeType>,
    ) -> Option<(String, String)> {
        match DataUrl::parse(data_url.as_str()) {
            Ok(parsed) => {
                if self.is_valid_mime_type(mime_types, parsed.get_media_type()) {
                    let comma_index = data_url.find(",").unwrap_or(0);
                    // Skip comma and all characters before it
                    let data = data_url.chars().skip(comma_index + 1).collect::<String>();

                    Some((String::from(parsed.get_media_type()), data))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    async fn read_data_url(
        &self,
        driver: &WebDriver,
        src: &String,
        mime_types: &Vec<ImageMimeType>,
    ) -> Option<(String, String)> {
        let result = driver
            .execute(
                format!(
                    "return fetch('{src}')
                    .then(response => response.blob())
                    .then(blob => new Promise(callback => {{
                        let reader = new FileReader();
                        reader.onload = function() {{
                            callback(this.result);
                        }};
                        reader.readAsDataURL(blob);
                    }}))
                    .then(data => data);"
                )
                .as_str(),
                vec![],
            )
            .await;

        match result {
            Ok(result) => match result.convert() {
                Ok(data_url) => self.get_data(data_url, mime_types),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}

#[async_trait]
impl Scrape for ImageScraper {
    async fn scrape(mut self, urls: &Vec<String>) -> WebDriverResult<()> {
        let driver = new_driver().await?;

        for url in urls {
            driver.goto(url).await?;

            let title = driver.title().await?;
            let img_tags = driver.find_all(By::Tag("img")).await?;

            for img in img_tags {
                if !self
                    .is_valid_size(&img, self.options.min_width, self.options.min_height)
                    .await
                {
                    continue;
                }

                match img.attr("src").await? {
                    Some(src) => match self
                        .read_data_url(&driver, &src, &self.options.mime_types)
                        .await
                    {
                        Some((mime_type, data)) => {
                            self.tx
                                .send(ScrapedImage {
                                    title: title.clone(),
                                    mime_type,
                                    encoded_content: data,
                                })
                                .await
                                .unwrap();
                        }
                        None => continue,
                    },
                    None => continue,
                };
            }
        }

        Ok(driver.quit().await?)
    }
}
