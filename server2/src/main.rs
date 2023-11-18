use std::net::UdpSocket;
use std::str;
use std::thread;
// use std::time::Duration;
use std::sync::{Arc, Mutex};
use common_lib::queue::Queue;
use common_lib::utils;

static ID: i32 = 2;

//requests port: 1231, 1232, 1233
//offline ports: 2231, 2232, 2233
//token ports: 3231, 3232, 3233

static ONLINE_SERVERS: [&str; 2] = [
	"127.0.0.1:2231",
    "127.0.0.1:2233",
];

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
	thread::spawn(move || utils::token_handle(flag_clone, ID, ID+1));

	let off_server = Arc::new(Mutex::new(0));
	let off_server_clone = Arc::clone(&off_server);
	// launch a  thread for offline handler
	thread::spawn(move || utils::offline_handler(off_server_clone, ID));

	// let req_buff = Arc::new(Mutex::new(Queue::new()));
	// let req_buff_clone = Arc::clone(&req_buff);
	// // launch a  thread for offline handler
	// thread::spawn(move || utils::buffer_requests(requests_port, req_buff_clone));

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