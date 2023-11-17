use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use common_lib::queue::Queue;
use common_lib::utils;

static ID: i32 = 1;

//requests port: 1231, 1232, 1233
//offline ports: 2231, 2232, 2233
//token ports: 3231, 3232, 3233

static ONLINE_SERVERS: [&str; 2] = [
	"127.0.0.1:2232",
    "127.0.0.1:2233",
];

fn handle_regular_requests(socket: &UdpSocket, servers: &mut Queue<i32>, flg: Arc<Mutex<bool>>, off_server: Arc<Mutex<i32>>) {
    let mut buffer = [0; 512];
	let mut size = 512;
	let mut _src_addr;
	let mut handled: bool = true;
    loop {
		if handled {
        	(size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
		}
		if let Some((old, new_head)) = servers.dequeue() {
            let token = flg.lock().unwrap();
            
            if new_head == Some(&ID) {
                // Case 1: Top of the queue and not offline
                if *token == false {
                    let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
                    println!("Handling request: {}", message);
					handled = true;
                } 
                // Case 2: Top of the queue and Offline
                else {
					println!("Offline :(");
                    // utils::send_offline(ID, ONLINE_SERVERS);
					// thread::sleep(Duration::from_secs(2 as u64));
					// utils::send_status(format!("online - {}", ID), ONLINE_SERVERS)
                }
            }
			// Case 3: not top of the queue and online
			// else {
			// 	handled = false;
			// }
            servers.enqueue(old);
            drop(token);
        }
    }
}


fn main() {
	let requests_port = 1230 + ID;

	//build the servers queue
	let mut servers: Queue<i32> = Queue::new();
	servers.enqueue(1);
	servers.enqueue(2);
	// servers.enqueue(3);

	let flag = Arc::new(Mutex::new(false));
	let flag_clone = Arc::clone(&flag);
	// launch a thread for token handler
	thread::spawn(move || utils::token_handle(flag_clone, ID, ID+1));

	let off_server = Arc::new(Mutex::new(0));
	let off_server_clone = Arc::clone(&off_server);
	// launch a  thread for offline handler
	thread::spawn(move || utils::offline_handler(off_server_clone, ID));

	println!("Listening for requests on port {}", requests_port);
  	loop {
		let socket = UdpSocket::bind(format!("0.0.0.0:{}", requests_port)).expect("Failed to bind socket");
		handle_regular_requests(
			&socket,
			&mut servers,
			Arc::clone(&flag),
			Arc::clone(&off_server)
		);
	}
}