use crate::queue::Queue;

extern crate steganography;
extern crate photon_rs;
extern crate image;

use std::net::{UdpSocket, SocketAddr};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use std::convert::TryInto; 
use image::{open,ImageOutputFormat};
use steganography::encoder::*;
use image::DynamicImage;
use std::io::Cursor;
use std::io::Read;
use std::io;
use std::io::BufReader;

const MAX_PACKET_SIZE: usize = 65535;
const HEADER_SIZE: usize = 8; // Adjust according to your actual header size
const END_OF_TRANSMISSION: usize = usize::MAX;
const CHUNK_SIZE: usize = 508;
const ACK_TIMEOUT: Duration = Duration::from_millis(500);

//requests port: 1111
// img port: 1112
//offline ports: 2222
//token ports: 3333

// TODO:
// vec of servers IPs to be shared globaly --> all 3 IPs

pub fn failure_token_handle(flag: Arc<Mutex<bool>>, next_token_add: &str){
	let token_port = 3333;
	let next_server: SocketAddr = format!("{}:{}", next_token_add, token_port).parse().expect("Failed to parse server address");
    let token_socket = UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";
    
    loop{
		let mut buffer = [0; 512];
		let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
		let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
		let mut token: std::sync::MutexGuard<'_, bool> = flag.lock().unwrap();
		*token = true;
		drop(token);
		println!("I have the token now :( Yalaaaaahwy");
		thread::sleep(Duration::from_secs(2 as u64));
		token = flag.lock().unwrap();
		*token = false;
		drop(token);
		println!("Released token now :)");
		token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}

pub fn working_token_handle(off_server: Arc<Mutex<i32>>, work_flag: Arc<Mutex<bool>>, next_add: i32, next_next_add: i32, servers: [&str; 3]){
    /*
    * if received a request.
    * if online server:
        * check if I have token (I should receive a token before)
            * handle_incomming
            if the next is not offline:
                * pass to it
            else:
                * pass to the next
        * if no token:
            * recieve token (listening)
    */

    let token_port = 3333;
    let token_socket = UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";

    loop{
        let mut buffer = [0; 512];
        let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
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
        match off_server_id {
            Ok(off_id) => {
                if *off_id == next_add {
                    token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_next_add as usize], token_port)).expect("Failed to send message");
                }
                else {
                    token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_add as usize], token_port)).expect("Failed to send message");
                }
            }
            Err(e) => {
                // Handle the error if lock cannot be acquired (e.g., if another thread has panicked while holding the lock)
                println!("Failed to acquire the lock: {:?}", e);
            }
        }
    }
}

pub fn send_offline (off_server: &str, online_servers: [&str; 2]){
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
	// send to all servers my current status (offline/online)
	for server in online_servers {
		let server_add: SocketAddr = server.parse().expect("Failed to parse server address");
		socket.send_to(off_server.as_bytes(), server_add).unwrap();
		thread::sleep(Duration::from_millis(5));
	}
}

pub fn who_offline_handler(off_server: Arc<Mutex<i32>> , id: i32) {
    let offline_port = 2222;
    let offline_add = format!("0.0.0.0:{}", offline_port);
    let socket = UdpSocket::bind(offline_add).expect("Failed to bind socket");
    // listen for messages from other clients and print them out
    loop{
        let mut buffer = [0; 4];
		socket.recv_from(&mut buffer).expect("Failed to receive message");
        let mut off_server_id = off_server.lock().unwrap();
        *off_server_id = i32::from_be_bytes(buffer);
        // println!("Sleep server is: {}", off_server_id);
    }
}

