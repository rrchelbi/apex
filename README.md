# apex

A DNS server built from scratch in Rust.

---

## what it does

Parses raw DNS packets from the wire, resolves queries recursively, and responds — no dependencies on system resolvers.

Built on top of a hand-rolled byte buffer parser that speaks RFC 1035 directly.

---

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
```

---

## build

```bash
# dev
cargo build

# release
cargo build --release
```

Binary lands at `target/release/oxidns`.

---

## stack

| crate    | why               |
| -------- | ----------------- |
| `anyhow` | error propagation |

---

## status

early stage. currently handles:

- [x] packet parsing (header, question, answer sections)
- [x] A record resolution
- [ ] recursive resolution
- [ ] AAAA, MX, CNAME records
- [ ] caching

---

## license

MIT
