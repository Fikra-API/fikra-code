#!/usr/bin/env python3
"""Minimal HTTP CONNECT proxy that logs every destination host.

Usage: connect_proxy.py <hosts-log> <port>
Writes "<hosts-log>.ready" once listening.
"""
import socket
import sys
import threading

LOG, PORT = sys.argv[1], int(sys.argv[2])
lock = threading.Lock()


def log_host(host: str) -> None:
    with lock, open(LOG, "a") as f:
        f.write(host + "\n")


def pump(src, dst):
    try:
        while True:
            data = src.recv(65536)
            if not data:
                break
            dst.sendall(data)
    except OSError:
        pass
    try:
        dst.shutdown(socket.SHUT_WR)
    except OSError:
        pass


def handle(conn):
    try:
        buf = b""
        while b"\r\n\r\n" not in buf:
            chunk = conn.recv(4096)
            if not chunk:
                return
            buf += chunk
        head, rest = buf.split(b"\r\n\r\n", 1)
        parts = head.split(b"\r\n")[0].decode("latin1").split()
        if len(parts) < 2:
            return
        if parts[0].upper() != "CONNECT":
            # Plain HTTP: log the host and refuse; Fikra traffic is HTTPS.
            target = parts[1].split("//", 1)[-1].split("/", 1)[0]
            log_host(target.rsplit(":", 1)[0])
            conn.sendall(b"HTTP/1.1 403 Forbidden\r\n\r\n")
            return
        hostport = parts[1]
        host, port = hostport.rsplit(":", 1)
        log_host(host)
        upstream = socket.create_connection((host, int(port)), timeout=30)
        conn.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        if rest:
            upstream.sendall(rest)
        threading.Thread(target=pump, args=(upstream, conn), daemon=True).start()
        pump(conn, upstream)
        upstream.close()
    except OSError:
        pass
    finally:
        try:
            conn.close()
        except OSError:
            pass


server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", PORT))
server.listen(64)
with open(LOG + ".ready", "w") as f:
    f.write("ready\n")
while True:
    c, _ = server.accept()
    threading.Thread(target=handle, args=(c,), daemon=True).start()
