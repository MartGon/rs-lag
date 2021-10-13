
#[macro_use]
extern crate clap;
use clap::App;

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let listen_port = matches.value_of("listen-port").unwrap();
    let listen_port = listen_port.parse::<u16>().unwrap();

    let connect_port = matches.value_of("connect-port").unwrap();
    let connect_port = connect_port.parse::<u16>().unwrap();
    
    println!("The listen port is {}", listen_port);
    println!("Datagrams will be redirected to port {}", connect_port);

    let server_address = ToSocketAddrs::to_socket_addrs(&format!("127.0.0.1:{}", connect_port)[..]).expect("Invalid connection port").next().unwrap();
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", listen_port)).expect(&format!("Could not bind to UDP port {}", listen_port)[..]);

    let mut client_src : Option<SocketAddr> = None;
    loop
    {
        let mut buf = [0; 255];
        if let Ok(res) = socket.recv_from(&mut buf)
        {
            let (size, src) = (res.0, res.1);

            if client_src == None
            {
                client_src = Some::<SocketAddr>(src);
            }

            let buf = &mut buf[..size];
            println!("Packet recv from {} of size {}", src, size);

            let millis = std::time::Duration::from_millis(5);
            std::thread::sleep(millis);

            let from_server = src.port() == 8081;
            if from_server
            {
                println!("Packet sent to client");
                socket.send_to(buf, client_src.unwrap()).expect("Error while sending to client");
            }
            else
            {
                println!("Packet sent to server");
                socket.send_to(buf, &server_address).expect("Error while sending to server");
            }
        }
    }
}
