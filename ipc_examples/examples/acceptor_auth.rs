//! Example of a server using bespoke authorization
//! ```q
//! q)h:hopen `:unix://4321:homer:j:simpson
//! ```

use kxkdb::ipc::*;
use std::io;
use async_trait::async_trait;

struct TestAuth;
#[async_trait]
impl Auth for TestAuth {
    async fn authorize(&mut self, credential: &str) -> Result<()> {
        if credential.starts_with("homer:") && credential.ends_with(":simpson") {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "authentication failed").into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create authorization object
    let mut test_auth = TestAuth {};
    // Start listenening over UDS at the port 4321 
    if let Ok(mut socket) = QStream::accept_auth(ConnectionMethod::UDS, "", 4321, &mut test_auth).await {
        loop {
                match socket.receive_message().await {
                    Ok((_, message)) => {
                        println!("request: {}", message);
                    }
                    _ => {
                        socket.shutdown().await.unwrap();
                        break;
                    }
                }
            }
    }
    Ok(())
}
