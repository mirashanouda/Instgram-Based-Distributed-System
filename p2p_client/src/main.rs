use common_lib::utils;
use std::thread;


fn main() {
    // let friend_addr: SocketAddr = "127.0.0.1:9999".parse().expect("Failed to parse server address");

    // for i in 0..4 {
    //     let image_name = format!("image{}.jpg", i);
    //     utils::convert_to_low_resolution(&image_name, 50, 50);
    // }

    let mut handles = vec![];
    let handle1 = thread::spawn(move || {
        utils::send_my_img()
    });
    handles.push(handle1);

    for handle in handles {
        handle.join().unwrap();
    }
}
