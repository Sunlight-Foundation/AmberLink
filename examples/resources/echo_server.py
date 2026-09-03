# echo_server.py — local test server for examples/net_test.amb (stdlib only).
# Usage: python examples/resources/echo_server.py   (serves 127.0.0.1:8123)
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    def _send(self, body: bytes):
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/hello":
            self._send(b"hello from local server")
        else:
            self.send_error(404)

    def do_POST(self):
        if self.path == "/echo":
            n = int(self.headers.get("Content-Length", 0))
            self._send(self.rfile.read(n))
        else:
            self.send_error(404)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    HTTPServer(("127.0.0.1", 8123), Handler).serve_forever()
