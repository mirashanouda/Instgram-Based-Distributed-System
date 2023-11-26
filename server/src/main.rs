use std::net::UdpSocket;
use std::str;
use std::thread;
// use std::time::Duration;
use std::sync::{Arc, Mutex};
use common_lib::utils;

static ID: i32 = 1;

//requests port: 1231 -> 1111
//offline ports: 2231 -> 2222
//token ports: 3231 -> 3333

static ONLINE_SERVERS: [&str; 2] = [
	"127.0.0.1:2222",
    "127.0.0.1:2222",
];

static SERVERS: [&str; 3] = [
	"10.40.32.49",
    "10.40.37.106",
    "10.40.37.106"
];

fn main() {
	let requests_port = 4444;
	let next_server = 2;
	let next_next_server = 3;

	let off_flag = Arc::new(Mutex::new(false));
	let off_flag_clone = Arc::clone(&off_flag);
	thread::spawn(move || utils::failure_token_handle(off_flag_clone, SERVERS[next_server as usize]));

	let off_server = Arc::new(Mutex::new(0));
	let off_server_clone = Arc::clone(&off_server);
	// launch a  thread for offline handler
	thread::spawn(move || utils::who_offline_handler(off_server_clone, ID));

	let off_server_clone = Arc::clone(&off_server);
	let work_flag = Arc::new(Mutex::new(false));
	let work_flag_clone = Arc::clone(&work_flag);
	thread::spawn(move || utils::working_token_handle(off_server_clone, work_flag_clone, next_server, next_next_server, SERVERS));

	println!("Listening for requests on port {}", requests_port);
  	loop {
		let socket = UdpSocket::bind(format!("0.0.0.0:{}", requests_port)).expect("Failed to bind socket");
		// println!("socket = {:?}", socket);
		utils::handle_requests_v2(
			&socket,
			ONLINE_SERVERS,
			Arc::clone(&work_flag),
			Arc::clone(&off_flag),
			ID,
			SERVERS
		);
	}
}