use common_lib::utils;
use std::net::{SocketAddr, UdpSocket};
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;


static ID: i32 = 2;

//requests port: 1231 -> 1111
//offline ports: 2231 -> 2222
//token ports: 3231 -> 3333

static ONLINE_SERVERS: [&str; 2] = [
	"10.40.32.184:2222", 
	"10.40.44.118:2222"
];

static SERVERS: [&str; 3] = [
	"10.40.32.184", 
	"10.40.44.118",
	"10.40.45.33", 
];

fn failure_token_handle_sender(flag: Arc<Mutex<bool>>, next_token_add: &str) {
    let token_port = 3333;
    let next_server: SocketAddr = format!("{}:{}", next_token_add, token_port)
        .parse()
        .expect("Failed to parse server address");
    println!("{:?}", next_server);
    let token_socket =
        UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";
    token_socket
        .send_to(msg.as_bytes(), next_server)
        .expect("Failed to send message");
    thread::sleep(Duration::from_millis(10 as u64));

    loop {
        let mut buffer = [0; 512];
        let (size, _) = token_socket
            .recv_from(&mut buffer)
            .expect("Failed to receive message");
        let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        let mut token = flag.lock().unwrap();
        *token = true;
        drop(token);
        // println!("I have the token now :( Yalaaaaahwy");
        thread::sleep(Duration::from_millis(2000 as u64));
        token = flag.lock().unwrap();
        *token = false;
        drop(token);
        //println!("Released token now :)");
        token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}

fn working_token_handle_sender(
    off_server: Arc<Mutex<i32>>,
    work_flag: Arc<Mutex<bool>>,
    next_add: i32,
    next_next_add: i32,
    servers: [&str; 3],
) {
    println!("Working token initializer");
    let token_port = 6666;
    let next_server: SocketAddr = format!("{}:{}", servers[next_add as usize], token_port)
        .parse()
        .expect("Failed to parse server address");
    let token_socket =
        UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";
    // initial send.
    token_socket
        .send_to(msg.as_bytes(), next_server)
        .expect("Failed to send message");
    thread::sleep(Duration::from_millis(10 as u64));

    loop {
        let mut buffer = [0; 512];
        let (size, _) = token_socket
            .recv_from(&mut buffer)
            .expect("Failed to receive message");
        let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        let mut work_token = work_flag.lock().unwrap();
        *work_token = true;
        drop(work_token);
        println!("I am working now yaaaaay");
        thread::sleep(Duration::from_millis(1000 as u64));
        work_token = work_flag.lock().unwrap();
        *work_token = false;
        drop(work_token);
        println!("Released work token ^^)");

        // checking next server:
        let off_server_id = off_server.lock();
        // match off_server_id {
            // Ok(off_id) => {
                // if *off_id == next_add {
                //     token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_next_add as usize], token_port)).expect("Failed to send message");
                // } else {
                    token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_add as usize], token_port)).expect("Failed to send message");
                // }
            // }
            // Err(e) => {
            //     // Handle the error if lock cannot be acquired (e.g., if another thread has panicked while holding the lock)
            //     println!("Failed to acquire the lock: {:?}", e);
            // }
        // }
    }
}

fn main() {
    let requests_port = 4444;
    let next_server = 0;
    let next_next_server = 1;

    let mut handles = vec![];

    let off_flag = Arc::new(Mutex::new(false));
    let off_flag_clone = Arc::clone(&off_flag);
    // thread::spawn(move || {
    //     failure_token_handle_sender(off_flag_clone, SERVERS[next_server as usize])
    // });

    let off_server = Arc::new(Mutex::new(0));
    let off_server_clone = Arc::clone(&off_server);
    // launch a  thread for offline handler
    // thread::spawn(move || utils::who_offline_handler(off_server_clone, ID));

    let off_server_clone = Arc::clone(&off_server);
    let work_flag = Arc::new(Mutex::new(false));
    let work_flag_clone = Arc::clone(&work_flag);
    // thread::spawn(move || {
    //     working_token_handle_sender(
    //         off_server_clone,
    //         work_flag_clone,
    //         next_server as i32,
    //         next_next_server,
    //         SERVERS,
    //     )
    // });
    
    let dos_map: HashMap<String, bool> = HashMap::new();
    let dos_map = Arc::new(Mutex::new(dos_map));
    let dos_map_clone = Arc::clone(&dos_map);
    let handle1 = thread::spawn(move || {
        utils::store_in_dos(Arc::clone(&dos_map_clone))
    });
    handles.push(handle1);

    let handle2 = thread::spawn(move || {
        utils::send_who_online(Arc::clone(&dos_map))
    });
    handles.push(handle2);

    println!("Listening for requests on port {}", requests_port);
    // loop {
    //     let socket =
    //         UdpSocket::bind(format!("0.0.0.0:{}", requests_port)).expect("Failed to bind socket");
    //     // println!("socket = {:?}", socket);
    //     utils::handle_requests_v2(
    //         &socket,
    //         ONLINE_SERVERS,
    //         Arc::clone(&work_flag),
    //         Arc::clone(&off_flag),
    //         ID,
    //         SERVERS,
    //     );
    // }
    for handle in handles {
        handle.join().unwrap();
    }
}
