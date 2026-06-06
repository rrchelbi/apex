# apex

A DNS server built from scratch in Rust.

## what it does

Parses raw DNS packets from the wire, resolves queries recursively, and responds — no dependencies on system resolvers.

Built on top of a hand-rolled byte buffer parser that speaks RFC 1035 directly.

## quick start

```bash
git clone https://github.com/rrchelbi/apex
cd apex
cargo run
```

Server binds to `0.0.0.0:2053` by default.

To test it:

```bash
dig @127.0.0.1 -p 2053 google.com
dig @127.0.0.1 -p 2053 google.com AAAA
dig @127.0.0.1 -p 2053 gmail.com MX
```

To see the full resolution trace:

```bash
RUST_LOG=debug cargo run
```

```bash
INFO apex: listening on 0.0.0.0:2053
INFO apex: received query: DnsQuestion { name: "google.com", qtype: A }
DEBUG apex: looking up A google.com via 198.41.0.4
DEBUG apex: looking up A google.com via 216.239.34.10
DEBUG apex: answer: A { domain: "google.com", addr: 142.250.185.46, ttl: 207 }
```

## build

```bash
cargo build --release          # target/release/apex
cargo install --path .         # install globally
```

## stack

| crate                | why                  |
| -------------------- | -------------------- |
| `anyhow`             | error propagation    |
| `tracing`            | structured logging   |
| `tracing-subscriber` | `RUST_LOG` filtering |

## status

early stage. currently handles:

- [x] zero-copy wire format parser and serializer
- [x] A, AAAA, NS, CNAME, MX record support
- [x] recursive resolution from root servers
- [x] unknown record passthrough
- [ ] TTL caching
- [ ] concurrent query handling
- [ ] authoritative mode / zone files
- [ ] DoT / DoH

## license

MIT
