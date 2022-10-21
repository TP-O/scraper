use std::{
    fs::{self, File},
    io::{self, BufRead, Write},
    path::Path,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use clap::Parser;
use thirtyfour::prelude::WebDriverResult;
use tokio::sync::mpsc::channel;

use crate::{scraper::*, util::*};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of crawled urls
    #[arg(short = 'l', long)]
    urls: Vec<String>,

    /// List of paths to files containing crawled urls
    #[arg(short, long)]
    paths: Vec<String>,

    /// Number of workers to scrape data at the same time
    #[arg(short, long)]
    worker: Option<usize>,

    /// Path to output folder
    #[arg(short, long)]
    output: Option<String>,

    /// Scrape URLs from given URL list
    #[arg(short, long)]
    url_scrape: bool,

    /// Download images from given URL list
    #[arg(short, long)]
    image_download: bool,

    /// Min width of downloaded images
    #[arg(long)]
    image_width: Option<usize>,

    /// Min height of downloaded images
    #[arg(long)]
    image_height: Option<usize>,

    /// MIME types of downloaded images
    #[arg(long)]
    image_types: Vec<ImageMimeType>,

    /// Name of HTML tag containing crawled urls
    #[arg(long)]
    url_tags: Vec<UrlTag>,

    /// Format of crawled urls
    #[arg(long)]
    url_regex: Option<String>,
}

pub struct CommandLineInterface {
    //
}

impl CommandLineInterface {
    pub fn new() -> Self {
        Self {}
    }

    async fn download_images(
        &self,
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
        &self,
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
        let args = Args::parse();
        let mut urls = Vec::from(args.urls);
        let mut strategy = ScrapeStrategies::default();

        if Vec::len(&args.paths) > 0 {
            for p in args.paths {
                let path = Path::new(&p);

                if path.is_file() {
                    let f = File::open(path.to_str().unwrap())?;
                    let reader = io::BufReader::new(f).lines();

                    for line in reader {
                        if let Ok(line) = line {
                            urls.push(line);
                        }
                    }
                } else if path.is_dir() {
                    for entry in path.read_dir().unwrap() {
                        if let Ok(entry) = entry {
                            let f = File::open(entry.path().to_str().unwrap())?;
                            let reader = io::BufReader::new(f).lines();

                            for line in reader {
                                if let Ok(line) = line {
                                    urls.push(line);
                                }
                            }
                        }
                    }
                }
            }
        }

        if args.worker.is_some() {
            strategy = strategy.set_number_of_windows(args.worker.unwrap());
        }

        if args.output.is_some() {
            strategy = strategy.set_destination(args.output.unwrap());
        }

        if args.url_scrape {
            let mut option = ScrapeUrlOptions::default();

            if Vec::len(&args.url_tags) > 0 {
                option = option.remove_tag(UrlTag::A);

                for tag in args.url_tags {
                    option = option.add_tag(tag);
                }
            }

            if args.url_regex.is_some() {
                option = option.set_regex(&args.url_regex.unwrap());
            }

            self.scrape_urls(&urls, strategy, option).await?;
        } else if args.image_download {
            let mut option = ScrapeImageOptions::default();

            if Vec::len(&args.image_types) > 0 {
                option = option.remove_mime_type(ImageMimeType::Jpeg);

                for mime_type in args.image_types {
                    option = option.add_mime_type(mime_type);
                }
            }

            if args.image_width.is_some() {
                option = option.set_min_width(args.image_width.unwrap());
            }

            if args.image_height.is_some() {
                option = option.set_min_height(args.image_height.unwrap());
            }

            self.download_images(&urls, strategy, option).await?;
        }

        Ok(())
    }
}
