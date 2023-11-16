use std::net::{UdpSocket, SocketAddr};
use std::thread;
use std::str;
use std::io::{self, ErrorKind};
use std::time::{Duration, Instant};

fn send_message_to_server(thread_id: usize, server_address: &SocketAddr, msg: String, cnt: u32) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");

    for i in 0..cnt {
        let message = format!("{} - i = {}", msg, i);
        let timeout_duration = Duration::from_secs(30); // Total time to wait for an acknowledgment
        let start_time = Instant::now();

        loop {
            if Instant::now().duration_since(start_time) > timeout_duration {
                println!("Thread {}: No acknowledgment received within the timeout period, giving up.", thread_id);
                break;
            }

            socket.send_to(message.as_bytes(), server_address).expect("Failed to send message");

            let mut buf = [0; 1024];
           
            let socket_recieve = UdpSocket::bind("0.0.0.0:65421").expect("Failed to bind socket");
            socket_recieve.set_read_timeout(Some(Duration::from_millis(1000))).expect("Failed to set read timeout");

            match socket_recieve.recv_from(&mut buf) {
                Ok((_size, _src)) => {
                    println!("Thread {}: Acknowledgment received.", thread_id);
                    break;
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Timeout occurred, resend the message
                    println!("Thread {}: Acknowledgment not received, retrying...", thread_id);
                }
                Err(e) => {
                    eprintln!("Thread {}: Failed to receive data: {}", thread_id, e);
                    break;
                }
            }
        }

        thread::sleep(Duration::from_millis(5)); // Adjust this delay if needed
    }
}

fn main() {
    let server_addresses = vec![
        "10.40.54.24:65432",
        "127.0.0.1:1234",
    ]
    .into_iter()
    .map(|addr| addr.parse::<SocketAddr>().expect("Failed to parse server address"))
    .collect::<Vec<_>>();

    let mut server_handles = Vec::new();

    for address in server_addresses {
        let handle = thread::spawn(move || {
            let num_threads = 1;
            let mut client_handles = Vec::with_capacity(num_threads);

            for i in 0..num_threads {
                let server_address = address.clone();
                let client_handle = thread::spawn(move || {
                    send_message_to_server(i, &server_address, format!("Hello from thread {}", i), 2)
                });
                client_handles.push(client_handle);
            }

            for handle in client_handles {
                handle.join().unwrap();
            }

            // send_message_to_server(0, &address, "exit".to_string(), 1);
        });

        server_handles.push(handle);
    }

    for handle in server_handles {
        handle.join().unwrap();
    }
}
