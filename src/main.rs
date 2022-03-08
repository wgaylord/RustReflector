use std::net::UdpSocket;
use std::time::SystemTime;
use std::net::SocketAddr;
use std::collections::HashMap;


struct Client {
    socket_addr: SocketAddr,
    callsign: [u8; 6],
    module: u8,
    ping_time: SystemTime,
    pingged: bool,
    talking: bool,
}

impl Client {
    fn pong(&mut self,) -> (){ //Set last pong recv time and set that we are not waiting for a pong.
        self.ping_time = SystemTime::now();
        self.pingged = false;
    }

    fn new(sock: SocketAddr, call: [u8; 6], module: u8) -> Client {
        Client{
        socket_addr: sock,
        callsign: call,
        module: module,
        ping_time: SystemTime::now(),
        pingged: true,
        talking: false}
    }
}

fn main() {
    println!("Hello, M17 world! Starting Reflector.");

    let socket = UdpSocket::bind("0.0.0.0:17000").expect("couldn't bind to address"); //Get a UDP socket setup
    let running = true; //Not used yet but will be used for killing the server gracefully later.
    let mut buf = [0; 54]; //Buffer for holding packets. M17 IP packets are never longer then 54 bytes.
    let mut clients =  HashMap::<SocketAddr,Client>::new(); //Hashmap to hold Client Socket relations. 
    let mut response_bytes = Vec::<u8>::new(); //Holds the response to be sent.

    while running {
        let (_number_of_bytes, src_addr) = socket.recv_from(&mut buf).expect("Didn't receive data"); //Recv bytes from the socket.
        //If the message is to long for the buffer the extra bytes are dropped.
                                        
        handle_packet(&socket,src_addr,&buf,&mut clients); //Call out packet handler. 
        //Borrow out socket,buffer and client hashmap. Just give it the src_addr.

        clients.retain(|_key, value| { //Used for removing clients that timed out.
            response_bytes.clear(); //Clear the response buffer
            let mut keep = true;
            match value.ping_time.elapsed() { //Check Elapsed time on the ping of each client.
                Ok(elapsed) => {
                    if elapsed.as_secs() > 10 { //If over 10 secs since last PONG send a PING.
                        response_bytes.extend_from_slice(&[80,73,78,71]);
                        response_bytes.extend_from_slice(&value.callsign);
                        socket.send_to(&response_bytes,value.socket_addr).expect("Error sending PING");
                        keep = true;
                    }
                    if elapsed.as_secs() > 60 { //If over 60 sec since last PONG, DISC the cleint and remove it from the hashmap.
                        let mut response_bytes = Vec::<u8>::new();
                        response_bytes.extend_from_slice(&[68,73,83,67]);
                        response_bytes.extend_from_slice(&value.callsign);
                        socket.send_to(&response_bytes,value.socket_addr).expect("Error sending DISC");
                        keep = false;
                    }
                }    
                Err(error) => { //Catch any errors. No idea what error could happen but we need to take care of it.
                    // an error occurred!
                    println!("Error: {:?}", error); 
                }
            }
            keep //Return if we keep it or not.
        });
    }
}


fn handle_packet(socket: &UdpSocket,addr:SocketAddr,buf: &[u8;54],clients: &mut HashMap::<SocketAddr,Client> ) {
    let mut response_bytes = Vec::<u8>::new(); //Buffer for responses.
    match buf {
        [67,79,78,78, ..] => { //Handle CONN packets
            //println!("CONN");
            socket.send_to(&[65,67,75,78],addr).expect("Error sending ACKN"); //Send ACKN to the client
            let client = Client::new(addr,[buf[4],buf[5],buf[6],buf[7],buf[8],buf[9]],buf[10]); //Make a new client
            clients.insert(addr,client); //Add client to hashmap
            response_bytes.extend_from_slice(&[80,73,78,71]); //Add PING to response
            response_bytes.extend_from_slice(&clients[&addr].callsign); //Add client callsign
            socket.send_to(&response_bytes,addr).expect("Error sending PING"); //Send Ping
        },
        [80,79,78,71, ..] => { //Handle PONG packets
            match clients.get_mut(&addr){ //Get client from the hashmap
                Some(client) => { //Client existed
                    if client.pingged { //Did we actually ping it or is it blindly sending pongs?
                        client.pong(); //Set that we recved a pong!
                    }
                }
                None => {println!("PONG from client that never connected?!?");} //Someone sending junk?
            } 
        },
        [68,73,83,67, ..] => { //Handle DISC packets
            //println!("DISC"); //Got a DISC need to reply with just DISC for client to properly disconnect.
            socket.send_to(&[68,73,83,67],addr).expect("Error sending DISC"); //Send DISC
            clients.remove(&addr); //Remove client from hashmap. NEED TO DO ERROR CHECKING HERE. Non-existant client disconnecting crashes server.
        },
        [77,49,55,32, ..] => { //Handle M17 packets
            //println!("M17");
            match clients.get_mut(&addr){ //Get client from the hashmap
                Some(client) => { //Client existed
                    if (buf[36] & 0x80) == 0x80 {
                        client.talking = false;
                    }else{
                        client.talking = true; //Set client to be talking.
                    }
                }
                None => {println!("Unconnected client trying to talk?!");}
            } 
            for (_key, value) in &*clients { //Loop over all the clients.
                if !(value.socket_addr == addr) { // As long as its not the sender continue to next check.
                    if value.module == clients[&addr].module{ //Is the client we are looking at on the senders module?
                        socket.send_to(buf,value.socket_addr).expect("Error sending M17");//Send them the packet. (Should there be any packet rewriting here? Docs make this unclear.)
                    }
                }
            }
        },
        [73,78,70,79, ..] => { //Handle INFO packets - NON STANDARD - Used for the dashboard to conmunicate to the reflector.
            //println!("INFO");
            response_bytes.extend([73,78,70,79]); //Add INFO to the response
            response_bytes.push(clients.len() as u8); //Get number of clients and add to buffer 
            for (_key, value) in &*clients { //Loop over all the clients.
                response_bytes.extend_from_slice(&value.callsign); //Send the 6 bytes for the callsign
                response_bytes.push(value.module); //Send the single byte module
                if value.talking{
                    response_bytes.push(1);
                }else{
                    response_bytes.push(0);                
                }
            }
            socket.send_to(&response_bytes,addr).expect("Error sending INFO");//Send the packet
        },
        _ => {println!("WERID UNKNOWN PACKET!");}, //Anything else
    }
}







