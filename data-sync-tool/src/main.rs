use chrono::{Local};
// use chrono::DateTime;
// use chrono::Local;
// mod infrastructure;
// mod application;
// mod domain;
// mod presentation;
// mod services;

use log::{info, trace, warn, error};

use env_logger;

use tungstenite::{connect, Message};
use url::Url;

fn main() {
    env_logger::init();
    let (mut socket, response) =
        connect(Url::parse("ws://121.40.165.18:8800").unwrap()).expect("Failed to connect");

    log::info!("Connected to the server");
    log::info!("Response HTTP code: {}", response.status());
    
    socket
        .write_message(Message::Text("Hello Echo Test WebSocket".into()))
        .unwrap();

    loop {
        let msg = socket.read_message().expect("Error reading message");
        println!("Received: {}", msg);

        // In this example, we'll break the loop after receiving the first message.
        // In a real-world application, you'd probably have a more complex condition.
        // break;
    }
}
