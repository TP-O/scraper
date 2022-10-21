use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::Path,
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use clap::Parser;
use tokio::sync::mpsc::channel;

use crate::{error::ScrapeResult, scraper::*, util::*};

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
        strategy: ScrapeStrategy,
        filter: ScrapeImageFilter,
    ) -> ScrapeResult<()> {
        let (tx, mut rx) = channel(100);

        for i in 0..*strategy.number_of_windows() {
            // Split urls to smaller batches
            match get_batch_range(urls.len(), *strategy.number_of_windows(), i) {
                Some((start, end)) => {
                    let tx_clone = tx.clone();
                    let filter_clone = filter.clone();
                    let sub_urls = Vec::from(&urls[start..end]);

                    tokio::spawn(async move {
                        let mut scraper = ImageScraper::new(tx_clone, filter_clone);
                        scraper.scrape(&sub_urls).await.unwrap();
                    });
                }
                None => break,
            }
        }

        drop(tx);

        while let Some(data) = rx.recv().await {
            data.save(strategy.dest_dir()).unwrap();
        }

        Ok(())
    }

    async fn scrape_urls(
        &self,
        urls: &Vec<String>,
        strategy: ScrapeStrategy,
        filter: ScrapeUrlFilter,
    ) -> ScrapeResult<()> {
        let (tx, mut rx) = channel(100);

        for i in 0..*strategy.number_of_windows() {
            // Split urls to smaller batches
            match get_batch_range(urls.len(), *strategy.number_of_windows(), i) {
                Some((start, end)) => {
                    let tx_clone = tx.clone();
                    let filter_clone = filter.clone();
                    let sub_urls = Vec::from(&urls[start..end]);

                    tokio::spawn(async move {
                        let mut scraper = UrlScraper::new(tx_clone, filter_clone);
                        scraper.scrape(&sub_urls).await.unwrap();
                    });
                }
                None => break,
            }
        }

        drop(tx);

        let mut file: Option<File> = None;
        let now: DateTime<Utc> = SystemTime::now().into();
        let name = now.timestamp_millis();

        if !strategy.dest_dir().is_empty() {
            fs::create_dir_all(format!("{}", strategy.dest_dir())).unwrap();
            file = Some(
                fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(format!("{}/{}.txt", strategy.dest_dir(), name))
                    .unwrap(),
            );
        }

        while let Some(data) = rx.recv().await {
            match file.as_mut() {
                Some(f) => {
                    if writeln!(f, "{}", data).is_err() {
                        println!(
                            "Failed to  write url in file: {}/{}.txt",
                            strategy.dest_dir(),
                            name,
                        );
                    }
                }
                None => println!("Url: {}", data),
            }
        }

        Ok(())
    }

    fn read_urls_from_paths(&self, urls: &mut Vec<String>, paths: &Vec<String>) {
        for p in paths {
            let path = Path::new(&p);

            if path.is_file() {
                let f = File::open(path.to_str().unwrap())
                    .expect(&format!("Unable to open file: {}", p));
                let reader = BufReader::new(f).lines();

                for line in reader {
                    if let Ok(line) = line {
                        urls.push(line);
                    }
                }
            } else if path.is_dir() {
                for entry in path.read_dir().unwrap() {
                    if let Ok(entry) = entry {
                        let f = File::open(entry.path().to_str().unwrap()).expect(&format!(
                            "Unable to open file: {}",
                            entry.file_name().to_str().unwrap_or("unknown :D")
                        ));
                        let reader = BufReader::new(f).lines();

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

    pub async fn run(&self) -> ScrapeResult<()> {
        let args = Args::parse();
        let mut urls = Vec::from(args.urls);
        let mut strategy = ScrapeStrategy::default();

        self.read_urls_from_paths(&mut urls, &args.paths);

        if args.worker.is_some() {
            strategy.set_number_of_windows(args.worker.unwrap());
        }

        if args.output.is_some() {
            strategy.set_destination(args.output.unwrap());
        }

        if args.url_scrape {
            let mut filter = ScrapeUrlFilter::default();

            if Vec::len(&args.url_tags) > 0 {
                filter.replace_tags(args.url_tags);
            }

            if args.url_regex.is_some() {
                filter.set_regex(args.url_regex.unwrap());
            }

            self.scrape_urls(&urls, strategy, filter).await?;
        } else if args.image_download {
            let mut filter = ScrapeImageFilter::default();

            if Vec::len(&args.image_types) > 0 {
                filter.replace_mime_types(args.image_types);
            }

            if args.image_width.is_some() {
                filter.set_min_width(args.image_width.unwrap());
            }

            if args.image_height.is_some() {
                filter.set_min_height(args.image_height.unwrap());
            }

            self.download_images(&urls, strategy, filter).await?;
        }

        Ok(())
    }
}
