use image::{
    imageops::{self, FilterType},
    ImageBuffer, ImageError, Rgb, RgbImage,
};

const PICTURES_PER_PAGE: usize = 2;

pub fn create_from(
    image_paths: &Vec<String>,
    width: u32,
    from_left: bool,
) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, ImageError> {
    let mut height = 0;
    let mut pictures = vec![];
    // Max picture height for each row
    let mut max_picture_height = 0;
    // Y axis of each row
    let mut picture_y = vec![0];
    // Picture width when the number of pictures can fill the row
    let default_picture_width = width / PICTURES_PER_PAGE as u32;
    // Picture width when the remaining number of pictures cannot fill the row
    let fallback_picture_width = match image_paths.len() % PICTURES_PER_PAGE != 0 {
        true => Some(width / (image_paths.len() % PICTURES_PER_PAGE) as u32),
        false => None,
    };

    for (i, image_path) in image_paths.iter().enumerate() {
        // Check if use default width or fallback width for image processing
        let picture_width =
            if fallback_picture_width.is_some() && image_paths.len() - i < PICTURES_PER_PAGE {
                fallback_picture_width.unwrap()
            } else {
                default_picture_width
            };
        let loaded_image = image::open(image_path)?.into_rgb8();
        let resized_image = imageops::resize(
            &loaded_image,
            picture_width,
            picture_width * loaded_image.height() / loaded_image.width(),
            FilterType::Triangle,
        );

        if max_picture_height < resized_image.height() {
            max_picture_height = resized_image.height();
        }

        // Height of the page is calculated as sum of all maximum picture heights in each row
        if (i + 1) % PICTURES_PER_PAGE == 0 || i == image_paths.len() - 1 {
            height += max_picture_height;
            picture_y.push(height);
            max_picture_height = 0;
        }

        pictures.push(resized_image);
    }

    let mut created_page = RgbImage::new(width, height);

    for (i, picture) in pictures.iter().enumerate() {
        // Check if use default width or fallback width for image processing
        let picture_width =
            if fallback_picture_width.is_some() && image_paths.len() - i < PICTURES_PER_PAGE {
                fallback_picture_width.unwrap()
            } else {
                default_picture_width
            };
        // X axis of the current picture in the page
        let x = if from_left {
            if fallback_picture_width.is_some() && image_paths.len() - i < PICTURES_PER_PAGE {
                (i % (image_paths.len() - i)) * picture_width as usize
            } else {
                (i % PICTURES_PER_PAGE) * picture_width as usize
            }
        } else {
            if fallback_picture_width.is_some() && image_paths.len() - i < PICTURES_PER_PAGE {
                ((i + (image_paths.len() - i) - 1) % PICTURES_PER_PAGE) * picture_width as usize
            } else {
                ((i + PICTURES_PER_PAGE - 1) % PICTURES_PER_PAGE) * picture_width as usize
            }
        };

        imageops::overlay(
            &mut created_page,
            picture,
            x as i64,
            *picture_y.get(i / PICTURES_PER_PAGE).unwrap() as i64,
        );
    }

    Ok(created_page)
}
