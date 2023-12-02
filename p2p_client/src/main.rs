use common_lib::utils;
use std::net::UdpSocket;


fn main() {
    let mut handles = vec![];
    let handle1 = thread::spawn(move || {
        utils::send_my_img()
    });
    handles.push(handle1);

    for handle in handles {
        handle.join().unwrap();
    }
}
