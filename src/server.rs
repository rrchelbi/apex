use anyhow::{Context, Result};
use apex::protocol::{Packet, PacketBuffer, QueryType, Question, ResultCode};
use std::{
    fmt::Display,
    net::{Ipv4Addr, ToSocketAddrs, UdpSocket},
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

fn lookup(qname: impl Into<String>, qtype: QueryType, server: (Ipv4Addr, u16)) -> Result<Packet> {
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

            match recursive_lookup(&question.name, question.qtype) {
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

fn recursive_lookup(qname: &str, qtype: QueryType) -> Result<Packet> {
    let mut ns = "198.41.0.4".parse::<Ipv4Addr>().unwrap();

    loop {
        tracing::debug!("looking up {:?} {} via {}", qtype, qname, ns);

        let response = lookup(qname, qtype, (ns, 53))?;

        match response.header.result_code {
            ResultCode::NOERROR if !response.answers.is_empty() => return Ok(response),
            ResultCode::NXDOMAIN => return Ok(response),
            _ => {}
        }

        if let Some(new_ns) = response.get_resolved_ns(qname) {
            ns = new_ns;
            continue;
        }

        let Some(ns_name) = response.get_unresolved_ns(qname) else {
            return Ok(response);
        };

        let recursive_response = recursive_lookup(ns_name, QueryType::A)?;

        match recursive_response.get_random_a() {
            Some(new_ns) => ns = new_ns,
            None => return Ok(response),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apex::protocol::{QueryType, Record};
    use std::net::Ipv4Addr;

    #[test]
    #[ignore]
    fn test_lookup_a_record() {
        let server = ("8.8.8.8".parse::<Ipv4Addr>().unwrap(), 53);
        let response = lookup("google.com", QueryType::A, server).unwrap();

        assert!(
            !response.answers.is_empty(),
            "expected at least one answer for google.com"
        );
        assert!(
            response
                .answers
                .iter()
                .any(|r| matches!(r, Record::A { .. })),
            "expected an A record in the answer section"
        );
    }

    #[test]
    #[ignore]
    fn test_lookup_nxdomain() {
        use apex::protocol::ResultCode;

        let server = ("8.8.8.8".parse::<Ipv4Addr>().unwrap(), 53);
        let response = lookup("this.domain.does.not.exist.invalid", QueryType::A, server).unwrap();

        assert_eq!(response.header.result_code, ResultCode::NXDOMAIN);
    }

    #[test]
    #[ignore]
    fn test_lookup_aaaa_record() {
        let server = ("8.8.8.8".parse::<Ipv4Addr>().unwrap(), 53);
        let response = lookup("google.com", QueryType::AAAA, server).unwrap();

        assert!(
            response
                .answers
                .iter()
                .any(|r| matches!(r, Record::AAAA { .. })),
            "expected an AAAA record for google.com"
        );
    }

    #[test]
    #[ignore]
    fn test_recursive_lookup_a() {
        let response = recursive_lookup("google.com", QueryType::A).unwrap();

        assert!(
            !response.answers.is_empty(),
            "expected answers from recursive lookup"
        );
        assert!(
            response
                .answers
                .iter()
                .any(|r| matches!(r, Record::A { .. })),
            "expected an A record from recursive lookup"
        );
    }

    #[test]
    #[ignore]
    fn test_recursive_lookup_mx() {
        let response = recursive_lookup("google.com", QueryType::MX).unwrap();

        assert!(
            response
                .answers
                .iter()
                .any(|r| matches!(r, Record::MX { .. })),
            "expected an MX record for google.com"
        );
    }

    #[test]
    #[ignore]
    fn test_recursive_lookup_nxdomain() {
        use apex::protocol::ResultCode;

        let response =
            recursive_lookup("this.domain.does.not.exist.invalid", QueryType::A).unwrap();
        assert_eq!(response.header.result_code, ResultCode::NXDOMAIN);
    }

    #[test]
    #[ignore]
    fn test_handle_query_valid_question() {
        use apex::protocol::PacketBuffer;
        use apex::protocol::{Packet, Question};

        // spin up server on a random port
        let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        server_socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();

        // build a valid A query for google.com
        let mut request = Packet::new();
        request.header.id = 1234;
        request.header.recursion_desired = true;
        request
            .questions
            .push(Question::new("google.com", QueryType::A));

        let mut req_buffer = PacketBuffer::new();
        request.write(&mut req_buffer).unwrap();

        // send the query to our server socket
        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client
            .send_to(&req_buffer.buf[..req_buffer.pos()], server_addr)
            .unwrap();

        // let the server handle it
        handle_query(&server_socket).unwrap();
    }

    #[test]
    fn test_handle_query_empty_question_returns_formerr() {
        use apex::protocol::PacketBuffer;
        use apex::protocol::{Packet, ResultCode};

        let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        server_socket
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();

        // send a packet with no questions
        let mut empty = Packet::new();
        empty.header.id = 9999;

        let mut buf = PacketBuffer::new();
        empty.write(&mut buf).unwrap();

        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client.send_to(&buf.buf[..buf.pos()], server_addr).unwrap();

        // capture the response
        handle_query(&server_socket).unwrap();

        let mut res_buf = PacketBuffer::new();
        client.recv_from(&mut res_buf.buf).unwrap();
        let response = Packet::try_from(&mut res_buf).unwrap();

        assert_eq!(response.header.id, 9999);
        assert_eq!(response.header.result_code, ResultCode::FORMERR);
    }
}
