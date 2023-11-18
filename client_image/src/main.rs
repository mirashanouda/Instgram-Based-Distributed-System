use std::net::{UdpSocket, SocketAddr};
use std::thread;
use std::time::Duration;
use std::convert::TryInto;
use std::io::{self, Read, Write};
use std::io::ErrorKind;
use std::fs::File;
use std::collections::HashMap;

const CHUNK_SIZE: usize = 508;
const HEADER_SIZE: usize = 8; // Assuming usize is 8 bytes (on a 64-bit system)
const END_OF_TRANSMISSION: usize = usize::MAX;
const ACK_TIMEOUT: Duration = Duration::from_millis(500);

fn receive_image_from_server(socket: &UdpSocket, save_path: &str) -> io::Result<()> {
    println!("Receiving...");
    let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut buffer = [0u8; CHUNK_SIZE + HEADER_SIZE];
    let mut end_of_transmission_received = false;
	while !end_of_transmission_received {
	    println!("Waiting for data...");
	    match socket.recv_from(&mut buffer) {
		Ok((size, _)) => {
		    println!("Received data: {} bytes", size);
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
            println!("Sequence Number: {}", sequence_number);

            // Send ACK for the received chunk
            let ack_packet = sequence_number.to_be_bytes();
            if let Err(e) = socket.send_to(&ack_packet,"127.0.0.1:65432") {
                eprintln!("Failed to send ACK: {}", e);
                continue;
            }
            println!("Sending ACK for sequence {}", sequence_number);

            if sequence_number == END_OF_TRANSMISSION {
                end_of_transmission_received = true;
            } else {
                let image_data = &buffer[HEADER_SIZE..size];
                file_storage.insert(sequence_number, image_data.to_vec());
            }
/////////////////////////////////////////////////////////////////////////////////////
		    // let sequence_number = usize::from_be_bytes(buffer[..HEADER_SIZE].try_into().unwrap());
		    // println!("Sequence Number: {}", sequence_number);

		    // if sequence_number == END_OF_TRANSMISSION {
		    //     println!("End of Transmission received");
		    //     end_of_transmission_received = true;
		    // } else {
		    //     let image_data = &buffer[HEADER_SIZE..size];
		    //     file_storage.insert(sequence_number, image_data.to_vec());
		    //     println!("Stored image chunk: Sequence {}", sequence_number);

		    //     let ack_packet = sequence_number.to_be_bytes();
		    //     println!("Sending ACK for sequence {}", sequence_number);
		    //     socket.send_to(&ack_packet, "127.0.0.1:65432")?; // Replace with the server address if necessary
		    // }
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

    let mut file = File::create(save_path)?;
    file.write_all(&complete_image)?;
    println!("Image completed!!!!!!!!!!!!!");
  
    Ok(()) 
    
}

   
fn send_image_to_server(server_address: &SocketAddr, image_path: &str ,socket: &UdpSocket) -> io::Result<()> {
	// Extract the image name from the path
    let image_name = image_path.rsplit('/').next().unwrap_or(image_path);

    //let socket = UdpSocket::bind("0.0.0.0:65432")?;
    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    socket.set_read_timeout(Some(ACK_TIMEOUT))?;

    // Send the image name first
    let mut name_packet = Vec::with_capacity(HEADER_SIZE + image_name.len());
    name_packet.extend_from_slice(&(usize::MAX - 1).to_be_bytes()); // Special sequence number for the name
    name_packet.extend_from_slice(image_name.as_bytes());

    socket.send_to(&name_packet, server_address)?;
    // Wait for ACK for the name packet here (similar to how you wait for ACKs below)

    //Start of sending the image itself
    println!("Sending Image");
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
    println!("Done chunking");
        loop {
            println!("start ACK");
            socket.send_to(&packet, server_address)?;
            let mut ack_buffer = [0; HEADER_SIZE];
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    println!("ACK");
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
    println!("Done ACK");
    // Send end-of-transmission notification
    let mut eot_packet = Vec::with_capacity(HEADER_SIZE);
    eot_packet.extend_from_slice(&END_OF_TRANSMISSION.to_be_bytes());
    eot_packet.extend(vec![0; HEADER_SIZE - eot_packet.len()]);
    socket.send_to(&eot_packet, server_address)?;
    println!("Done");
   
    
    Ok(())
    
}

fn select_server(address: &str,socket: &UdpSocket) {	 

	let server_address_str = address;
         match server_address_str.parse::<SocketAddr>() {
         Ok(server_address) => {
            let image_path = "/home/mira/Distributed/Instgram-Based-Distributed-System/client_image/src/image1.png";
            if let Err(e) = send_image_to_server(&server_address, image_path,socket) {
                eprintln!("Failed to send image: {}", e);
            }
        },
        Err(_) => {
            eprintln!("Invalid server address: {}", server_address_str);
        }
    }

}


fn send_message_to_server(socket: &UdpSocket,server_address: &SocketAddr,msg: String) {
    let message = format!("{}",msg);
    socket.send_to(message.as_bytes(), server_address).expect("Failed to send message");
    println!("client sent req");         
}
       
       
fn recive_ack(socket: &UdpSocket,ser_list: &Vec<&str>)
{
	let mut ack_buffer = [0;4];
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    let ack_id_number = u32::from_be_bytes(ack_buffer);
                    let ack_id_number_usize  =ack_id_number as usize;
                        println!("Message recived from {}",ser_list[ack_id_number_usize-1]);
                        select_server(ser_list[ack_id_number_usize-1],socket); //send the socket ?

            	},
                Err(e) => return (), // Some other error
           }
}         
             
fn main() {
    let ser_list = vec![
        "10.40.54.24:65432",
        "127.0.0.1:65432",
        "10.40.63.13:65432",
    ];

    let server_addresses = vec![
        "10.40.54.24:65432",
        "127.0.0.1:65432",
        "10.40.63.13:65432",
    ]
    .into_iter()
    .map(|addr| addr.parse::<SocketAddr>().expect("Failed to parse server address"))
    .collect::<Vec<_>>();
    
    //list of path to image 
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    for address in server_addresses {
        // Directly call the function here without spawning a new thread
        send_message_to_server(&socket,&address,"Request".to_string());
    }
    
    recive_ack(&socket, &ser_list);

    if let Err(e) = receive_image_from_server(&socket, "server_output.png") {
        eprintln!("Failed to receive image: {}", e);
        }
    

}