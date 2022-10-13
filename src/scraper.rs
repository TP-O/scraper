mod image;

use std::io::{Error, ErrorKind};
use std::process::Command;

use derive_getters::Getters;
use thirtyfour::{prelude::WebDriverResult, DesiredCapabilities, WebDriver};

pub use self::image::*;

const DRIVER_PORT: &str = "9515";
const DISABLE_CORS_EXTENSION: &str = "ext/disable-cors";

static IS_DRIVER_STARTING: bool = false;

#[derive(Getters)]
pub struct ScrapeStrategies {
    number_of_windows: usize,
    dest_dir: String,
}

impl Default for ScrapeStrategies {
    fn default() -> Self {
        Self {
            number_of_windows: 1,
            dest_dir: String::from("download/"),
        }
    }
}

impl ScrapeStrategies {
    pub fn set_number_of_windows(mut self, windows: usize) -> Self {
        if windows > 0 && windows != self.number_of_windows {
            self.number_of_windows = windows;
        }

        self
    }

    pub fn set_destination(mut self, path: String) -> Self {
        self.dest_dir = path;

        self
    }
}

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
