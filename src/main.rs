use manga_sp::*;

#[tokio::main]
async fn main() {
    download_images_from_urls(
        &vec![String::from(
            "https://www.nettruyenme.com/truyen-tranh/dao-hai-tac/chap-1062/912998",
        )],
        1,
    )
    .await
    .unwrap();
}
