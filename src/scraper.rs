mod image;

use std::io::{Error, ErrorKind};
use std::process::Command;

use thirtyfour::{prelude::WebDriverResult, By, DesiredCapabilities, WebDriver};
use tokio::sync::mpsc::Sender;

use self::image::ImageData;
pub use self::image::{ImageFilter, ImageMimeType};

const DRIVER_PORT: &str = "9515";
const DISABLE_CORS_EXTENSION: &str = "ext/disable-cors";

static IS_DRIVER_STARTING: bool = false;

fn start_driver() -> Result<String, Error> {
    if IS_DRIVER_STARTING {
        // Server URL that the driver is listening
        return Ok(format!("http:/localhost:{}", DRIVER_PORT));
    }

    let mut cmd: Command;

    if cfg!(target_os = "linux") {
        cmd = Command::new("driver/linux-chromedriver");
    } else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "This feature is not yet available in your operating system",
        ));
    }

    cmd.arg(format!("--port={DRIVER_PORT}")).spawn()?;

    Ok(format!("http:/localhost:{}", DRIVER_PORT))
}

async fn new_driver() -> WebDriverResult<WebDriver> {
    let mut caps = DesiredCapabilities::chrome();
    caps.add_chrome_arg(format!("--load-extension={}", DISABLE_CORS_EXTENSION).as_str())?;
    let driver = WebDriver::new(start_driver()?.as_str(), caps).await?;

    Ok(driver)
}

pub async fn scrape_images(
    tx: &Sender<ImageData>,
    urls: &Vec<String>,
    filter: ImageFilter,
) -> WebDriverResult<()> {
    let driver = new_driver().await?;

    for url in urls {
        driver.goto(url).await?;

        let title = driver.title().await?;
        let img_tags = driver.find_all(By::Tag("img")).await?;

        for img in img_tags {
            if !image::is_valid_size(&img, *filter.min_width(), *filter.min_height()).await {
                continue;
            }

            match img.attr("src").await? {
                Some(src) => match image::read_data_url(&driver, &src, filter.mime_types()).await {
                    Some((mime_type, data)) => {
                        tx.send(ImageData::new(title.clone(), mime_type, data))
                            .await
                            .ok();
                    }
                    None => continue,
                },
                None => continue,
            };
        }
    }

    driver.close_window().await?;

    Ok(())
}
