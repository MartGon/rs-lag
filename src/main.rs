
#[macro_use]
extern crate clap;
use clap::App;
use rand::prelude::{Distribution, ThreadRng};

use std::mem::swap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::collections::{BTreeMap, HashMap};

use rand::distributions::{Uniform};
use std::convert::TryFrom;

mod network;
use crate::network::Packet;

fn now_nanos() -> u128{
    let time = std::time::SystemTime::now();
    let now = time.duration_since(std::time::UNIX_EPOCH).expect("Time is wrong");
    now.as_nanos()
}

fn millis_to_nanos(millis : u128) -> u128{
    millis * 1_000_000
}

fn i128_millis_to_nanos(millis : i128) -> i128
{
    millis * 1_000_000
}

fn gen_sent_date(lag: u128, dist : Uniform::<i128>, rng : &mut ThreadRng) -> u128
{
    let now = now_nanos();
    let sent_date = now + gen_delay(lag, dist, rng);
    return sent_date;
}

fn gen_delay(lag: u128, dist : Uniform::<i128>, rng : &mut ThreadRng) -> u128
{
    let network_delay: u128 = millis_to_nanos(lag);
    let jitter = i128_millis_to_nanos(dist.sample(rng));
    let delay = u128::try_from(std::cmp::max(network_delay as i128 + jitter, 0)).expect("Conversion error");
    return delay;
}
// TODO: Uses threads for sending/recv

fn main() {

    // Arg parsing
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let listen_port = matches.value_of("listen-port").unwrap();
    let listen_port = listen_port.parse::<u16>().unwrap();

    let connect_port = matches.value_of("connect-port").unwrap();
    let connect_port = connect_port.parse::<u16>().unwrap();

    println!("The listen port is {}", listen_port);
    println!("Datagrams will be redirected to port {}", connect_port);

    let lag = matches.value_of("lag").unwrap();
    let lag = std::cmp::max((lag.parse::<u128>().unwrap()) / 2, 0);
    println!("A packet will take {} ms to traverse the network", lag);

    let jitter = matches.value_of("jitter").unwrap();
    let jitter : i128 = jitter.parse::<i128>().unwrap();
    println!("The RTT will have {} ms variance", jitter);

    let duplication_chance = matches.value_of("duplication").unwrap();
    let duplication_chance : f32 = duplication_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Packet Duplication chance: {}", duplication_chance);

    let unorder_chance = matches.value_of("unorder").unwrap();
    let unorder_chance : f32 = unorder_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Packet Unorder chance: {}", unorder_chance);

    // Setup
    let mut packet_queue : BTreeMap<u128, Packet> = BTreeMap::new();
    let mut unordered_packets = HashMap::<SocketAddr, Packet>::new();

    let server_address = ToSocketAddrs::to_socket_addrs(&format!("127.0.0.1:{}", connect_port)[..]).expect("Invalid connection port").next().unwrap();
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", listen_port)).expect(&format!("Could not bind to UDP port {}", listen_port)[..]);

    let jitter_dist = Uniform::<i128>::from(-jitter..(jitter + 1));
    let duplication_dist = Uniform::<f32>::from(0f32..100f32);
    let unorder_dist = Uniform::<f32>::from(0f32..100f32);
    let mut rng = rand::thread_rng();

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
            let sent_date = gen_sent_date(lag, jitter_dist, &mut rng);
            //println!("Packet recv from {} of size {} to be sent to {}", src, size, dest);

            let roll = duplication_dist.sample(&mut rng);
            if roll < duplication_chance
            {
                let sent_date = gen_sent_date(lag, jitter_dist, &mut rng);
                let duplicate = Packet::new(buffer.clone(), dest, sent_date);
                packet_queue.insert(sent_date, duplicate);
            }

            let packet = Packet::new(buffer, dest, sent_date);

            let roll = unorder_dist.sample(&mut rng);
            if roll < unorder_chance && !unordered_packets.contains_key(&packet.dest)
            {
                //println!("Packet is gonna arrive after the next");
                unordered_packets.insert(packet.dest, packet);
            }
            else if let Some((_, prev_packet)) = unordered_packets.remove_entry(&packet.dest)
            {
                //println!("Packet queued after the next");
                let sent_date = packet.sent_date + gen_delay(lag, jitter_dist, &mut rng);
                packet_queue.insert(packet.sent_date, prev_packet);
                packet_queue.insert(sent_date, packet);
            }
            else
            {
                packet_queue.insert(sent_date, packet);
            }
        }
        
        let now = now_nanos();
        let mut to_send = packet_queue.split_off(&now);
        swap(&mut to_send, &mut packet_queue);
        for (_, packet) in to_send
        {
            //println!("Packet sent to {} of size {} with delay: {} ms", packet.dest, packet.size(), 0);
            socket.send_to(&packet.buffer, packet.dest).expect("Error while sending");
        }
    }
}
