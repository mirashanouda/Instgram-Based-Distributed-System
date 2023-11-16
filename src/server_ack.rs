use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
mod queue;
use queue::Queue;

static ID: i32 = 1;
//30, 32
//10.40.58.96:65432
static  OTHER_SERVERS : &str = "10.40.54.24:65432"; // List other server addresses here

fn handle_regular_requests(socket: &UdpSocket, servers: &mut Queue<i32>, flg: Arc<Mutex<bool>>) {
    let mut buffer = [0; 512];
   
    loop {
        let (size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        println!("yes\n");
        if let Some((old, new_head)) = servers.dequeue() {
            println!("First if\n");
            let mut token = flg.lock().unwrap();

            if new_head == Some(&ID) {
                println!("first\n");
                if !*token { // Case 1: Top of the queue and not offline
                    let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
                    println!("Handling request: {}", message);
                    let msg = "Ack";
                    socket.send_to(msg.as_bytes(), "127.0.0.1:65421").expect("Failed to send message");
                    thread::sleep(Duration::from_millis(5 as u64));
                    println!("Ack sent\n");

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
    loop{
      // recive
      let mut buffer = [0; 512];
      let token_socket = UdpSocket::bind("0.0.0.0:65430").expect("Failed to bind socket");
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
      let msg = "ball";
      let next_server: SocketAddr = "10.40.54.24:65430".parse().expect("Failed to parse server address");
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
    println!("Listening for peers on port 1234");
    let mut count = 0;
    // loop {
        // Spawn a thread to listen for incoming messages
        let socket = UdpSocket::bind("0.0.0.0:1234").expect("Failed to bind socket");
        handle_regular_requests(&socket , &mut servers, Arc::clone(&flag));
    // }
}