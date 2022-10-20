use std::{
    fs::{self, File},
    io::Write,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use thirtyfour::prelude::WebDriverResult;
use tokio::sync::mpsc::channel;

use super::util::*;
use crate::scraper::*;

pub struct CommandLineInterface {
    //
}

impl CommandLineInterface {
    pub fn new() -> Self {
        Self {}
    }

    async fn scrape_images(
        urls: &Vec<String>,
        strategies: ScrapeStrategies,
        options: ScrapeImageOptions,
    ) -> WebDriverResult<()> {
        let (tx, mut rx) = channel(100);

        for i in 0..*strategies.number_of_windows() {
            // Split urls to smaller batches
            match get_batch_range(urls.len(), *strategies.number_of_windows(), i) {
                Some((start, end)) => {
                    let tx_clone = tx.clone();
                    let opt_clone = options.clone();
                    let sub_urls = Vec::from(&urls[start..end]);

                    tokio::spawn(async move {
                        let scraper = ImageScraper::new(tx_clone, opt_clone);
                        scraper.scrape(&sub_urls).await.unwrap();
                    });
                }
                None => break,
            }
        }

        drop(tx);

        while let Some(data) = rx.recv().await {
            data.save(strategies.dest_dir())?;
        }

        Ok(())
    }

    async fn scrape_urls(
        urls: &Vec<String>,
        strategies: ScrapeStrategies,
        options: ScrapeUrlOptions,
    ) -> WebDriverResult<()> {
        let (tx, mut rx) = channel(100);

        for i in 0..*strategies.number_of_windows() {
            // Split urls to smaller batches
            match get_batch_range(urls.len(), *strategies.number_of_windows(), i) {
                Some((start, end)) => {
                    let tx_clone = tx.clone();
                    let opt_clone = options.clone();
                    let sub_urls = Vec::from(&urls[start..end]);

                    tokio::spawn(async move {
                        let scraper = UrlScraper::new(tx_clone, opt_clone);
                        scraper.scrape(&sub_urls).await.unwrap();
                    });
                }
                None => break,
            }
        }

        drop(tx);

        let mut file: Option<File> = None;

        if !strategies.dest_dir().is_empty() {
            let now: DateTime<Utc> = SystemTime::now().into();
            let name = now.timestamp_millis();

            fs::create_dir_all(format!("{}", strategies.dest_dir()))?;
            file = Some(
                fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(format!("{}/{}.txt", strategies.dest_dir(), name))
                    .unwrap(),
            );
        }

        while let Some(data) = rx.recv().await {
            match file.as_mut() {
                Some(f) => {
                    writeln!(f, "{}", data)?;
                }
                None => println!("Url: {}", data),
            }
        }

        Ok(())
    }

    pub async fn run(&self) -> WebDriverResult<()> {
        println!("AA");

        Ok(())
    }
}
