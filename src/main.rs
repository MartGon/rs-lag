
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
mod proxy;
mod conditions;
use crate::conditions::Conditions;
use crate::network::Packet;
use crate::proxy::Proxy;

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

    // Initializing proxy
    let conditions = Conditions::new(lag, jitter, duplication_chance, unorder_chance, loss_chance);

    let proxy_addr = format!("0.0.0.0:{}", listen_port);
    let proxy_addr = ToSocketAddrs::to_socket_addrs(&proxy_addr).expect("Could not convert addr").next().unwrap();
    let server_addr = format!("127.0.0.1:{}", connect_port);
    let server_addr = ToSocketAddrs::to_socket_addrs(&server_addr).expect("Could not convert addr").next().unwrap();
    
    let proxy = Proxy::new(proxy_addr, server_addr, conditions);
    proxy.run();
}
