"""Drive an unchanged Gleam TCP/TLS server through explicit port-0 readiness.

Pass the original Erlang, Geam, embedding, or built application command after
this script. Each READY <scenario> <port> selects an explicit client contract
and requires a matching PASS <scenario>. No fixed port or startup sleep is used.
"""

import queue
from contextlib import ExitStack
import os
import signal
import socket
import ssl
import subprocess
import sys
import threading
import time
from pathlib import Path


def read_output(stream, events, label):
    for line in stream:
        events.put((label, line.rstrip("\n")))
    events.put((label, None))


def exchange(kind, port):
    with ExitStack() as sockets:
        streams = []
        for _ in range(2 if kind == "pool" else 1):
            raw = sockets.enter_context(socket.create_connection(
                ("::1" if kind == "ipv6" else "127.0.0.1", port), timeout=10))
            if kind in {"tls", "tls_none", "tls_mismatch"}:
                context = ssl.create_default_context(cafile=str(Path(__file__).parent / "certs/cert.pem"))
                if kind != "tls_none":
                    context.set_alpn_protocols(
                        ["unregistered-protocol"] if kind == "tls_mismatch" else ["http/1.1", "h2"])
                if kind == "tls_mismatch":
                    try:
                        sockets.enter_context(context.wrap_socket(raw, server_hostname="localhost"))
                    except ssl.SSLError:
                        # OpenSSL versions name the fatal alert differently;
                        # the source separately asserts handshake failure.
                        return
                    raise AssertionError("server accepted an ALPN mismatch")
                stream = sockets.enter_context(context.wrap_socket(raw, server_hostname="localhost"))
            else:
                stream = raw
            streams.append(stream)
            stream.settimeout(10)
            stream.sendall(b"ping")
        # Pool clients are connected and have sent their data before either
        # response is read, exercising independent concurrent handlers.
        for stream in streams:
            packet = b""
            while len(packet) < 4:
                chunk = stream.recv(4 - len(packet))
                if not chunk:
                    raise AssertionError("connection closed before complete reply")
                packet += chunk
            if packet != b"pong":
                raise AssertionError(f"{kind} reply: {packet!r}")
            if kind == "tls" and stream.selected_alpn_protocol() != "h2":
                raise AssertionError("server did not negotiate h2")
            if kind == "tls_none" and stream.selected_alpn_protocol() is not None:
                raise AssertionError("server invented ALPN without a client offer")


def main(command):
    if not command:
        raise SystemExit("usage: run.py <application command> [arguments...]")
    events = queue.Queue()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, start_new_session=os.name != "nt")
    readers = [threading.Thread(target=read_output, args=(stream, events, label))
               for stream, label in ((process.stdout, "stdout"), (process.stderr, "stderr"))]
    for reader in readers:
        reader.start()
    expected = []
    completed = []
    ended = 0
    deadline = time.monotonic() + 180
    try:
        while ended < 2:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("application readiness/completion deadline")
            label, line = events.get(timeout=remaining)
            if line is None:
                ended += 1
                continue
            print(line, file=sys.stdout if label == "stdout" else sys.stderr, flush=True)
            if label == "stdout" and line.startswith("READY "):
                _, kind, port = line.split()
                if kind not in {"tcp", "tls", "server", "pool", "ipv6", "tls_none", "tls_mismatch"}:
                    raise AssertionError(f"unknown contract {kind}")
                expected.append(kind)
                exchange(kind, int(port))
            elif label == "stdout" and line.startswith("PASS "):
                completed.append(line.removeprefix("PASS "))
        status = process.wait(timeout=max(1, deadline - time.monotonic()))
        if status or not expected or completed != expected:
            raise AssertionError(f"application exit={status}, ready={expected}, completed={completed}")
    finally:
        if os.name == "nt":
            if process.poll() is None:
                subprocess.run(["taskkill", "/F", "/T", "/PID", str(process.pid)],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        else:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        process.wait(timeout=10)
        for reader in readers:
            reader.join(timeout=10)
        while not events.empty():
            label, line = events.get_nowait()
            if line is not None:
                print(line, file=sys.stdout if label == "stdout" else sys.stderr, flush=True)
        process.stdout.close()
        process.stderr.close()


if __name__ == "__main__":
    main(sys.argv[1:])
