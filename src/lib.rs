mod scraper;
pub mod util;

use thirtyfour::prelude::WebDriverResult;
use tokio::sync::mpsc::channel;

pub use scraper::*;

pub async fn scrape_images(
    urls: &Vec<String>,
    strategies: ScrapeStrategies,
    scrape_opts: ScrapeImageOptions,
) -> WebDriverResult<()> {
    let (tx, mut rx) = channel(100);

    for i in 0..*strategies.number_of_windows() {
        // Split urls to smaller batches
        match util::get_batch_range(urls.len(), *strategies.number_of_windows(), i) {
            Some((start, end)) => {
                let tx_clone = tx.clone();
                let opt_clone = scrape_opts.clone();
                let sub_urls = Vec::from(&urls[start..end]);

                tokio::spawn(async move {
                    scraper::scrape_images(&tx_clone, &sub_urls, opt_clone)
                        .await
                        .unwrap();

                    drop(tx_clone);
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

pub async fn scrape_urls(
    urls: &Vec<&'static str>,
    strategies: ScrapeStrategies,
    scrape_opts: ScrapeUrlOptions,
) -> WebDriverResult<()> {
    let (tx, mut rx) = channel(100);

    for i in 0..*strategies.number_of_windows() {
        // Split urls to smaller batches
        match util::get_batch_range(urls.len(), *strategies.number_of_windows(), i) {
            Some((start, end)) => {
                let tx_clone = tx.clone();
                let opt_clone = scrape_opts.clone();
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

    while let Some(data) = rx.recv().await {
        println!("url: {}", data);
    }

    Ok(())
}
