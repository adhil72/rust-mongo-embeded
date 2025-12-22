# mongo-embedded

A Rust library that simplifies using MongoDB for local testing and development by automatically downloading, extracting, and running a MongoDB Community Edition binary.

It handles:
- **OS/Arch Detection**: Automatically selects the correct binary for Linux, macOS, and Windows.
- **Downloading**: Fetches the binary from the official MongoDB download center.
- **Extraction**: Unpacks `.tgz` or `.zip` archives.
- **Execution**: Starts the `mongod` process on a specified port.

## Usage

### Installation

Run the following command in your project directory:

```bash
cargo add mongo-embedded
```

Or add the following to your `Cargo.toml`:

```toml
[dependencies]
mongo-embedded = "0.1.0"
tokio = { version = "1.0", features = ["full"] } # Required for the example below
```

### Example

```rust
use mongo_embedded::MongoEmbedded;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // fastdl.mongodb.org version
    let version = "7.0.2"; 
    
    // Initialize
    let mongo = MongoEmbedded::new(version).unwrap()
        .set_port(12345); // Optional, default is 27017

    // Start
    println!("Starting MongoDB...");
    let mut process = mongo.start().await.expect("Failed to start MongoDB");
    
    // Your DB operations here...
    // MongoDB is running at mongodb://127.0.0.1:12345/

    sleep(Duration::from_secs(5)).await;
    
    // Stop
    process.kill().expect("Failed to kill MongoDB process");
}
```

## Configuration

The library uses the `directories` crate to find suitable locations for:
- **Cache**: Stores downloaded archives (e.g., `~/.cache/mongo-embedded` on Linux).
- **Data**: Stores the database files (e.g., `~/.local/share/mongo-embedded` on Linux).

## License

MIT
