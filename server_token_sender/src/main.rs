use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
mod queue;
use queue::Queue;

static ID: i32 = 2;

static OTHER_SERVERS: [&str; 2] = [
	"127.0.0.1:65432",
    "127.0.0.1:1234",
];


fn send_status (msg: String){
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
	// send to all servers my current status (offline/online)
	for server in OTHER_SERVERS {
		let server_add: SocketAddr = server.parse().expect("Failed to parse server address");
		socket.send_to(msg.as_bytes(), server_add).unwrap();
		thread::sleep(Duration::from_secs(5));
	}
}

fn handle_regular_requests(socket: &UdpSocket, servers: &mut Queue<i32>, flg: Arc<Mutex<bool>>) {
    let mut buffer = [0; 512];

    loop {
        let (size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        if let Some((old, new_head)) = servers.dequeue() {
            let token = flg.lock().unwrap();
            
            if new_head == Some(&ID) {
                // Case 1: Top of the queue and not offline
                if !*token {
                    let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
                    println!("Handling request: {}", message);
                } 
                // Case 2: Top of the queue and Offline
                else {
                    send_status(format!("offline - {}", ID));
					thread::sleep(Duration::from_secs(2 as u64));
					send_status(format!("online - {}", ID))
                }
            }
            servers.enqueue(old);
            drop(token);
        }
    }
}

fn token_handle(flag: Arc<Mutex<bool>>){
	let token_port = 1235;
	let next_token_port = 1236;
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
		println!("I have the token now :( Yalaaaaahwy");
		thread::sleep(Duration::from_millis(2000 as u64));
		token = flag.lock().unwrap();
		*token = false;
		drop(token);
		println!("Released token now :)");
		token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}

fn main() {
	let requests_port = 1232;

	//build the servers queue
	let mut servers: Queue<i32> = Queue::new();
	servers.enqueue(1);
	servers.enqueue(2);
	// servers.enqueue(3);

	let flag = Arc::new(Mutex::new(false));
	let flag_clone = Arc::clone(&flag);
	// launch a thread for token handler
	thread::spawn(move || token_handle(flag_clone));

	println!("Listening for requests on port {}", requests_port);
  	loop {
		let socket = UdpSocket::bind(format!("0.0.0.0:{}", requests_port)).expect("Failed to bind socket");
		handle_regular_requests(&socket , &mut servers, Arc::clone(&flag));
	}
}