pub fn send_image_to_client( client_addr: &SocketAddr, image_path: &str) -> io::Result<()> {
    println!(" client socket = {}", format!("{}:{}", client_addr.ip(), client_addr.port()));

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", client_addr.port())).expect("Failed to bind socket");
    
    let file = File::open(image_path)?;
    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();
    buf_reader.read_to_end(&mut buffer)?;

    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    socket.set_read_timeout(Some(ACK_TIMEOUT))?;

    for (i, chunk) in buffer.chunks(CHUNK_SIZE).enumerate() {
        let mut packet = Vec::with_capacity(HEADER_SIZE + chunk.len());
        packet.extend_from_slice(&i.to_be_bytes()); // Add sequence number as header
        packet.extend_from_slice(chunk);

        loop {
            socket.send_to(&packet, client_addr)?;
            let mut ack_buffer = [0; HEADER_SIZE];
            //  println!("{:?}",socket);
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    let ack_seq_number = usize::from_be_bytes(ack_buffer.try_into().unwrap());
                    //  println!("Ach seq = {}", ack_seq_number);
                    if ack_seq_number == i {
                        break; // Correct ACK received, proceed to next chunk
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Timeout; ACK not received, resend the packet
                    continue;
                }
                Err(e) => return Err(e), // Some other error
            }
        }
    }

    // Send end-of-transmission notification
    let mut eot_packet = Vec::with_capacity(HEADER_SIZE);
    eot_packet.extend_from_slice(&END_OF_TRANSMISSION.to_be_bytes());
    socket.send_to(&eot_packet, client_addr)?;

    let mut buffer = [0; 512];
    let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");

    Ok(())
}

pub fn receive_image(socket: &UdpSocket , src_addr: &SocketAddr)  {
    let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    let mut end_of_transmission_received = false;
    let mut image_name = String::new();
    let mut first_packet = true;

    // Initialize as Option types outside the loop
    let mut size_opt: Option<usize> = None;


    while !end_of_transmission_received {
        // println!("Waiting for data...");
        match socket.recv_from(&mut buffer) {
            Ok((size,_)) => {
                // println!("Received data: {} bytes", size);
                size_opt = Some(size);

                // Check if the size of received data is less than HEADER_SIZE
                if size < HEADER_SIZE {
                    println!("Data size less than HEADER_SIZE, continuing...");
                    continue;
                }

                let sequence_number = match buffer[..HEADER_SIZE].try_into() {
                    Ok(bytes) => usize::from_be_bytes(bytes),
                    Err(e) => {
                        eprintln!("Failed to convert header bytes: {}", e);
                        continue;
                    }
                };

                // Send ACK for the received chunk
                let ack_packet = sequence_number.to_be_bytes();
                if let Err(e) = socket.send_to(&ack_packet, &src_addr) {
                    eprintln!("Failed to send ACK: {}", e);
                    continue;
                }

                if sequence_number == END_OF_TRANSMISSION {
                    end_of_transmission_received = true;
                } else if first_packet && sequence_number == usize::MAX - 1 {
                    image_name = String::from_utf8_lossy(&buffer[HEADER_SIZE..size]).to_string();
                    first_packet = false;
                } else {
                    let image_data = &buffer[HEADER_SIZE..size];
                    file_storage.insert(sequence_number, image_data.to_vec());
                }
            },
            Err(e) => {
                eprintln!("Failed to receive data: {}", e);
                continue;
            }
        };
    }

    // Reassemble the image
    let mut complete_image = Vec::new();
    for i in 0..file_storage.len() {
        if let Some(chunk) = file_storage.remove(&i) {
            complete_image.extend_from_slice(&chunk);
        }
    }
    
    // Write the complete image to a file
    // Assuming image_name contains something like "image1"
    let output_file_name = format!("output_{}", image_name);

    // Write the complete image to a file with the new name
    let mut file = File::create(&output_file_name).expect("Failed to create file");
    file.write_all(&complete_image).expect("Failed to write to file");

    // Load the secret image you want to hide
    let secret_image: DynamicImage = open(&output_file_name).expect("Secret image not found");

    
    let cover_image_path = "/home/mira/Distributed/Instgram-Based-Distributed-System/common_lib/src/cover.png";
    let cover_image = match open(cover_image_path) {
        Ok(img) => img,
        Err(e) => {
        eprintln!("Failed to open cover image at '{}': {:?}", cover_image_path, e);
        return; // Or handle the error as appropriate
        }
    };
    // Convert the secret image to bytes
    let mut secret_image_bytes = Cursor::new(Vec::new());
    secret_image.write_to(&mut secret_image_bytes, ImageOutputFormat::PNG).expect("Failed to write secret image to bytes");
    let secret_image_bytes = secret_image_bytes.into_inner();

    // Create an Encoder instance
    let encoder = Encoder::new(&secret_image_bytes, cover_image);

    // Encode the secret into the cover image
    let encoded_image = encoder.encode_alpha(); // Adjust this according to the actual encode method signature
    
    // Get the dimensions of the image and save the encoded image
    let (width, height) = encoded_image.dimensions();
    let img = DynamicImage::ImageRgba8(image::RgbaImage::from_raw(width, height, encoded_image.to_vec()).unwrap());

        // Assuming image_name is a String
        let encoded_image_path = format!("encoded_{}", image_name);
        // Save and process the image

    img.save(&encoded_image_path).expect("Failed to save the image");
    println!("done Encoding \n");

    let src= format!("{}:{}", src_addr.ip(), src_addr.port() + 1).parse::<SocketAddr>().expect("Failed to parse server address");
    //  println!("{}", format!("{}:{}", src_addr.ip(), src_addr.port() + 1));
    
    let values :i32 = 55;
    let message = values.to_be_bytes();
    socket.send_to(&message, &src).expect("Failed to send Ready");
    // Send the encoded image to the client
    send_image_to_client(&src, &encoded_image_path).expect("Failed to send encoded image to client");

}


