use anyhow::Result;
use apex::protocol::{Packet, PacketBuffer};
use std::{fs::File, io::Read};

fn main() -> Result<()> {
    let mut file = File::open("/tmp/response_packet.txt")?;
    let mut pb = PacketBuffer::new();
    file.read(&mut pb.buf)?;

    let packet = Packet::try_from(&mut pb)?;
    println!("{:#?}", packet.header);

    for q in packet.questions {
        println!("{:#?}", q);
    }

    Ok(())
}
