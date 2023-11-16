use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
mod queue;
use queue::Queue;

static ID: i32 = 1;

static  OTHER_SERVERS : &str = "10.7.57.199:65432"; // List other server addresses here

fn handle_regular_requests(socket: &UdpSocket, servers: &mut Queue<i32>, flg: Arc<Mutex<bool>>) {
    let mut buffer = [0; 512];

    loop {
        let (size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");

        if let Some((old, new_head)) = servers.dequeue() {
            let mut token = flg.lock().unwrap();

            if new_head == Some(&ID) {
                if !*token { // Case 1: Top of the queue and not offline
                    let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
                    println!("Handling request: {}", message);
   
                } else { // Case 2: Offline
                    notify_servers("I am offline");
                    thread::sleep(Duration::from_secs(5)); // Example sleep duration
                    *token = false; // Mark server as back online
                    let next_server = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to a random port");
                                        

                }
            } else {
                // Case 3: Not top of the queue and not offline
                let off_socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to a random port");
                let mut buf = [0; 512];
                if let Ok(msg) = str::from_utf8(&buf[..size]) {
                    if msg == "I am offline" {
                        println!("yes\n");
                        pass_request_to_next_server(socket, servers, &buffer, size);
                    }
                }
            }

            servers.enqueue(old);
            drop(token);
        }
    }
}

fn notify_servers(message: &str) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to a random port");
    //for server in OTHER_SERVERS.iter() {
        socket.send_to(message.as_bytes(), OTHER_SERVERS).expect("Failed to send message");
//    }
}

fn pass_request_to_next_server(socket: &UdpSocket, servers: &mut Queue<i32>, buffer: &[u8], size: usize) {
    if let Some((_old, new_head)) = servers.dequeue() {
        if let Some(&next_server_id) = new_head {
            let next_server_address = format!("10.7.57.{}:65432", next_server_id);
            socket.send_to(&buffer[..size], &next_server_address).expect("Failed to send message to next server");
        }
    }
}


fn token_handle(flag: Arc<Mutex<bool>>){
    loop{
      // recive
      let mut buffer = [0; 512];
      let token_socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
      let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
      let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
      let mut token = flag.lock().unwrap();
      *token = true;
      println!("I have the token now :( Yalaaaaahwy");
      thread::sleep(Duration::from_millis(2000 as u64));
      token = flag.lock().unwrap();
      *token = false;
      println!("Released token now :)");
      let msg = "ball";
      let next_server: SocketAddr = "10.7.57.199:65432".parse().expect("Failed to parse server address");
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