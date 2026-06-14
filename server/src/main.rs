#[tokio::main]
async fn main() -> anyhow::Result<()> {
    corgitrack::run().await
}
