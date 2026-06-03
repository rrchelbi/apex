use anyhow::{Result, Context};
use apex::protocol::{Packet, PacketBuffer, QueryType, Question, ResultCode};
use std::{
    fmt::Display,
    net::{ToSocketAddrs, UdpSocket},
};


pub fn run<A>(addr: A) -> Result<()>
where
    A: ToSocketAddrs + Display + Copy,
{
    let socket = UdpSocket::bind(addr)?;
    tracing::info!("apex listening on {}", addr);

    loop {
        match handle_query(&socket) {
            Ok(_) => {}
            Err(e) => tracing::error!("query failed: {}", e),
        }
    }
}

fn lookup(qname: impl Into<String>, qtype: QueryType) -> Result<Packet> {
    let server = ("8.8.8.8", 53);
    let socket = UdpSocket::bind(("0.0.0.0", 0))?;

    let mut packet = Packet::new();
    packet.header.id = 4444;
    packet.header.question_count = 1;
    packet.header.recursion_desired = true;
    packet.questions.push(Question::new(qname, qtype));

    let mut req_buffer = PacketBuffer::new();
    packet.write(&mut req_buffer)?;
    socket.send_to(&req_buffer.buf[..req_buffer.pos()], server)?;

    let mut res_buffer = PacketBuffer::new();
    socket.recv_from(&mut res_buffer.buf)?;
    Packet::try_from(&mut res_buffer)
}

fn handle_query(socket: &UdpSocket) -> Result<()> {
    let mut request_pb = PacketBuffer::new();
    let (_, src) = socket.recv_from(&mut request_pb.buf)?;
    let mut request = Packet::try_from(&mut request_pb)?;

    let mut response = Packet::new();
    response.header.id = request.header.id;
    response.header.recursion_desired = true;
    response.header.recursion_available = true;
    response.header.is_response = true;

    match request.questions.pop() {
        Some(question) => {
            tracing::info!("received query: {:?}", question);

            match lookup(&question.name, question.qtype) {
                Ok(result) => {
                    response.header.result_code = result.header.result_code;
                    response.questions.push(question);

                    for rec in result.answers {
                        tracing::debug!("answer: {:?}", rec);
                        response.answers.push(rec);
                    }
                    for rec in result.authorities {
                        tracing::debug!("authority: {:?}", rec);
                        response.authorities.push(rec);
                    }
                    for rec in result.additionals {
                        tracing::debug!("additional: {:?}", rec);
                        response.additionals.push(rec);
                    }
                }
                Err(e) => {
                    tracing::error!("lookup failed: {}", e);
                    response.header.result_code = ResultCode::SERVFAIL;
                }
            }
        }
        None => {
            tracing::warn!("received query with no questions");
            response.header.result_code = ResultCode::FORMERR;
        }
    }

    let mut res_buffer = PacketBuffer::new();
    response.write(&mut res_buffer)?;
    let len = res_buffer.pos();
    socket.send_to(
        res_buffer
            .bytes(0, len)
            .context("failed to slice response")?,
        src,
    )?;
    Ok(())
}
