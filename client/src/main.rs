use std::net::{UdpSocket, SocketAddr};
use std::thread;
use std::time::Duration;

fn send_message_to_server(_thread_id: usize, server_address: &SocketAddr, msg: String, cnt: u32) {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");

    for i in 0..cnt {
        let message = format!("{} - i = {}", msg, i);
        socket.send_to(message.as_bytes(), server_address).expect("Failed to send message");
        thread::sleep(Duration::from_millis(200)); // Adjust this delay if needed
    }
}

fn main() {
    let server_addresses = vec![
        "127.0.0.1:1231",
        "127.0.0.1:1232",
        "127.0.0.1:1233",
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
                    send_message_to_server(i, &server_address, format!("thread {}", i), 50)
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