// pub fn handle_regular_requests(
//     socket: &UdpSocket,
//     servers: &mut Queue<i32>,
//     flg: Arc<Mutex<bool>>,
//     off_server: Arc<Mutex<i32>>,
//     id: i32,
//     online_servers: [&str; 2])
// {
//     let mut buffer = [0; 512];
//     // let mut size;
//     let mut handled: bool = true;
//     let mut message = String::from("");
//     loop {
//         // if handled {
//         // deque from requests buff
//         let (size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
//         message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
//         // }
//         if let Some((old, new_head)) = servers.dequeue() {
//         let token = flg.lock().unwrap();

//             if new_head == Some(&id) {
//                 // println!("I am top {}", id);
//                 // Case 1: Top of the queue and not offline
//                 if *token == false {
//                     println!("Handling request: {}", message);
//                     handled = true;
//                 }
//                 // Case 2: Top of the queue and Offline
//                 else {
//                     println!("Offline :( at req - {}", message);
//                     send_offline(SERVERS[id as usize], online_servers);
//                     handled = false;
//                     // thread::sleep(Duration::from_secs(2 as u64));
//                                         // break;
//                     // utils::send_status(format!("online - {}", ID), ONLINE_SERVERS)
//                 }
//             }
//             else {
//                 let off_id = off_server.lock().unwrap();
//                 match new_head {
//                     Some(head) => {
//                     // Case 3: not top of the queue and online
//                     if *off_id == *head {
//                         handled = false;
//                     // println!("Not top!")
//                     }
//                     else {
//                         handled = true;
//                     }
//                 } None => todo!()
//                 }
//             }
//             servers.enqueue(old);
//             drop(token);
//         }
//     }
// }

pub fn handle_requests_v2(
    socket: &UdpSocket,
    online_servers: [&str; 2],
    work_flg: Arc<Mutex<bool>>,
    offline_flg: Arc<Mutex<bool>>,
    id: i32,
    servers: [&str; 3])
{
    let mut buffer = [0; 512];
   
    let mut message = String::from("");
    loop {
       
        let (size, _src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        
        let work_token = work_flg.lock().unwrap();
        let offline_token = offline_flg.lock().unwrap();
        if *work_token == true {
            println!("Handling request: {}", message);
        }
        else if *offline_token == true{
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
            send_offline (servers[id as usize], online_servers);
        }
        drop(work_token);
        drop(offline_token);
    }
}

