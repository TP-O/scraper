mod scraper;
pub mod util;

use thirtyfour::prelude::{WebDriverError, WebDriverResult};
use tokio::sync::mpsc::channel;

pub use scraper::*;

pub async fn download_images_from_urls(
    urls: &Vec<String>,
    windows: u8,
    filter: ImageFilter,
) -> WebDriverResult<()> {
    if windows == 0 {
        return Err(WebDriverError::CustomError(String::from(
            "Invalid number of windows",
        )));
    }

    let (tx, mut rx) = channel(100);

    for i in 0..windows as usize {
        match util::get_batch_range(urls.len(), windows.into(), i) {
            None => break,
            Some((start, end)) => {
                let tx_clone = tx.clone();
                let filter_clone = filter.clone();
                let sub_urls = Vec::from(&urls[start..end]);

                tokio::spawn(async move {
                    scraper::scrape_images(&tx_clone, &sub_urls, filter_clone)
                        .await
                        .unwrap();

                    drop(tx_clone);
                });
            }
        }
    }

    drop(tx);

    while let Some(data) = rx.recv().await {
        data.save("test/")?;
    }

    Ok(())
}
