use std::{collections::{BTreeMap, HashMap}, mem::swap, net::{SocketAddr, UdpSocket}, sync::{Arc, Condvar, Mutex}, time::Duration};

use crate::{network::Packet};
use crate::conditions::Conditions;

pub struct Proxy{
    packet_queues : HashMap<
        SocketAddr, 
        Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>
    >,

    server_socket : Arc::<UdpSocket>,
    address : SocketAddr,
    server_address : SocketAddr,

    client_proxy_conditions : Conditions,
    server_proxy_conditions : Conditions,
    timeout : Duration,
}

impl Proxy{

    pub fn new(address : SocketAddr, server_address : SocketAddr, client_proxy_conditions : Conditions, server_proxy_conditions : Conditions, timeout : Duration) -> Proxy{
        let packet_queues = HashMap::<SocketAddr, Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>>::new();
        Proxy{
            packet_queues,
            server_socket: Arc::new(UdpSocket::bind(address).expect(&format!("Could not bind to UDP {}", address))),
            address,
            server_address,
            client_proxy_conditions,
            server_proxy_conditions,
            timeout
        }
    }

    pub fn run(&mut self) -> ()
    {        
        // Create server's packet queue
        let queue = (Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new());
        self.packet_queues.insert(self.server_address, Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>::new(queue));

        // Spawn Proxy -> Client thread
        let server_queue = self.get_server_queue().clone();
        let server_socket = self.server_socket.clone();
        std::thread::spawn(move || {
            loop{
                Proxy::send_pending_packets(&server_queue, &server_socket);
            }
        });

        // Spawn Client -> Proxy thread
        let server_address = self.server_address.clone();
        loop{
            let mut buffer : Vec<u8> = vec![0; 255];
            let res = self.server_socket.recv_from(&mut buffer);

            if let Ok((size, src)) = res
            {
                buffer.truncate(size);
                if !self.packet_queues.contains_key(&src)
                {
                    self.add_client(src);
                    println!("Client connected from {}", src);
                }

                let dest = server_address;
                self.queue_packet(buffer, src, dest);
            }
        }
    }

    fn get_server_queue(&self) -> &Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>
    {
        return self.packet_queues.get(&self.server_address).expect("Server's queue was not added");
    }

    fn add_client(&mut self, client_address : SocketAddr)
    {
        // Create new socket address
        let port = self.address.port() + self.packet_queues.len() as u16 + 1;
        let mut client_socket_addr = self.address;
        client_socket_addr.set_port(port);

        // Crate client socket
        let client_socket = Arc::new(UdpSocket::bind(&client_socket_addr).expect(&format!("Could not bind to dynamic address {}", client_socket_addr)));
        client_socket.set_read_timeout(Some(self.timeout)).expect("Error on set read timeout");

        // Create packet queue
        let queue = (Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new());
        let queue = Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>::new(queue);
        self.packet_queues.insert(client_address, queue.clone());
        
        // Server -> Proxy Listening thread
        let conditions = self.server_proxy_conditions.clone();
        let queue_recv = self.get_server_queue().clone();
        let client_socket_recv = client_socket.clone();
        let _handle = std::thread::spawn(move || {
            while Proxy::recv_inc_packets(&queue_recv, &client_socket_recv, client_address, conditions)
            {
            }
            println!("Client disconnected from {}", client_address);
        });

        // Proxy -> Server Listening thread
        let queue_send = queue.clone();
        let client_socket_send = client_socket.clone();
        let _handle = std::thread::spawn(move ||{
            while Arc::strong_count(&client_socket_send) > 1
            {
                Proxy::send_pending_packets(&queue_send, &client_socket_send);
            }
        });
    }

    fn recv_inc_packets(queue : &Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>, socket : &Arc<UdpSocket>, dest : SocketAddr, conditions : Conditions) -> bool
    {
        let mut buffer : Vec<u8> = vec![0; 4096];
        let res = socket.recv_from(&mut buffer);
        let handled = res.is_ok();
        if let Some((size, _src)) = res.ok()
        {
            buffer.truncate(size);

            //println!("Recv packet from server {} with destiny {}", _src, dest);
            Proxy::queue_packet_static(&queue, buffer, dest, conditions);
        }

        return handled;
    }

    fn send_pending_packets(queue : &Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>, socket : &Arc<UdpSocket>)
    {
        let time = std::time::SystemTime::now();
        let now = time.duration_since(std::time::UNIX_EPOCH).expect("Time is wrong");
        let now = now.as_nanos();
        
        let (q, cvar) = &**queue;
        let queue_guard = q.lock().unwrap();
        let (mut queue_guard, _) = cvar.wait_timeout(queue_guard, std::time::Duration::from_micros(10)).unwrap();

        let mut to_send = queue_guard.split_off(&now);
        swap(&mut to_send, &mut queue_guard);
        for (_, packet) in to_send
        {
            //println!("Sending packet from proxy socket {} to server {}", client_socket_addr, packet.dest);
            socket.send_to(&packet.buffer, packet.dest).expect("Error while sending");
        }
    }

    fn queue_packet(&mut self, buffer : Vec<u8>, src: SocketAddr, dest: SocketAddr)
    {
        if let Some(packet_queue) = self.packet_queues.get(&src)
        {
            Proxy::queue_packet_static(packet_queue, buffer, dest, self.client_proxy_conditions);
        }
    }

    fn queue_packet_static(packet_queue: &Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>, buffer : Vec<u8>, 
        dest: SocketAddr, conditions : Conditions)
    {
        let mut rng = rand::thread_rng();
        let mut sent_date = conditions.gen_sent_date(&mut rng);

        if conditions.arrived(&mut rng)
        {
            // Insert a copy
            if conditions.duplicated(&mut rng)
            {
                let sent_date = conditions.gen_sent_date(&mut rng);
                Proxy::queue_packet_base_static(&packet_queue, buffer.clone(), dest, sent_date);
            }
            
            // This packet should arrive after the next. Increase delay
            if conditions.unordered(&mut rng)
            {
                sent_date = sent_date + conditions.gen_delay(&mut rng);
            }
            
            // Insert new packet into the queue
            Proxy::queue_packet_base_static(&packet_queue, buffer, dest, sent_date);
        }
    }

    fn queue_packet_base_static(packet_queue: &Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>, buffer : Vec<u8>, 
        dest: SocketAddr, sent_date: u128)
    {
        let packet = Packet::new(buffer, dest, sent_date);
        let (q, cvar) = &**packet_queue;
        let mut q = q.lock().unwrap();
        if !q.contains_key(&sent_date)
        {
            q.insert(sent_date, packet);
            cvar.notify_one();
        }
        else
        {
            println!("Error: Queue already contains a packet for that date");
        }
    }
}