use std::{collections::{BTreeMap, HashMap}, mem::swap, net::{SocketAddr, UdpSocket}, sync::{Arc, Condvar, Mutex}};

use crate::network::Packet;
use crate::conditions::Conditions;

pub struct Proxy{
    packet_queues : HashMap<
        SocketAddr, 
        Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>
    >,
    client_sockets : HashMap<SocketAddr, Arc::<UdpSocket>>,

    server_socket : UdpSocket,
    address : SocketAddr,
    server_address : SocketAddr,

    network_conditions : Conditions,
}

impl Proxy{

    pub fn new(address : SocketAddr, server_address : SocketAddr, network_conditions : Conditions) -> Proxy{
        //let packet_queues = Arc::new((Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new()));
        let packet_queues = HashMap::<SocketAddr, Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>>::new();
        let client_sockets = HashMap::<SocketAddr, Arc::<UdpSocket>>::new();
        Proxy{
            packet_queues,
            client_sockets,
            server_socket: UdpSocket::bind(address).expect(&format!("Could not bind to UDP {}", address)),
            address,
            server_address,
            network_conditions
        }
    }

    pub fn run(mut self) -> ()
    {        
        // Create server's packet queue
        let queue = (Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new());
        self.packet_queues.insert(self.server_address, Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>::new(queue));

        let server_address = self.server_address.clone();

        // Client -> Proxy -> Server Listening thread
        std::thread::spawn(move || {
            let mut buffer : Vec<u8> = vec![0; 255];
            let res = self.server_socket.recv_from(&mut buffer);

            if let Ok((size, src)) = res
            {
                buffer.truncate(size);
                if !self.client_sockets.contains_key(&src)
                {
                    self.add_client(src);
                }

                let dest = server_address;
                self.queue_packet(buffer, src, dest);
            }
        });

        // Server -> Proxy -> Client thread
        loop{
            let time = std::time::SystemTime::now();
            let now = time.duration_since(std::time::UNIX_EPOCH).expect("Time is wrong");
            let now = now.as_nanos();
            
            let queue = self.packet_queues.get(&self.server_address).expect("Server's queue was not added");
            let (q, cvar) = **queue;
            let queue_guard = q.lock().unwrap();
            let (mut queue_guard, _) = cvar.wait_timeout(queue_guard, std::time::Duration::from_micros(10)).unwrap();

            let mut to_send = queue_guard.split_off(&now);
            swap(&mut to_send, &mut queue_guard);
            for (_, packet) in to_send
            {
                self.server_socket.send_to(&packet.buffer, packet.dest).expect("Error while sending");
            }
        }
    }

    fn add_client(self, client_address : SocketAddr)
    {
        // Create new socket address
        let port = self.address.port() + self.client_sockets.len() as u16 + 1;
        let client_socket_addr = self.address;
        client_socket_addr.set_port(port);

        // Crate client socket
        let client_socket = Arc::new(UdpSocket::bind(&client_socket_addr).expect(&format!("Could not bind to dynamic address {}", client_socket_addr)));
        self.client_sockets.insert(client_address, client_socket.clone());

        // Create packet queue
        let queue = (Mutex::new(BTreeMap::<u128, Packet>::new()), Condvar::new());
        self.packet_queues.insert(client_address, Arc::<(Mutex::<BTreeMap::<u128, Packet>>, Condvar)>::new(queue));
        
        std::thread::spawn(move || {
            // Server -> Proxy -> Client Listening thread
            let mut buffer : Vec<u8> = vec![0; 255];
            let res = client_socket.recv_from(&mut buffer);
            let (size, src) = res.expect("Recv from socket failed"); 
            buffer.truncate(size);

            self.queue_packet(buffer, self.server_address, client_address);
        });
    }

    fn queue_packet(self, buffer : Vec<u8>, src: SocketAddr, dest: SocketAddr) -> ()
    {
        let mut rng = rand::thread_rng();
        let &conditions = &self.network_conditions;
        let sent_date = self.network_conditions.gen_sent_date(&mut rng);

        if conditions.arrived(&mut rng)
        {
            // Insert a copy
            if conditions.duplicated(&mut rng)
            {
                let sent_date = conditions.gen_sent_date(&mut rng);
                self.queue_packet_base(buffer.clone(), src, dest, sent_date);
            }
            
            // This packet should arrive after the next. Increase delay
            if conditions.unordered(&mut rng)
            {
                sent_date = sent_date + self.network_conditions.gen_delay(&mut rng);
            }
            
            // Insert new packet into the queue
            self.queue_packet_base(buffer, src, dest, sent_date);
        }
    }

    fn queue_packet_base(self, buffer : Vec<u8>, src: SocketAddr, dest: SocketAddr, sent_date : u128)
    {
        let packet = Packet::new(buffer, dest, sent_date);
        if let Some(packet_queue) = self.packet_queues.get(&src)
        {
            let (q, cvar) = **packet_queue;
            let mut q = q.lock().unwrap();
            q.insert(sent_date, packet);
            cvar.notify_one();
        }
    }
}