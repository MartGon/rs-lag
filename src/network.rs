
use std::net::SocketAddr;

pub struct Packet{
    pub buffer: Vec<u8>,
    pub dest: SocketAddr,
    pub sent_date : u128,
}

impl Packet
{
    pub fn new(buffer : Vec<u8>, dest : SocketAddr, sent_date : u128) -> Packet{
        Packet{
            buffer, dest, sent_date
        }
    }

    pub fn size(&self) -> usize{
        self.buffer.len()
    }
}

impl PartialEq for Packet
{
    fn eq(&self, other : &Packet) -> bool
    {
        self.sent_date == other.sent_date
    }
}

impl Eq for Packet
{
    
}

impl PartialOrd for Packet
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.sent_date < other.sent_date
        {
            return Some(std::cmp::Ordering::Greater);
        }
        else
        {
            return Some(std::cmp::Ordering::Less);
        }
    }
}

impl Ord for Packet
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.sent_date < other.sent_date
        {
            return std::cmp::Ordering::Greater;
        }
        else
        {
            return std::cmp::Ordering::Less;
        }
    }
}

#[cfg(test)]
mod tests{

    use std::vec;

    use super::*;

    #[test]
    fn it_orders()
    {
        let mut packet_queue = Vec::<Packet>::new();
        let localhost = SocketAddr::V4(std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8080));
        let packet1 = Packet::new(vec![0], localhost, 0);
        let packet2 = Packet::new(vec![0], localhost, 125);
        let packet3 = Packet::new(vec![0], localhost, 50);

        packet_queue.reserve(2);
        packet_queue.push(packet1);
        packet_queue.push(packet2);
        packet_queue.push(packet3);
        packet_queue.sort();

        assert_eq!(packet_queue.last().unwrap().sent_date, 0);
        assert_eq!(packet_queue.first().unwrap().sent_date, 125);
    }
}