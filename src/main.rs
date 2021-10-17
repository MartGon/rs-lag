
#[macro_use]
extern crate clap;
use clap::App;
use rand::prelude::{Distribution, ThreadRng};

use std::mem::swap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Condvar, Mutex};

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

    let loss_chance = matches.value_of("loss").unwrap();
    let loss_chance : f32 = loss_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Packet Loss chance: {}", loss_chance);

    // Setup
    let packet_queue_recv = Arc::new((Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new()));
    let packet_queue_send = Arc::clone(&packet_queue_recv);

    let server_address = ToSocketAddrs::to_socket_addrs(&format!("127.0.0.1:{}", connect_port)[..]).expect("Invalid connection port").next().unwrap();
    let socket_recv = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", listen_port)).expect(&format!("Could not bind to UDP port {}", listen_port)[..]));
    let socket_send = socket_recv.clone();

    let jitter_dist = Uniform::<i128>::from(-jitter..(jitter + 1));
    let roll_dist = Uniform::<f32>::from(0f32..100f32);
    
    // Get client address from first packet. Wait for it
    let mut buf = [0; 255];
    let (_, client_src) = socket_recv.peek_from(&mut buf).expect("Error while peeking on socket");
    println!("Client connected from {}", client_src);    

    let _listen_handle = std::thread::spawn(move ||
    {
        let mut rng = rand::thread_rng();
        let mut unordered_packets = HashMap::<SocketAddr, Packet>::new();
        loop
        {
            // Recv packets
            let mut buffer : Vec<u8> = vec![0; 255];
            let res = socket_recv.recv_from(&mut buffer);

            if let Ok((size, src)) = res
            {
                buffer.truncate(size);
                let from_server = src.port() == connect_port;
                let dest = if from_server {client_src} else {server_address};
                let sent_date = gen_sent_date(lag, jitter_dist, &mut rng);
                //println!("Packet recv from {} of size {} to be sent to {}", src, size, dest);

                let roll = roll_dist.sample(&mut rng);
                let arrived = roll > loss_chance;
                if arrived
                {
                    let roll = roll_dist.sample(&mut rng);
                    let duplicated = roll < duplication_chance;
                    if duplicated
                    {
                        let sent_date = gen_sent_date(lag, jitter_dist, &mut rng);
                        let duplicate = Packet::new(buffer.clone(), dest, sent_date);
                        let (q, cvar) = &*packet_queue_recv;
                        let mut q = q.lock().unwrap();
                        q.insert(sent_date, duplicate);
                        cvar.notify_one();
                    }

                    let packet = Packet::new(buffer, dest, sent_date);

                    let roll = roll_dist.sample(&mut rng);
                    let unordered = roll < unorder_chance;
                    if unordered && !unordered_packets.contains_key(&packet.dest)
                    {
                        //println!("Packet is gonna arrive after the next");
                        unordered_packets.insert(packet.dest, packet);
                    }
                    else if let Some((_, prev_packet)) = unordered_packets.remove_entry(&packet.dest)
                    {
                        //println!("Packet queued after the next");
                        let sent_date = packet.sent_date + gen_delay(lag, jitter_dist, &mut rng);
                        let (q, cvar) = &*packet_queue_recv;
                        let mut q = q.lock().unwrap();
                        q.insert(packet.sent_date, prev_packet);
                        q.insert(sent_date, packet);
                        cvar.notify_one();
                    }
                    else
                    {
                        let (q, cvar) = &*packet_queue_recv;
                        let mut q = q.lock().unwrap();
                        q.insert(sent_date, packet);
                        cvar.notify_one();
                    }
                }
            }
        }
    });

    loop
    {
        let now = now_nanos();

        let (q, cvar) = &*packet_queue_send;
        let queue_guard = q.lock().unwrap();
        let (mut queue_guard, _) = cvar.wait_timeout(queue_guard, std::time::Duration::from_micros(10)).unwrap();

        let mut to_send = queue_guard.split_off(&now);
        swap(&mut to_send, &mut queue_guard);
        for (_, packet) in to_send
        {
            //println!("Packet sent to {} of size {}", packet.dest, packet.size());
            socket_send.send_to(&packet.buffer, packet.dest).expect("Error while sending");
        }
    }
}
