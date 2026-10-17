#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    nyxi_driver::run().await
}
