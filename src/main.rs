
#[macro_use]
extern crate clap;
use clap::App;


use std::net::{ToSocketAddrs};
use std::time::Duration;

mod network;
mod proxy;
mod conditions;
use crate::conditions::Conditions;
use crate::proxy::Proxy;

fn main() {

    // Arg parsing
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let bind_port = matches.value_of("bind-port").unwrap();
    let bind_port = bind_port.parse::<u16>().unwrap();

    let connect_addr = matches.value_of("connect-addr").unwrap();
    let server_addr = ToSocketAddrs::to_socket_addrs(&connect_addr).expect("Could not convert connect-addr").next().unwrap();

    println!("The listen port is {}", bind_port);
    println!("Datagrams will be redirected to {}", connect_addr);

    let lag = matches.value_of("lag").unwrap();
    let lag = std::cmp::max((lag.parse::<u128>().unwrap()) / 2, 0);
    println!("A packet will take {} ms to traverse the network", lag);

    let jitter = matches.value_of("jitter").unwrap();
    let jitter : i128 = jitter.parse::<i128>().unwrap();
    println!("The RTT will have {} ms variance", jitter);

    let cs_duplication_chance = matches.value_of("client-server-duplication").unwrap();
    let cs_duplication_chance : f32 = cs_duplication_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Client->Server Packet Duplication chance: {}", cs_duplication_chance);

    let cs_unorder_chance = matches.value_of("client-server-unorder").unwrap();
    let cs_unorder_chance : f32 = cs_unorder_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Client->Server Packet Unorder chance: {}", cs_unorder_chance);

    let cs_loss_chance = matches.value_of("client-server-loss").unwrap();
    let cs_loss_chance : f32 = cs_loss_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Client->Server Packet Loss chance: {}", cs_loss_chance);

    let sc_duplication_chance = matches.value_of("server-client-duplication").unwrap();
    let sc_duplication_chance : f32 = sc_duplication_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Server->Client Packet Duplication chance: {}", sc_duplication_chance);

    let sc_unorder_chance = matches.value_of("server-client-unorder").unwrap();
    let sc_unorder_chance : f32 = sc_unorder_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Server->Client Packet Unorder chance: {}", sc_unorder_chance);

    let sc_loss_chance = matches.value_of("server-client-loss").unwrap();
    let sc_loss_chance : f32 = sc_loss_chance.parse::<f32>().unwrap().min(100f32).max(0f32);
    println!("Server->Client Packet Loss chance: {}", sc_loss_chance);

    let timeout = matches.value_of("timeout").unwrap();
    let timeout = timeout.parse::<u64>().unwrap();
    println!("Timeout : {}", timeout);

    // Initializing proxy
    let server_client_conditions = Conditions::new(lag, jitter, sc_duplication_chance, sc_unorder_chance, sc_loss_chance);
    let client_server_conditions = Conditions::new(lag, jitter, cs_duplication_chance, cs_unorder_chance, cs_loss_chance);

    let proxy_addr = format!("0.0.0.0:{}", bind_port);
    let proxy_addr = ToSocketAddrs::to_socket_addrs(&proxy_addr).expect("Could not convert addr").next().unwrap();

    let mut proxy = Proxy::new(proxy_addr, server_addr, 
        client_server_conditions, server_client_conditions, Duration::from_millis(timeout));
    proxy.run();
}
