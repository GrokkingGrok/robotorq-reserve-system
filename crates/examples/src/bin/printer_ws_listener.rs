use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

use commons::util::error::ExampleError;

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    let ws_url =
        std::env::var("PRINTER_WS").unwrap_or_else(|_| "ws://192.168.12.182:9999/".to_string());
    let _origin =
        std::env::var("PRINTER_ORIGIN").unwrap_or_else(|_| "http://192.168.12.182".to_string());

    // Prefer the URL-based helper so the client constructs a valid WebSocket handshake.
    // Some servers will respond incorrectly when given a pre-built request; using the
    // helper avoids `InvalidHeader("sec-websocket-key")` issues observed in the wild.
    let conn = tokio_tungstenite::connect_async(&ws_url).await;
    let (ws_stream, resp) = match conn {
        Ok(v) => v,
        Err(e) => return Err(ExampleError::Other(format!("connect error: {}", e))),
    };
    println!("Connected to {} (HTTP {})", ws_url, resp.status());

    let (_write, mut read) = ws_stream.split();

    loop {
        match read.next().await {
            Some(Ok(m)) => match m {
                Message::Text(t) => println!("TEXT: {}", t),
                Message::Binary(b) => println!("BINARY ({} bytes)", b.len()),
                Message::Ping(p) => println!("PING: {} bytes", p.len()),
                Message::Pong(p) => println!("PONG: {} bytes", p.len()),
                Message::Close(Some(cf)) => {
                    println!("CLOSE: code={} reason={}", cf.code, cf.reason);
                    break;
                }
                Message::Close(None) => {
                    println!("CLOSE");
                    break;
                }
                other => println!("OTHER MESSAGE: {:?}", other),
            },
            Some(Err(e)) => return Err(ExampleError::Other(format!("read error: {}", e))),
            None => break,
        }
    }

    Ok(())
}
