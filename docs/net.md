# Networking

Vulcan 1.16 ships a TCP socket layer that works identically on all three
backends (C, JS/Node, Python 3) — including a full Discord-style chat app
built on top of it (`examples/discord.vulcan`). Sockets are text-oriented:
the API sends and receives strings, and NUL bytes truncate a message.

## The API

| builtin | signature | notes |
|---|---|---|
| `net_listen` | `(port: i64) -> i64` | bind + listen; `0` picks an ephemeral port; returns a server handle |
| `net_port`   | `(sock: i64) -> i64` | the actual port a server is listening on |
| `net_accept` | `(sock: i64) -> i64` | accept one pending connection; `-1` if none (non-blocking) |
| `net_connect`| `(host: string, port: i64) -> i64` | blocking with a 5s timeout; returns a connection handle |
| `net_send`   | `(conn: i64, data: string) -> i64` | bytes sent, or `-1` on error |
| `net_recv`   | `(conn: i64) -> string` | everything buffered; `""` if nothing (non-blocking) |
| `net_ready`  | `(sock: i64) -> bool` | server: a connection is pending; conn: data is buffered |
| `net_alive`  | `(conn: i64) -> bool` | peer still connected (false for closed handles and servers) |
| `net_peer`   | `(conn: i64) -> string` | `"ip:port"` of the remote end |
| `net_close`  | `(h: i64) -> bool` | close any handle |

## A tiny echo server

```vulcan
let sock = net_listen(0)          // 0 = ephemeral port
print("listening on port ", net_port(sock))
while true {
    if net_ready(sock) {
        let conn = net_accept(sock)
        if conn >= 0 {
            let data = net_recv(conn)
            if data != "" { net_send(conn, "echo: " + data) }
        }
    }
    sleep_ms(20)
}
```

## A client

```vulcan
let conn = net_connect("127.0.0.1", 7777)
if conn >= 0 {
    net_send(conn, "hello\n")
    while net_ready(conn) == false { sleep_ms(2) }   // wait for the reply
    print(net_recv(conn))
    net_close(conn)
}
```

## Backend notes

- **C** — plain POSIX sockets (`_WIN32` guarded with winsock), `fcntl`
  non-blocking connects, a 5s connect timeout. Handles are table ids, so
  `-1`/`0` never collide with real fds.
- **JS** — sockets live on a `worker_threads` worker; the main thread talks
  to it through a `SharedArrayBuffer` + `Atomics.wait` protocol, so
  `net_recv` never blocks the event loop. Because a `net_send` on the main
  thread returns before the worker has written to the socket, a reply can
  race a follow-up `net_ready` — use the small retry loop shown above.
- **Python** — the `socket` module with `select`; same handle-table design.

The three backends produce byte-identical program output — the selftest runs
every networking case on all of them, and `examples/discord.vulcan` is a
full client/server integration test (see `scripts/e2e-discord.sh`).

## HTTP on top of sockets

`lib/http.vulcan` is a pure-Vulcan HTTP/1.1 client built on these builtins:

```vulcan
let r = http_get("example.com", 80, "/")
print(http_status(r))    // 200
print(http_body(r))      // the response body
print(http_header(r, "content-type"))
```

## Name-shadowing hardening

User functions can be named anything — including `sys`, `os`, `time`,
`select`, `process`, `console` or `require` — without breaking the runtime:
the Python prelude imports its modules under private aliases, and the JS
backend wraps the whole user program in an IIFE so its declarations can
never shadow Node globals. (Before 1.16, a vulcan `fn sys(...)` silently
disabled `key_ready()` on the Python backend.)
