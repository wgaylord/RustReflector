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
}

fn main() {
    println!("Hello, M17 world! Starting Reflector.");

    let socket = UdpSocket::bind("0.0.0.0:17000").expect("couldn't bind to address"); //Get a UDP socket setup
    let running = true; //Not used yet but will be used for killing the server gracefully later.
    let mut buf = [0; 54]; //Buffer for holding packets. M17 IP packets are never longer then 54 bytes.
    let mut clients =  HashMap::<SocketAddr,Client>::new(); //Hashmap to hold Client Socket relations. 
    let mut response_bytes = Vec::<u8>::new(); //Holds the response to be sent.

    while running {
        let (number_of_bytes, src_addr) = socket.recv_from(&mut buf).expect("Didn't receive data"); //Recv bytes from the socket. If the message is to long for the buffer the extra bytes are dropped.
                                        
        println!("Number of Bytes {} Buffer {:X?}", number_of_bytes,buf); //Print out the number of bytes recieved and the full buffer. Sort of debug. But useful for testing.
        println!("SRC Client IP/Port Pair {}", src_addr); //Print the ip / port pair for the client that sent this to us.

        handle_packet(&socket,src_addr,&buf,&mut clients); //Call out packet handler. Borrow out socket,buffer and client hashmap. Just give it the src_addr.
        clients.retain(|key, value| { //Used for removing clients that timed out.
            response_bytes.clear(); //Clear the response buffer
            let mut keep = true;
          	match value.ping_time.elapsed() { //Check Elapsed time on the ping of each client.
       	        Ok(elapsed) => {
       	            if elapsed.as_secs() > 10 { //If over 10 secs since last PONG send a PING.
          			    response_bytes.extend([65,67,75,78].iter());
		                response_bytes.extend(value.callsign.iter());
		                socket.send_to(&response_bytes,value.socket_addr);
		                keep = true;
          		    }
          		    if elapsed.as_secs() > 60 { //If over 60 sec since last PONG, DISC the cleint and remove it from the hashmap.
          			    let mut response_bytes = Vec::<u8>::new();
          			    response_bytes.extend([68,73,83,67].iter());
		                response_bytes.extend(value.callsign.iter());
		                socket.send_to(&response_bytes,value.socket_addr);
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


fn handle_packet(socket: &UdpSocket,addr:SocketAddr,buf: &[u8],clients: &mut HashMap::<SocketAddr,Client> ) {
    let mut response_bytes = Vec::<u8>::new(); //Buffer for responses.
    match buf {
	    [67,79,78,78, ..] => { //Handle CONN packets
	        println!("CONN");
	        socket.send_to(&[65,67,75,78],addr); //Send ACKN to the client
	        let client = Client{  //Build a Client struct instance. 
                socket_addr: addr,
    			callsign: [buf[4],buf[5],buf[6],buf[7],buf[8],buf[9]], //Need to find a better way to copy parts of a u8 array in Rust
		    	module: buf[10],
		    	ping_time: SystemTime::now(),
		    	pingged: true
            };
            clients.insert(addr,client); //Add client to hashmap
		    response_bytes.extend([65,67,75,78].iter()); //Add PING to response
		    response_bytes.extend(clients[&addr].callsign.iter()); //Add client callsign
		    socket.send_to(&response_bytes,addr); //Send Ping
		},
	    [80,79,78,71, ..] => { //Handle PONG packets
	        println!("PONG");
	        if clients[&addr].pingged { //Did we actually ping this client? Or is it s dumb client just sending PONG?
	            clients.get_mut(&addr).unwrap().ping_time = SystemTime::now(); //Updating time of last PONG using some hacky stuff to be able to modify struct inside hashmap. :/ Annoying Rust
	            clients.get_mut(&addr).unwrap().pingged = false; //Update pingged field in same way.
			} 
	    },
	    [68,73,83,67, ..] => { //Handle DISC packets
	        println!("DISC"); //Got a DISC need to reply with just DISC for client to properly disconnect.
	        socket.send_to(&[68,73,83,67],addr); //Send DISC
		    clients.remove(&addr); //Remove client from hashmap. NEED TO DO ERROR CHECKING HERE. Non-existant client disconnecting crashes server.
	    },
	    [77,49,55,32, ..] => { //Handle M17 packets
	        println!("M17");
	        for (key, value) in &*clients { //Loop over all the clients.
			    if !(value.socket_addr == addr) { // As long as its not the sender continue to next check.
			        if value.module == clients[&addr].module{ //Is the client we are looking at on the senders module?
				        socket.send_to(&buf,value.socket_addr);//Send them the packet. (Should there be any packet rewriting here? Docs make this unclear.)
			        }
		        }
		    }
	    },
	    [_] => {println!("WERID PACKET!")}, //Packet with only 1 byte of data
	    [] => {println!("IMPOSSIBLE PACKET")}, //Packet with no data !?!
	    [_, ..] => {println!("WERID PACKET!")}, //Packet that doesn't match anything else.
    }
}


