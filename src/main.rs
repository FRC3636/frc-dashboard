#![feature(lookup_host)]

extern crate sixense;
extern crate mio;
extern crate byteorder;

mod message;
mod server;

use std::thread;
use std::time::Duration;
use server::ServerHandle;
use message::{DsMessage, Hand};
use sixense::Sixense;
fn main() {
    let mut server = ServerHandle::new();
    let mut sixense = Sixense::new();
    loop {
        server.tick();
        let data = sixense.all_newest_data();
        server.send_udp(DsMessage::Sixense(data[0], Hand::Left));
        server.send_udp(DsMessage::Sixense(data[1], Hand::Right));
        thread::sleep(Duration::from_millis(10));
        while let Some(msg) = server.recv() {
            println!("{:?}", msg);
        }
        //println!("Connected: {}", server.connected);
    }
}
