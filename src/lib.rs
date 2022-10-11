mod page;
mod scraper;

use std::fs;

use thirtyfour::prelude::{WebDriverError, WebDriverResult};
use tokio::sync::mpsc::channel;

const SUPPORTED_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

pub fn convert_webtoon_to_manga(dir_path: &str, images_per_page: u8, from_feft: bool) {
    let mut converted_image_paths = Vec::new();
    let mut image_paths: Vec<String> = fs::read_dir(dir_path)
        .unwrap()
        .filter(|path_result| {
            path_result.as_ref().unwrap().path().extension().is_some()
                && SUPPORTED_EXTENSIONS.contains(
                    &path_result
                        .as_ref()
                        .unwrap()
                        .path()
                        .extension()
                        .unwrap()
                        .to_ascii_lowercase()
                        .to_str()
                        .unwrap(),
                )
        })
        .map(|path_result| {
            path_result
                .unwrap()
                .path()
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    image_paths.sort();

    for path in image_paths {
        converted_image_paths.push(path);

        if converted_image_paths.len() == images_per_page as usize {
            let page = page::create_from(&converted_image_paths, 1000, from_feft).unwrap();
            page.save("test2.jpg").unwrap();

            converted_image_paths.clear();
        }
    }
}

pub async fn download_images_from_urls(urls: &Vec<String>, windows: u8) -> WebDriverResult<()> {
    if windows == 0 {
        return Err(WebDriverError::CustomError(String::from(
            "Invalid number of windows",
        )));
    }

    let (sender, mut receiver) = channel(100);

    for i in 0..windows as usize {
        let sender_clone = sender.clone();
        let sub_urls = Vec::from(&urls[i..1]);

        tokio::spawn(async move {
            match scraper::scrape_image_data_urls(sender_clone, &sub_urls).await {
                Ok(_) => 1,
                Err(_) => 2,
            };
        });
    }

    let mut c = 0;

    while let Some(data) = receiver.recv().await {
        data.save("test/", c.to_string().as_str());
        c += 1;

        if c == urls.len() {
            receiver.close();
        }
    }

    Ok(())
}
