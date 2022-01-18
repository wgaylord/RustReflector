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

    let socket = UdpSocket::bind("0.0.0.0:17000").expect("couldn't bind to address");
    let running = true;
    let mut buf = [0; 54];
    let mut clients =  HashMap::<SocketAddr,Client>::new();
    let mut response_bytes = Vec::<u8>::new();

    while running {
        let (number_of_bytes, src_addr) = socket.recv_from(&mut buf).expect("Didn't receive data");
                                        
        println!("Number of Bytes {} Buffer {:X?}", number_of_bytes,buf);
        println!("New client {}", src_addr);

        handle_packet(&socket,src_addr,&buf,&mut clients);
        clients.retain(|key, value| {
            response_bytes.clear();
            let mut keep = true;
          	match value.ping_time.elapsed() {
       	        Ok(elapsed) => {
       	            if elapsed.as_secs() > 10 {
          			    response_bytes.extend([65,67,75,78].iter());
		                response_bytes.extend(value.callsign.iter());
		                socket.send_to(&response_bytes,value.socket_addr);
		                keep = true;
          		    }
          		    if elapsed.as_secs() > 60 {
          			    let mut response_bytes = Vec::<u8>::new();
          			    response_bytes.extend([68,73,83,67].iter());
		                response_bytes.extend(value.callsign.iter());
		                socket.send_to(&response_bytes,value.socket_addr);
		                keep = false;
          		    }
          		}	
                Err(e) => {
                    // an error occurred!
                    println!("Error: {:?}", e);
                }
            }
            keep //Return if we keep it or not.
        });
    }
}


fn handle_packet(socket: &UdpSocket,addr:SocketAddr,buf: &[u8],clients: &mut HashMap::<SocketAddr,Client> ) {
    let mut response_bytes = Vec::<u8>::new();
    match buf {
	    [67,79,78,78, ..] => { //Handle CONN packets
	        println!("CONN");
	        socket.send_to(&[65,67,75,78],addr);
	        let client = Client{ 
                socket_addr: addr,
    			callsign: [buf[4],buf[5],buf[6],buf[7],buf[8],buf[9]],
		    	module: buf[10],
		    	ping_time: SystemTime::now(),
		    	pingged: true
            };
            clients.insert(addr,client);
		    response_bytes.extend([65,67,75,78].iter());
		    response_bytes.extend(clients[&addr].callsign.iter());
		    socket.send_to(&response_bytes,addr);
		},
	    [80,79,78,71, ..] => { //Handle PONG packets
	        println!("PONG");
	        if clients[&addr].pingged {
	            clients.get_mut(&addr).unwrap().ping_time = SystemTime::now(); //Use some hacky stuff to be able to modify struct inside hashmap. :/ Annoying Rust
	            clients.get_mut(&addr).unwrap().pingged = false;
			} 
	    },
	    [68,73,83,67, ..] => { //Handle DISC packets
	        println!("DISC"); 
	        socket.send_to(&[68,73,83,67],addr);
		    clients.remove(&addr);
	    },
	    [77,49,55,32, ..] => { //Handle M17 packets
	        println!("M17");
	        for (key, value) in &*clients {
			    if !(value.socket_addr == addr) {
			        if value.module == clients[&addr].module{
				        socket.send_to(&buf,value.socket_addr);
			        }
		        }
		    }
	    },
	    [_] => {println!("WERID PACKET!")}, //Packet with only 1 byte of data
	    [] => {println!("IMPOSSIBLE PACKET")}, //Packet with no data !?!
	    [_, ..] => {println!("WERID PACKET!")}, //Packet that doesn't match anything else.
    }
}


