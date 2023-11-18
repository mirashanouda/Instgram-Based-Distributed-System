/**
 * Peer server:
 * recives the messages from the load balancer
 */

extern crate steganography;
extern crate photon_rs;
extern crate image;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use std::convert::TryInto; 
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use std::str;
use std::time::Duration;
use image::{open,ImageOutputFormat};
use image::ColorType;
use steganography::encoder::*;
use steganography::decoder::*;
use steganography::util::*;
use image::DynamicImage;
use photon_rs::PhotonImage;
use photon_rs::native::{image_to_bytes};
use std::io::Cursor;
use std::io::Read;
use std::io;
use std::io::BufReader;
use common_lib::queue::Queue;
use common_lib::utils;
 
 //edit ID for each server
 static ID: i32=2;
 
 const MAX_PACKET_SIZE: usize = 65535;
 const HEADER_SIZE: usize = 8; // Adjust according to your actual header size
 const END_OF_TRANSMISSION: usize = usize::MAX;
 const CHUNK_SIZE: usize = 508;
 const ACK_TIMEOUT: Duration = Duration::from_millis(500);
 
 fn send_image_to_client( client_addr: &SocketAddr, image_path: &str) -> io::Result<()> {
    let socket = UdpSocket::bind(format!("0.0.0.0:9877")).expect("Failed to bind socket");
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
             println!("{:?}",socket);
             match socket.recv_from(&mut ack_buffer) {
                 Ok(_) => {
                     let ack_seq_number = usize::from_be_bytes(ack_buffer.try_into().unwrap());
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
 
 fn receive_image(socket: &UdpSocket , src_addr: &SocketAddr)  {
     let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
     let mut buffer = [0u8; MAX_PACKET_SIZE];
     let mut end_of_transmission_received = false;
     let mut image_name = String::new();
     let mut first_packet = true;
 
     // Initialize as Option types outside the loop
     let mut size_opt: Option<usize> = None;
 
 
     while !end_of_transmission_received {
        println!("Waiting for data...");
         match socket.recv_from(&mut buffer) {
             Ok((size,_)) => {
                println!("Received data: {} bytes", size);
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
 
     
     let cover_image_path = "/home/mira/Distributed/Instgram-Based-Distributed-System/server_image/src/cover.png";
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
     let src= "127.0.0.1:9876".parse::<SocketAddr>().expect("Failed to parse server address");
     let values :i32 =55;
     let message=values.to_be_bytes();
     socket.send_to(&message, &src).expect("Failed to send Ready");
    // Send the encoded image to the client
    send_image_to_client(&src, &encoded_image_path).expect("Failed to send encoded image to client");
   
 }
 
 
 fn handle_incoming(socket: &UdpSocket,mut count : u32 , servers: &mut Queue<i32>) -> u32 {
     let mut buffer = [0; 512];
     loop {
        println!("{:?}",socket);
         let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
         println!("wrong recieve! {},{}",size,src_addr);
         let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
         println!("{}",message);
         if message == "Request"
         {
            println!("Request Recieved");
            //pop w add to end 
            if let Some((old, new_head)) = servers.dequeue(){  
            //if condtion 
            if new_head == Some(&ID) 
            { 
                println!("Server Selected");		
                count = count + 1;
                    // send back to the client
                    let ack_message =ID.to_be_bytes();
                    socket.send_to(&ack_message, &src_addr);
                    println!("ID sent {}",ID);
                    receive_image(&socket , &src_addr );
                }
            servers.enqueue(old);  
        }
 
    }
 }
 }

fn main() {

    let req_port = 65432;
    //build el queue 
    let mut servers: Queue<i32> = Queue::new();
    servers.enqueue(1);
    servers.enqueue(2);
    servers.enqueue(3);

    let flag = Arc::new(Mutex::new(false));
    let flag_clone = Arc::clone(&flag);
    println!("Listening for peers on port 65432");
    let mut count = 0;
    loop {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", req_port)).expect("Failed to bind socket");
        // Spawn a thread to listen for incoming messages
        count = handle_incoming(&socket,count , &mut servers);
    }
}