use thirtyfour::prelude::WebDriverError;

pub type ScrapeResult<T> = Result<T, ScrapeError>;

#[derive(Debug)]
pub enum ScrapeError {
    WebDriverError(WebDriverError),
    IncompatibleError(String),
    CmdError(String),
}
