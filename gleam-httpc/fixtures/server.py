"""Run a standalone Geam command against local HTTP and HTTPS fixture servers."""

import ssl
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


FIXTURES = Path(__file__).resolve().parent


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        if self.path == "/redirect":
            self.send_response(302)
            self.send_header("location", "/hello")
            self.send_header("content-length", "0")
            self.end_headers()
            return
        if self.path == "/hello":
            body = b"hello"
        elif self.path == "/invalid-utf8":
            body = bytes([255])
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("content-type", "text/plain; charset=utf-8")
        self.send_header("x-fixture", "yes")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, _format, *_args):
        pass


def main(command):
    if not command:
        raise SystemExit("usage: python3 server.py COMMAND [ARG ...]")
    http = ThreadingHTTPServer(("127.0.0.1", 38199), Handler)
    https = ThreadingHTTPServer(("127.0.0.1", 38200), Handler)
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(FIXTURES / "localhost-cert.pem", FIXTURES / "localhost-key.pem")
    https.socket = tls.wrap_socket(https.socket, server_side=True)
    threads = [
        threading.Thread(target=server.serve_forever, daemon=True)
        for server in (http, https)
    ]
    for thread in threads:
        thread.start()
    try:
        return subprocess.run(command, check=False).returncode
    finally:
        for server in (http, https):
            server.shutdown()
            server.server_close()
        for thread in threads:
            thread.join()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
