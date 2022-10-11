mod image;

use std::io::{Error, ErrorKind};
use std::process::Command;

use dataurl::DataUrl;
use thirtyfour::{prelude::WebDriverResult, By, DesiredCapabilities, WebDriver};
use tokio::sync::mpsc::Sender;

use self::image::ImageData;

const DRIVER_PORT: &str = "9515";

const DISABLE_CORS_EXTENSION: &str = "ext/disable-cors";

struct DataUrlParse {
    mime_type: String,
    data: String,
}

fn start_driver() -> Result<String, Error> {
    let mut cmd: Command;

    if cfg!(target_os = "linux") {
        cmd = Command::new("driver/linux-chromedriver")
    } else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "This feature is not yet available in your operating system",
        ));
    }

    cmd.arg(format!("--port={DRIVER_PORT}")).output()?;

    // Server URL that the driver is listening
    Ok(format!("http:/localhost:{}", DRIVER_PORT))
}

async fn new_driver() -> WebDriverResult<WebDriver> {
    let mut caps = DesiredCapabilities::chrome();
    caps.add_chrome_arg(format!("--load-extension={}", DISABLE_CORS_EXTENSION).as_str())?;
    let driver = WebDriver::new(start_driver()?.as_str(), caps).await?;

    Ok(driver)
}

async fn read_data_url(driver: &WebDriver, src: &String) -> WebDriverResult<String> {
    let data_url = driver
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
        .await?
        .convert()?;

    Ok(data_url)
}

fn get_data_part(data_url: String) -> Option<DataUrlParse> {
    match DataUrl::parse(data_url.as_str()) {
        Err(_) => None,
        Ok(parsed) => {
            // Format of data url: data:[<mediatype>][;base64],<data>
            if parsed.get_media_type() == "image/jpeg" || parsed.get_media_type() == "image/png" {
                let comma_index = data_url.find(",").unwrap_or(0);
                // Skip comma and all characters before it
                let data = data_url.chars().skip(comma_index + 1).collect::<String>();

                Some(DataUrlParse {
                    mime_type: parsed.get_media_type().to_owned(),
                    data,
                })
            } else {
                None
            }
        }
    }
}

pub async fn scrape_image_data_urls(
    sender: Sender<ImageData>,
    urls: &Vec<String>,
) -> WebDriverResult<()> {
    let driver = new_driver().await?;

    for url in urls {
        driver.goto(url).await?;

        let title = driver.title().await?;
        let img_tags = driver.find_all(By::Tag("img")).await?;

        for img in img_tags {
            match img.attr("src").await? {
                None => continue,
                Some(src) => {
                    let data_url: String = read_data_url(&driver, &src).await?;

                    match get_data_part(data_url) {
                        None => continue,
                        Some(parsed) => {
                            if sender
                                .send(ImageData::new(title.clone(), parsed.mime_type, parsed.data))
                                .await
                                .is_err()
                            {
                                return Err(thirtyfour::prelude::WebDriverError::CustomError(
                                    format!("Download failed: {}", url),
                                ));
                            }
                        }
                    }
                }
            };
        }
    }

    Ok(())
}
