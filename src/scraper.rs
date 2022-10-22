mod image;
mod url;

use std::process::Command;

use async_trait::async_trait;
use derive_getters::Getters;
use thirtyfour::{DesiredCapabilities, WebDriver};

use crate::error::*;

pub use self::image::*;
pub use self::url::*;

const DRIVER_PORT: &str = "9515";
const DISABLE_CORS_EXTENSION: &str = "ext/disable-cors";

static IS_DRIVER_STARTING: bool = false;

#[async_trait]
pub trait Scrape {
    async fn scrape(&mut self, urls: &Vec<String>) -> ScrapeResult<()>;
}

#[derive(Getters, Clone)]
pub struct ScrapeStrategy {
    number_of_windows: usize,
    dest_dir: String,
}

impl Default for ScrapeStrategy {
    fn default() -> Self {
        Self {
            number_of_windows: 1,
            dest_dir: String::from("download/"),
        }
    }
}

impl ScrapeStrategy {
    pub fn set_number_of_windows(&mut self, windows: usize) -> &mut Self {
        if windows > 0 && windows != self.number_of_windows {
            self.number_of_windows = windows;
        }

        self
    }

    pub fn set_destination(&mut self, path: String) -> &mut Self {
        self.dest_dir = path;

        self
    }
}

fn start_driver() -> ScrapeResult<String> {
    if IS_DRIVER_STARTING {
        // Server URL that the driver is listening
        return Ok(format!("http:/localhost:{}", DRIVER_PORT));
    }

    let mut cmd: Command;

    if cfg!(target_os = "linux") {
        cmd = Command::new("driver/linux-chromedriver");
    } else if cfg!(target_os = "windows") {
        cmd = Command::new("driver/win32-chromedriver");
    } else if cfg!(target_os = "macos") {
        cmd = Command::new("driver/mac_arm64-chromedriver");
    } else {
        return Err(ScrapeError::IncompatibleError(String::from(
            "This feature is not yet available in your operating system",
        )));
    }

    if cmd.arg(format!("--port={DRIVER_PORT}")).spawn().is_err() {
        return Err(ScrapeError::CmdError(String::from(
            "Unable to start driver",
        )));
    }

    Ok(format!("http:/localhost:{}", DRIVER_PORT))
}

async fn new_driver() -> ScrapeResult<WebDriver> {
    let mut caps = DesiredCapabilities::chrome();
    caps.add_chrome_arg(format!("--load-extension={}", DISABLE_CORS_EXTENSION).as_str())
        .unwrap();

    match WebDriver::new(start_driver()?.as_str(), caps).await {
        Ok(driver) => Ok(driver),
        Err(err) => Err(ScrapeError::WebDriverError(err)),
    }
}
