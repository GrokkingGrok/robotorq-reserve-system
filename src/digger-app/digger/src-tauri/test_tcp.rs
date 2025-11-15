// Quick test to see if basic TCP binding works
use std::net::TcpListener;

fn main() {
    println!("Testing TCP binding on 127.0.0.1:9000...");
    
    match TcpListener::bind("127.0.0.1:9000") {
        Ok(listener) => {
            println!("✅ Successfully bound to 127.0.0.1:9000");
            println!("   Local addr: {:?}", listener.local_addr());
            
            println!("Waiting for connections...");
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        println!("Got connection from: {:?}", s.peer_addr());
                    }
                    Err(e) => {
                        eprintln!("Connection error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to bind: {}", e);
        }
    }
}
