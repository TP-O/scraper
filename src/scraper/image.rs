use std::{
    fs,
    io::{Error, Result},
};

use base64::decode;

pub struct ImageData {
    pub title: String,
    mime_type: String,
    encoded_content: String,
}

impl ImageData {
    pub fn new(title: String, mime_type: String, encoded_content: String) -> ImageData {
        ImageData {
            title,
            mime_type,
            encoded_content,
        }
    }

    pub fn save(&self, path: &str, name: &str) -> Result<()> {
        match decode(self.encoded_content.clone()) {
            Ok(content) => {
                let slash_index = self.mime_type.find("/").unwrap_or(0);
                let extension = self
                    .mime_type
                    .chars()
                    .skip(slash_index + 1)
                    .collect::<String>();

                fs::create_dir_all(path)?;
                fs::write(format!("{}{}.{}", path, name, extension), content)?;

                Ok(())
            }
            Err(_) => Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "Base64 decode failed!",
            )),
        }
    }
}
