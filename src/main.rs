use manga_sp::*;

#[tokio::main]
async fn main() {
    scrape_images(
        &vec![
            "https://www.nettruyenme.com/truyen-tranh/dao-hai-tac/chap-1062/912998",
            "https://blogtruyen.vn/c732416/mung-papa-ve-chap-2",
            "https://blogtruyen.vn/c739424/watashi-ga-15-sai-de-wa-nakunatte-mo-chap-1",
            "https://blogtruyen.vn/c704542/yancha-gal-no-anjou-san-series-chap-125-anjou-san-muon-duoc-o-gan-som-hon",
            "https://blogtruyen.vn/c345998/yancha-gal-no-anjou-san-series-chuong-1",
        ],
        ScrapeStrategies::default().set_number_of_windows(2),
        ScrapeImageOptions::default().add_mime_type(ImageMimeType::Png),
    )
    .await
    .unwrap();

    // scrape_urls(
    //     &vec!["https://www.w3schools.com/tags/tag_figure.asp"],
    //     ScrapeStrategies::default(),
    //     ScrapeUrlOptions::default(),
    // )
    // .await
    // .unwrap();
}
