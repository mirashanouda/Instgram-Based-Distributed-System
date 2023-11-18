use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use common_lib::queue::Queue;
use common_lib::utils;

static ID: i32 = 3;

//requests port: 1231, 1232, 1233
//offline ports: 2231, 2232, 2233
//token ports: 3231, 3232, 3233

static ONLINE_SERVERS: [&str; 2] = [
	"127.0.0.1:2231",
    "127.0.0.1:2232",
];

fn token_handle(flag: Arc<Mutex<bool>>){
	let token_port = 3230 + ID;
	let next_token_port = 3231;
	let next_token_add = "127.0.0.1";

    let next_server: SocketAddr = format!("{}:{}", next_token_add, next_token_port).parse().expect("Failed to parse server address");
    let token_socket = UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";
    token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    thread::sleep(Duration::from_millis(10 as u64));
    
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

fn main() {
	let requests_port = 1230 + ID;

	//build the servers queue
	let mut servers: Queue<i32> = Queue::new();
	servers.enqueue(1);
	servers.enqueue(2);
	servers.enqueue(3);

	let flag = Arc::new(Mutex::new(false));
	let flag_clone = Arc::clone(&flag);
	// launch a thread for token handler
	thread::spawn(move || token_handle(flag_clone));

	let off_server = Arc::new(Mutex::new(0));
	let off_server_clone = Arc::clone(&off_server);
	// launch a  thread for offline handler
	thread::spawn(move || utils::offline_handler(off_server_clone, ID));

	println!("Listening for requests on port {}", requests_port);
  	loop {
		let socket = UdpSocket::bind(format!("0.0.0.0:{}", requests_port)).expect("Failed to bind socket");
		utils::handle_regular_requests(
			&socket,
			&mut servers,
			Arc::clone(&flag),
			Arc::clone(&off_server),
			ID,
			ONLINE_SERVERS
		);
	}
}