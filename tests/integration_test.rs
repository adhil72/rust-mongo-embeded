use mongo_embedded::MongoEmbedded;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_download_and_start() {
    // Use a specific version. 6.0.4 is relatively recent and stable.
    // 7.0.2 is also good.
    let version = "7.0.2";
    let mongo = MongoEmbedded::new(version).unwrap()
        .set_credentials("test", "test");
    
    // Use a custom port and db path to avoid conflicts
    let port = 12345;
    let temp_dir = std::env::temp_dir().join("mongo_test_db_auth");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    let mongo = mongo.set_port(port).set_db_path(temp_dir.clone());

    println!("Starting MongoDB...");
    let mut process = mongo.start().await.expect("Failed to start MongoDB");
    
    println!("MongoDB started successfully!");
    println!("Connection string: {}", process.connection_string);
    
    // Let it run for a bit
    sleep(Duration::from_secs(5)).await;
    
    println!("Stopping MongoDB...");
    process.kill().expect("Failed to kill MongoDB process");
    println!("MongoDB stopped.");
}

#[tokio::test]
async fn test_socket_bind() {
    let version = "7.0.2";
    let mongo = MongoEmbedded::new(version).unwrap();
    
    // Use a random temporary path for the socket
    let socket_path = std::env::temp_dir().join("test_mongo.sock");
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).unwrap();
    }
    
    let mongo = mongo.set_bind_ip(socket_path.to_str().unwrap());

    // Use a custom db path to avoid conflicts
    let temp_db_dir = std::env::temp_dir().join("mongo_test_db_socket");
    if temp_db_dir.exists() {
        std::fs::remove_dir_all(&temp_db_dir).unwrap();
    }
    let mongo = mongo.set_db_path(temp_db_dir.clone());
    
    println!("Starting MongoDB with socket: {:?}", socket_path);
    let mut process = mongo.start().await.expect("Failed to start MongoDB");
    
    // Wait for startup
    sleep(Duration::from_secs(5)).await;
    
    // Verify socket exists
    assert!(socket_path.exists(), "Socket file should exist");
    
    println!("Verified socket existence. Stopping MongoDB...");
    process.kill().expect("Failed to kill MongoDB process");
    
    // Cleanup
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).unwrap();
    }
}
