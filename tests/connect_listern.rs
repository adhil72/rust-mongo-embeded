use mongo_embedded::MongoEmbedded;
use mongodb::bson::doc;


#[tokio::test]
async fn test_connect_and_listen_unix_with_user() {
    let version = "7.0.2";
    let temp_db_dir = std::env::temp_dir().join("mongo_test_connect_listen_unix");
    if temp_db_dir.exists() {
        std::fs::remove_dir_all(&temp_db_dir).unwrap();
    }
    
    let socket_path = std::env::temp_dir().join("mongo_test_connect.sock");
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).unwrap();
    }

    let mongo = MongoEmbedded::new(version).unwrap()
        .set_credentials("admin", "password123")
        .set_bind_ip(socket_path.to_str().unwrap())
        .set_db_path(temp_db_dir.clone());

    println!("Starting MongoDB (Unix Socket)...");
    let mut process = mongo.start().await.expect("Failed to start MongoDB");

    println!("MongoDB started successfully!");
    println!("Connection URI: {}", process.connection_string);

    // Verify connection works with the generated URI
    let client_options = mongodb::options::ClientOptions::parse(&process.connection_string).await.expect("Failed to parse URI");
    let client = mongodb::Client::with_options(client_options).expect("Failed to create client");
    
    println!("Verifying connection with ping...");
    // Verify we can run a command (auth check)
    let db = client.database("admin");
    let ping = db.run_command(doc! { "ping": 1 }, None).await;
    
    match ping {
        Ok(_) => println!("Successfully pinged database with authenticated user!"),
        Err(e) => panic!("Failed to ping database: {}", e),
    }

    println!("MongoDB is running (Unix Socket). Press Ctrl+C to stop...");
    tokio::signal::ctrl_c().await.expect("failed to listen for event");

    println!("Stopping MongoDB...");
    process.kill().expect("Failed to kill MongoDB process");
    
    // Cleanup
    if temp_db_dir.exists() {
        std::fs::remove_dir_all(&temp_db_dir).unwrap();
    }
}
