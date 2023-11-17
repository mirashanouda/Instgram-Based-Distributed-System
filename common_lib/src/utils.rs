use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

pub fn token_handle(flag: Arc<Mutex<bool>>, id: i32, nxt_id: i32){
	let token_port = 3230 + id;
	let next_token_port = 3230 + nxt_id;
	let next_token_add = "127.0.0.1";

	let next_server: SocketAddr = format!("{}:{}", next_token_add, next_token_port).parse().expect("Failed to parse server address");
    let token_socket = UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";
    
    loop{
		let mut buffer = [0; 512];
		let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
		let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
		let mut token = flag.lock().unwrap();
		*token = true;
		drop(token);
		// println!("I have the token now :( Yalaaaaahwy");
		thread::sleep(Duration::from_millis(2000 as u64));
		token = flag.lock().unwrap();
		*token = false;
		drop(token);
		// println!("Released token now :)");
		token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}


pub fn send_offline (msg: i32, online_servers: [&str; 2]){
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
	// send to all servers my current status (offline/online)
	for server in online_servers {
		let server_add: SocketAddr = server.parse().expect("Failed to parse server address");
		socket.send_to(&msg.to_be_bytes(), server_add).unwrap();
		thread::sleep(Duration::from_secs(5));
	}
}

pub fn offline_handler(off_server: Arc<Mutex<i32>> , id: i32) {
    let offline_port = 2230 + id;
    let offline_add = format!("127.0.0.1:{}", offline_port);
    let socket = UdpSocket::bind(offline_add).expect("Failed to bind socket");
    // listen for messages from other clients and print them out
    loop{
        let mut buffer = [0; 4];
		socket.recv_from(&mut buffer).expect("Failed to receive message");
        let mut off_server_id = off_server.lock().unwrap();
        *off_server_id = i32::from_be_bytes(buffer);
        println!("Sleep server is: {}", off_server_id);
    }
}