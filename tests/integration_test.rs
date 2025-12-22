use mongo_embedded::MongoEmbedded;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_download_and_start() {
    // Use a specific version. 6.0.4 is relatively recent and stable.
    // 7.0.2 is also good.
    let version = "7.0.2";
    let mongo = MongoEmbedded::new(version).unwrap();
    
    // Use a custom port to avoid conflicts
    let port = 12345;
    let mongo = mongo.set_port(port);

    println!("Starting MongoDB...");
    let mut process = mongo.start().await.expect("Failed to start MongoDB");
    
    println!("MongoDB started successfully!");
    
    // Let it run for a bit
    sleep(Duration::from_secs(5)).await;
    
    println!("Stopping MongoDB...");
    process.kill().expect("Failed to kill MongoDB process");
    println!("MongoDB stopped.");
}
