extern crate steganography;
extern crate photon_rs;
extern crate image;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use std::convert::TryInto; 
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::str;
use std::time::Duration;
use image::{open,ImageOutputFormat, GenericImageView, DynamicImage};
use steganography::encoder::*;
use std::io::Cursor;
use std::io::Read;
use std::io;
use std::io::BufReader;
use std::thread;
use image::imageops::FilterType;
use std::process::Command;

const MAX_PACKET_SIZE: usize = 1000000;
const HEADER_SIZE: usize = 8; // Adjust according to your actual header size
const END_OF_TRANSMISSION: usize = usize::MAX;
const CHUNK_SIZE: usize = 65008;
const ACK_TIMEOUT: Duration = Duration::from_millis(500);


// This
pub fn send_image_to_client( client_addr: &SocketAddr, image_path: &str) -> io::Result<()> {
  // println!(" client socket = {}", format!("{}:{}", client_addr.ip(), client_addr.port()));
   
   let socket = UdpSocket::bind(format!("0.0.0.0:{}", client_addr.port())).expect("Failed to bind socket");
   
    let file = File::open(image_path)?;
    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();
    buf_reader.read_to_end(&mut buffer)?;

    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    socket.set_read_timeout(Some(ACK_TIMEOUT))?;

    for (i, chunk) in buffer.chunks(CHUNK_SIZE).enumerate() 
    {
        let mut packet = Vec::with_capacity(HEADER_SIZE + chunk.len());
        packet.extend_from_slice(&i.to_be_bytes()); // Add sequence number as header
        packet.extend_from_slice(chunk);

        //println!("{:?}",socket);

        loop {
            socket.send_to(&packet, client_addr)?;
            let mut ack_buffer = [0; HEADER_SIZE];
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    let ack_seq_number = usize::from_be_bytes(ack_buffer.try_into().unwrap());
                    //println!("Ach seq = {}", ack_seq_number);
                    if ack_seq_number == i {
                               //println!("Number == i");
                            break; // Correct ACK received, proceed to next chunk
                            }
                     }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    // Timeout; ACK not received, resend the packet
                                    //println!("Number not recieved ");
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

   //  let mut buffer = [0; 512];
   //  let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
    println!("Done Sending");
    Ok(())
}

pub fn receive_image_to_encode(socket: &UdpSocket , src_addr: &SocketAddr)  {
    let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    let mut end_of_transmission_received = false;
    let mut image_name = String::new();
    let mut first_packet = true;

    while !end_of_transmission_received {
       // println!("Waiting for data...");
        match socket.recv_from(&mut buffer) {
            Ok((size,_)) => {
               //println!("Received data: {} bytes", size);
                // Check if the size of received data is less than HEADER_SIZE
               //  if size < HEADER_SIZE {
               //     println!("Data size less than HEADER_SIZE, continuing...");
               //      continue;
               //  }

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

   println!("Image Saved");

    // Load the secret image you want to hide
    let secret_image: DynamicImage = open(&output_file_name).expect("Secret image not found");
    //println!("Secret Image Uploaded");
    
    let cover_image_path = "cover.jpg";
    let cover_image = match open(cover_image_path) {
        Ok(img) => img,
        Err(e) => {
        eprintln!("Failed to open cover image at '{}': {:?}", cover_image_path, e);
        return; // Or handle the error as appropriate
        }
    };

    //println!("Cover Image Uploaded");
    // Convert the secret image to bytes
    let mut secret_image_bytes = Cursor::new(Vec::new());
    secret_image.write_to(&mut secret_image_bytes, ImageOutputFormat::PNG).expect("Failed to write secret image to bytes");
    let secret_image_bytes = secret_image_bytes.into_inner();

    //println!("Secret Image Fragmented");
    // Create an Encoder instance
    let encoder = Encoder::new(&secret_image_bytes, cover_image);
    //println!("Encoder created");
    // Encode the secret into the cover image
    let encoded_image = encoder.encode_alpha(); // Adjust this according to the actual encode method signature
    //println!("Encoder image created");
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

// This
pub fn receive_image (socket: &UdpSocket, out_name: &str) {
    let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut buffer = [0u8; CHUNK_SIZE + HEADER_SIZE];
    let mut end_of_transmission_received = false;
    let mut src_img : SocketAddr;
	while !end_of_transmission_received {
        match socket.recv_from(&mut buffer) {
            Ok((size, src_img)) => {
            let sequence_number = match buffer[..HEADER_SIZE].try_into() {
                Ok(bytes) => usize::from_be_bytes(bytes),
                Err(e) => {
                    eprintln!("Failed to convert header bytes: {}", e);
                    continue;
                }
            };
            
            if sequence_number == END_OF_TRANSMISSION {
                end_of_transmission_received = true;
            } else {
                let image_data = &buffer[HEADER_SIZE..size];
                file_storage.insert(sequence_number, image_data.to_vec());
            }
            
            // Send ACK for the received chunk
            let ack_packet = sequence_number.to_be_bytes();
            if let Err(e) = socket.send_to(&ack_packet,&src_img) {
                eprintln!("Failed to send ACK: {}", e);
                continue;
            }
		},
		Err(e) => {
            eprintln!("Failed to receive data: {}", e);
		    continue; // Continue the loop even if there's an error
		}
    };
}

    println!("Done receiving");

    // Reassemble and save the image as before
    let mut complete_image = Vec::new();
    for i in 0..file_storage.len() {
        if let Some(chunk) = file_storage.remove(&i) {
            complete_image.extend_from_slice(&chunk);
        }
    }

    let mut file = File::create(out_name).unwrap();
    file.write_all(&complete_image).unwrap();
    println!("Image completed!!!!!!!!!!!!!");
}

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
		//println!("I have the token now :( Yalaaaaahwy");
		thread::sleep(Duration::from_secs(2 as u64));
		token = flag.lock().unwrap();
		*token = false;
		drop(token);
		//println!("Released token now :)");
		token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}

pub fn working_token_handle(off_server: Arc<Mutex<i32>>, work_flag: Arc<Mutex<bool>>, next_add: i32, next_next_add: i32, servers: [&str; 3]){
    let token_port = 6666;
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
        thread::sleep(Duration::from_millis(500 as u64));
        work_token = work_flag.lock().unwrap();
        *work_token = false;
        drop(work_token);
        //println!("Released work token ^^)");

        // checking next server:
        // let off_server_id = off_server.lock();
        // match off_server_id {
        //     Ok(off_id) => {
        //         if *off_id == next_add {
                    // token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_next_add as usize], token_port)).expect("Failed to send message");
        //         }
        //         else {
                    token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_add as usize], token_port)).expect("Failed to send message");
        //         }
        //     }
        //     Err(e) => {
        //         // Handle the error if lock cannot be acquired (e.g., if another thread has panicked while holding the lock)
        //         println!("Failed to acquire the lock: {:?}", e);
        //     }
        // }
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

pub fn who_offline_handler(off_server: Arc<Mutex<i32>>) {
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
        let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        if message == "Request" {
            println!("received message!");
            let work_token = work_flg.lock().unwrap();
            let offline_token = offline_flg.lock().unwrap();
            if *work_token == true {
                drop(work_token);
                // send back to the client
                let ack_message =(id).to_be_bytes();
                let _ = socket.send_to(&ack_message, &src_addr);
                println!("ID sent {}",id);
                receive_image_to_encode(&socket , &src_addr );
                // println!("Handling request: {}", message);
            }
            // else if *offline_token == true{
            //     drop(offline_token);
            //     println!("Offline at {}",message);
            //     send_offline (servers[id as usize], online_servers);
            // }
            else {
                println!("not working");
                continue;
            }
        }
    }
}

// from client to server
pub fn register_client_in_dos(ip: &str, servers: [&str; 3]){
    let dos_port = 7777;
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind dos socket");
    for server in servers {
		let server_add: SocketAddr = format!("{}:{}", server, dos_port).parse().expect("Failed to parse server address");
		socket.send_to(ip.as_bytes(), server_add).unwrap();
		thread::sleep(Duration::from_millis(5));
	}
}

// client asks server who's online:
pub fn whos_online(servers: [&str; 3]) -> Vec<String> {
    let port = 8888;
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    let msg = "Who online";
    for server in servers {
		let server_add: SocketAddr = format!("{}:{}", server, port).parse().expect("Failed to parse server address");
		socket.send_to(msg.as_bytes(), server_add).unwrap();
		thread::sleep(Duration::from_millis(5));
	}

    let mut buffer = [0; 512];    
    let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
    let ips = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
    let ips_vec: Vec<String> = ips.split('_')
                                  .map(|s| s.to_string())
                                  .collect();
    println!("{:?}", ips_vec);
    return ips_vec;
}

// server stores online clinets
pub fn store_in_dos (dos_map: Arc<Mutex<HashMap<String, bool>>>){
    let dos_port = 7777;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", dos_port)).expect("Failed to bind dos socket");
    loop {
        let mut buffer = [0; 512];
        let (size, _addr) = socket.recv_from(&mut buffer).expect("Did not correctly receive data");
        let ip = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        let mut dos_map = dos_map.lock().unwrap();        
        println!("Stored new IP {}", ip);
        dos_map.insert(ip, true);
    }
}

// server respond who's online
pub fn send_who_online(dos_map: Arc<Mutex<HashMap<String, bool>>>){
    let dos_port: i32 = 8888;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", dos_port)).expect("Failed to bind dos socket");
    loop {
        let mut buffer = [0; 512];
        let (size,addr) = socket.recv_from(&mut buffer).expect("Did not correctly receive data");
        let msg = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        if msg == "Who online".to_string() {
            let sender_addr: Vec<String>  = addr.to_string().split(':').map(|s| s.to_string()).collect();
            let sender_ip = sender_addr.get(0);
            let mut conc_ips = String::new();
            let mut dos_map = dos_map.lock().unwrap();        
 
            // loop over the map and concatinate ips 
            for (key, value) in dos_map.iter_mut(){
                // let def : &String;
                if let Some(ip) = sender_ip {
                    if ip != key {
                        if *value == true {
                            conc_ips.push_str(key);
                            conc_ips.push_str("_");
                        }
                    }
                }
            }
            socket.send_to(conc_ips.as_bytes(), addr).expect("Failed to send IPs");
        }
    }
}

// This
pub fn send_message(socket: &UdpSocket, target_address: &SocketAddr,msg: &str) {
    let message = format!("{}",msg);
    socket.send_to(message.as_bytes(), target_address).expect("Failed to send message");
}

pub fn convert_to_low_resolution(image_name: &str, new_width: u32, new_height: u32) {
    println!("converting {}", image_name);
    // Load the image
    let img = image::open(image_name).unwrap();
    // Resize the image to a lower resolution
    let resized = img.resize_exact(new_width, new_height, FilterType::Nearest);
    let output_file_name = format!("low_res_{}", image_name);
    resized.save(output_file_name).unwrap();
}

// client 1 (requester)
// 1. send msg to view imags --> "view all" 
//      listening for recieving low res
//      choose img ID to view
//      send ID to client
//      listen for number of views
//      listen for recieve of high res
//      recive it and save encrypted
//      prompt to view image or send another client.
//          view image:
// ...........................................................
//              check num of views -- decrypt -- view image
//          send another client -- continue
// and loop: 
//      client to view the image --> enter v
//      each time to have v --> decrement

// source IP client --> pair (img ID, number of views)

// client 2
// 2. listing for "view all"
//      send low resolution images
//      listening for ID
//      send number of views
//      send encoded img
//      send image

//Those 
// support multithreading
pub fn send_my_img () {
    let imgs_port: i32 = 9999;
    let mut buffer = [0; 512];
    println!("Waiting for requests");

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", imgs_port)).expect("Failed to bind dos socket");
	let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
	let view_msg = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();

    if view_msg == "view all".to_string() {
        println!("view request from {}!", src_addr);
        // send dynamic port to handle comminication
        for i in 0..4 {
            let image_name = format!("low_res_image{}.jpg", i);
            let _ = send_image_to_client(&src_addr, &image_name);
            thread::sleep(Duration::from_millis(200));
        }
        // waiting for other client to choose image:
        let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        let id = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        // TODO: change to encoded image.
        let image_name = format!("image{}.jpg", id);
        let views = "3";
        socket.send_to(&views.as_bytes(), src_addr).expect("Failed to send image views");
        let _ = send_image_to_client(&src_addr, &image_name);
    }
}

pub fn request_img_from_client (friend_addr: SocketAddr) {
    let socket = UdpSocket::bind(format!("0.0.0.0:0")).expect("Failed to bind dos socket");
    let msg = "view all";
    send_message(&socket, &friend_addr, msg);

    for i in 0..4 {
        let img_name = format!("image{}.jpg", i);
        receive_image (&socket, &format!("encrypted_{}", img_name));
    }

    let id = "2";
    let _ = socket.send_to(&id.as_bytes(), friend_addr);
    
    let mut buffer = [0; 512]; 
    let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
    // TODO: convert to mutex
    let view_msg = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
    let img_name = "final_image.jpg";
    receive_image (&socket, &img_name);
    thread::sleep(Duration::from_millis(500));
}

