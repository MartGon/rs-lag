
#[macro_use]
extern crate clap;
use clap::App;

use std::mem::swap;
use std::net::{ToSocketAddrs, UdpSocket};
use std::collections::BTreeMap;

mod network;
use crate::network::Packet;

// TODO: Uses threads for sending/recv

fn now_nanos() -> u128{
    let time = std::time::SystemTime::now();
    let now = time.duration_since(std::time::UNIX_EPOCH).expect("Time is wrong");
    now.as_nanos()
}

fn millis_to_nanos(millis : u128) -> u128{
    millis * 1_000_000
}

fn main() {

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let listen_port = matches.value_of("listen-port").unwrap();
    let listen_port = listen_port.parse::<u16>().unwrap();

    let connect_port = matches.value_of("connect-port").unwrap();
    let connect_port = connect_port.parse::<u16>().unwrap();

    let lag = matches.value_of("lag").unwrap();
    let lag = lag.parse::<u128>().unwrap();
    
    println!("The listen port is {}", listen_port);
    println!("Datagrams will be redirected to port {}", connect_port);

    let mut packet_queue : BTreeMap<u128, Packet> = BTreeMap::new();

    let server_address = ToSocketAddrs::to_socket_addrs(&format!("127.0.0.1:{}", connect_port)[..]).expect("Invalid connection port").next().unwrap();
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", listen_port)).expect(&format!("Could not bind to UDP port {}", listen_port)[..]);

    // Get client address from first packet. Wait for it
    let mut buf = [0; 255];
    let (_, client_src) = socket.peek_from(&mut buf).expect("Error while peeking on socket");
    println!("Client connected from {}", client_src);    

    // No longer block for packets
    const WAIT_TIME : std::time::Duration = std::time::Duration::from_nanos(1);
    socket.set_read_timeout(Some(WAIT_TIME)).expect("Set read timeout failed");
    loop
    {
        // Recv packets
        while let Ok((size, _)) = socket.peek_from(&mut buf)
        {
            let mut buffer : Vec<u8> = vec![0; size];
            let (_, src)  = socket.recv_from(&mut buffer).expect("Error while recv from socket.");

            let from_server = src.port() == connect_port;
            let dest = if from_server {client_src} else {server_address};
            println!("Packet recv from {} of size {} to be sent to {}", src, size, dest);

            let now = now_nanos();
            let network_delay: u128 = millis_to_nanos(lag);
            let sent_date = now + network_delay;
            println!("Date to send {}", sent_date);

            let packet = Packet::new(buffer, dest, sent_date);

            packet_queue.insert(sent_date, packet);
        }
        
        let now = now_nanos();
        let mut to_send = packet_queue.split_off(&now);
        swap(&mut to_send, &mut packet_queue);
        for (_, packet) in to_send
        {
            println!("Packet sent to {} of size {} with delay: {} ms", packet.dest, packet.size(), 0);
            socket.send_to(&packet.buffer, packet.dest).expect("Error while sending");
        }
    }
}
