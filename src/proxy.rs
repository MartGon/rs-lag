use std::{collections::BTreeMap, net::SocketAddr, sync::{Arc, Condvar, Mutex}};

use crate::network::Packet;
use crate::conditions::Conditions;

pub struct Proxy{
    packet_queue_recv : Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>,
    packet_queue_send : Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>,
    address : SocketAddr,
    server_address : SocketAddr,

    network_conditions : Conditions,
}

impl Proxy{

    pub fn new(address : SocketAddr, server_address : SocketAddr, network_conditions : Conditions) -> Proxy{
        let packet_queue_recv = Arc::new((Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new()));
        Proxy{
            packet_queue_recv: packet_queue_recv,
            packet_queue_send: Arc::clone(&packet_queue_recv),
            address,
            server_address,
            network_conditions
        }
    }
}