use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
mod queue;
use queue::Queue;
//30,32

static ID: i32 = 2;

static  OTHER_SERVERS : &str = "10.40.41.175:65432"; // List other server addresses here

fn handle_regular_requests(socket: &UdpSocket, servers: &mut Queue<i32>, flg: Arc<Mutex<bool>>) {
    let mut buffer = [0; 512];

    loop {
        let (size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        println!("yes\n");
        if let Some((old, new_head)) = servers.dequeue() {
            let mut token = flg.lock().unwrap();

            if new_head == Some(&ID) {
                println!("first\n");
                if !*token { // Case 1: Top of the queue and not offline
                    let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
                    println!("Handling request: {}", message);
                    let msg = "Ack";
                    socket.send_to(msg.as_bytes(), "10.40.41.175:65421").expect("Failed to send message");
                        thread::sleep(Duration::from_millis(5 as u64));
                } else { // Case 2: Offline
                    println!("second\n");
                    thread::sleep(Duration::from_secs(5)); // Example sleep duration
                    *token = false; // Mark server as back online
                }
            } 

            servers.enqueue(old);
            drop(token);
        }
    }
}






fn token_handle(flag: Arc<Mutex<bool>>){
    let next_server: SocketAddr = "10.40.41.175:65430".parse().expect("Failed to parse server address");
    let token_socket = UdpSocket::bind("0.0.0.0:65430").expect("Failed to bind socket");
    let msg = "ball";
    token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    thread::sleep(Duration::from_millis(10 as u64));
    loop{
        let mut buffer = [0; 512];
        let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
        let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        let mut token = flag.lock().unwrap();
        *token = true;
        println!("I have the token now :( Yalaaaaahwy");
        drop(token);
        thread::sleep(Duration::from_millis(5000 as u64));
        token = flag.lock().unwrap();
        *token = false;
        drop(token);
        println!("Released token now :)");
        token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}

  fn main() {

    //build el queue
    let mut servers: Queue<i32> = Queue::new();
    servers.enqueue(1);
    servers.enqueue(2);
    // servers.enqueue(3);

    let flag = Arc::new(Mutex::new(false));
    let flag_clone = Arc::clone(&flag);
    thread::spawn(move || token_handle(flag_clone));
  // launch a new thread (fun token )
    println!("Listening for peers on port 65432");
    let mut count = 0;
    loop {
        // Spawn a thread to listen for incoming messages
        let socket = UdpSocket::bind("0.0.0.0:65432").expect("Failed to bind socket");
        handle_regular_requests(&socket , &mut servers, Arc::clone(&flag));
    }
}