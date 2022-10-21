use scraper::cli::CommandLineInterface;

#[tokio::main]
async fn main() {
    let cli = CommandLineInterface::new();

    match cli.run().await {
        Ok(_) => println!("Let's check your download folder!"),
        Err(err) => println!("Error occurs: {:?}", err),
    }
}
