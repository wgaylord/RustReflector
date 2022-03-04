import asyncio
import json
from http.server import BaseHTTPRequestHandler,HTTPServer,ThreadingHTTPServer
import socket
import requests
import struct
import ipaddress

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
m17_server_address = ('127.0.0.1', 17000)

sock.settimeout(10)

stats= b"{}"
html = b""

def decode_callsign_base40(encoded_bytes):
    unpacked = struct.unpack(">HI",encoded_bytes)
    q = (unpacked[0]<<(8*4))+unpacked[1]  
    call = ""
    while q > 0:
        call += " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-/."[q%40]
        q = q //40
    return call

def loadHTML(): #Load the main client html file
    htmlFile = open("Client.html","rb")
    html = htmlFile.read()
    htmlFile.close()
    return html

class ClientHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/": #Handle requests to the server with no path
            self.send_response(200)
            self.send_header("Content-type", "text/html")
            self.end_headers()
            self.wfile.write(html)
            self.server.path = self.path
            return
        print(self.path)    
        if self.path == "/status":
            try:
                sock.sendto(b"INFO",m17_server_address)
                stats = sock.recv(4000)
            except:
                stats = json.dumps({"ERROR":{"module":"","talking":False}}).encode("ASCII")
            self.send_response(200)
            self.send_header("Content-type", 'application/json')
            self.end_headers()
            clients = {}
            data = stats[4:]
            for x in range(stats[4]):
                callsign = decode_callsign_base40(data[1+(7*x):7+(7*x)])
                module = chr(data[7+(7*x)])
                clients[callsign] = {"module":module,"talking":False}

            self.wfile.write(json.dumps(clients).encode("ASCII")) #Send history in JSOn format
            self.server.path = self.path 
            return
            
        if self.path == "/reflectors":
            reflectors = requests.get("https://reflectors.m17.link/ref-list/json").json()
            reflector_list = b''
            reflector_list += struct.pack(">H",len(reflectors["items"]))
            for x in reflectors["items"]:
                reflector_list += x["designator"].encode("ASCII")
                if x["ipv4"] == None:
                    reflector_list += b"\x00\x00\x00\x00"
                else:
                    reflector_list += ipaddress.ip_address(x["ipv4"]).packed
                if x["ipv6"] == None:
                    reflector_list += b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
                else:
                    reflector_list += ipaddress.ip_address(x["ipv6"]).packed
                reflector_list += struct.pack(">H",int(x["port"]))
            self.send_response(200)
            self.send_header("Content-type", 'application/octet-stream')
            self.end_headers()
            self.wfile.write(reflector_list) #Send history in JSOn format
            self.server.path = self.path 

        
        
def init_server(server_class=ThreadingHTTPServer, handler_class=ClientHandler): 
    server_address = ('0.0.0.0', 3001)
    httpd = server_class(server_address, handler_class)
    #httpd.serve_forever()
    while True:
        httpd.handle_request() #Handle Requests
    
html = loadHTML()
init_server()

