extern crate steganography;
extern crate photon_rs;
extern crate image;
use std::fs::File;
use std::collections::HashMap;
use std::convert::TryInto; 
use std::net::{UdpSocket, SocketAddr};
use std::str;
use image::ImageBuffer;
use image::DynamicImage;
use std::time::Duration;
use steganography::decoder::*;
use steganography::util::*;
use image::Rgba;
use std::thread;
use std::io::{self, Read, Write};
use steganography::util::file_as_image_buffer;
use image::{open,ImageOutputFormat};
use std::io::Cursor;
use image::load_from_memory;



const CHUNK_SIZE: usize = 65008;
const HEADER_SIZE: usize = 8; // Assuming usize is 8 bytes (on a 64-bit system)
const END_OF_TRANSMISSION: usize = usize::MAX;
const ACK_TIMEOUT: Duration = Duration::from_millis(1000);

fn receive_image_from_server(socket: &UdpSocket, img_name: &str ) -> io::Result<()> {
    let path = format!("encrypted_{}",img_name);
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

    let mut file = File::create(path.clone())?;
    file.write_all(&complete_image)?;
    println!("Image completed!!!!!!!!!!!!!");
    
   //START DECODING
   
   let encoded_image = file_as_image_buffer(path);
   let dec = Decoder::new(encoded_image);
   println!("Sucess to decode");
   let out_buffer =dec.decode_alpha();
   println!("Done Decoder");
   //let clean_buffer:Vec<u8>=out_buffer.into_iter().filter(|b|{*b!=0xff_u8}).collect();
   let shared_image = image::ImageRgba8(image::RgbaImage::from_raw(3024, 4032,out_buffer).expect("Failed to create image from raw data"));
  

   let de_path = format!("decrypted_{}",img_name);
   shared_image.save(de_path)?;
   /*
   //load cover image 
     /*let cover_image_path = "/home/dahab/Instgram-Based-Distributed-System/client_image/src/cover.jpg";
     let cover_image = match open(cover_image_path) {
         Ok(img) => img,
         Err(e) => {
         eprintln!("Failed to open cover image at '{}': {:?}", cover_image_path, e);
         return; // Or handle the error as appropriate
         }
     };*/
   
   let cover_image_path = "/home/dahab/Instgram-Based-Distributed-System/client_image/src/cover.jpg";
    let cover_image: DynamicImage = open(cover_image_path).expect("Secret image not found");
     let mut cover_image_bytes = Cursor::new(Vec::new());
     cover_image.write_to(&mut cover_image_bytes, ImageOutputFormat::PNG).expect("Failed to write encoded image to bytes");
     let cover_image_bytes = cover_image_bytes.into_inner();
 
    
     // Load the encoded image as DynamicImage
    
    let encoded_image_dynamic: DynamicImage = image::open(path).expect("Failed to open the encoded image");

    // Convert DynamicImage to ImageBuffer<Rgba<u8>, Vec<u8>>
    let encoded_image: ImageBuffer<Rgba<u8>, Vec<u8>> = encoded_image_dynamic.to_rgba();

 
    // Now create a decoder instance with the correct type
    let decoder = Decoder::new(encoded_image);//,cover_image_bytes);

    // Step 3: Decode the hidden image
    let decoded_data = decoder.decode_alpha(); // This will depend on your decoder's implementation

    // Step 4: Reconstruct the secret image
    // You need to know the dimensions of the secret image or find a way to determine them
    let secret_image = image::ImageRgba8(image::RgbaImage::from_raw(3024, 4032, decoded_data).expect("Failed to create image from raw data"));

    // Save the secret image
    let de_path = format!("decrypted_{}",img_name);
    secret_image.save(de_path).expect("Failed to save the secret image");
    */

  
    Ok(()) 
    
}

   
fn send_image_to_server(server_address: &SocketAddr, image_path: &str ,socket: &UdpSocket) -> io::Result<()> {
	// Extract the image name from the path
    let image_name = image_path.rsplit('/').next().unwrap_or(image_path);

    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    socket.set_read_timeout(Some(ACK_TIMEOUT))?;

    // Send the image name first
    let mut name_packet = Vec::with_capacity(HEADER_SIZE + image_name.len());
    name_packet.extend_from_slice(&(usize::MAX - 1).to_be_bytes()); // Special sequence number for the name
    name_packet.extend_from_slice(image_name.as_bytes());

    socket.send_to(&name_packet, server_address)?;
    // Wait for ACK for the name packet here (similar to how you wait for ACKs below)

    //Start of sending the image itself
    // println!("Sending Image");
    let mut file = File::open(image_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    /*let socket = UdpSocket::bind("0.0.0.0:65432")?;
    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    socket.set_read_timeout(Some(ACK_TIMEOUT))?;*/

    for (i, chunk) in buffer.chunks(CHUNK_SIZE).enumerate() {
        let mut packet = Vec::with_capacity(HEADER_SIZE + chunk.len());
        packet.extend_from_slice(&i.to_be_bytes()); // Add sequence number as header
        packet.extend_from_slice(chunk);
        // println!("Done chunking");
        loop {
            // println!("start ACK");
            socket.send_to(&packet, server_address)?;
            let mut ack_buffer = [0; HEADER_SIZE];
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    // println!("ACK");
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
    // println!("Done ACK");
    // Send end-of-transmission notification
    let mut eot_packet = Vec::with_capacity(HEADER_SIZE);
    eot_packet.extend_from_slice(&END_OF_TRANSMISSION.to_be_bytes());
    eot_packet.extend(vec![0; HEADER_SIZE - eot_packet.len()]);
    socket.send_to(&eot_packet, server_address)?;
    //println!("Done");
    Ok(())
}

fn select_server(address: &str,socket: &UdpSocket , image_path : &str) {	 

	let server_address_str = address;
         match server_address_str.parse::<SocketAddr>() {
         Ok(server_address) => {
            //let image_path = "image1.jpg";
            if let Err(e) = send_image_to_server(&server_address, image_path, socket) {
                eprintln!("Failed to send image: {}", e);
            }
        },
        Err(_) => {
            eprintln!("Invalid server address: {}", server_address_str);
        }
    }
}
       
       
fn recive_ack(socket: &UdpSocket,ser_list: &Vec<&str> , image_path : &str) {
	let mut ack_buffer = [0;4];
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    let ack_id_number = u32::from_be_bytes(ack_buffer);
                    let ack_id_number_usize = ack_id_number as usize;
                        println!("recieved ID = {}",ack_id_number_usize);
                        select_server(ser_list[ack_id_number_usize],socket,&image_path); //send the socket ?
            	},
                Err(e) => return (), // Some other error
           }
}     

fn recive_ready(socket: &UdpSocket,image_name:&str,i: &u32) {
    let mut ack_buffer=[0;4];
    loop {
	
        let (size, _) = socket.recv_from(&mut ack_buffer).expect("Failed to receive message");
            let message = u32::from_be_bytes(ack_buffer);
            let message_usize  = message as usize;
            if message_usize == 55
            {
            // println!("Recived 55");
            //let output_img =  format!("encrypted_{}", image_name);
            if let Err(e) = receive_image_from_server(&socket, &image_name) 
            {
                println!("Failed to receive image: {}", e);
            }
            break;

            }
            else {println!("Received something else ");}

   
        }

}
             
fn main() {
    let ser_list = vec![
        "10.40.32.184:65432",
        //"10.40.44.218:4444",
         //"10.40.45.33:4444",
    ];

    let server_addresses = vec![
        "10.40.32.184:65432",
        //"10.40.44.218:4444",
        //"10.40.45.33:4444",
    ]
    .into_iter()
    .map(|addr| addr.parse::<SocketAddr>().expect("Failed to parse server address"))
    .collect::<Vec<_>>();
    
    //list of path to image 
    let req_port = 65432;
    let img_port = req_port+1;
    let socket_req = UdpSocket::bind(format!("0.0.0.0:{}", req_port)).expect("Failed to bind socket");
    for i in 0..1 {
    
    	let image_name = format!("image{}.jpg",i);
        for address in &server_addresses {
            // Directly call the function here without spawning a new thread
            utils::send_message(&socket_req,&address,"Request".to_string());   
        }
        recive_ack(&socket_req, &ser_list ,&image_name);
        let socket_img = UdpSocket::bind(format!("0.0.0.0:{}", img_port)).expect("Failed to bind socket");
        recive_ready(&socket_img,&image_name,&i);

                // Set the duration of the delay (e.g., 5 seconds)
            let delay_duration = Duration::from_secs(5);

            // Pause the execution of the current thread for the specified duration
            thread::sleep(delay_duration);
    }   
}