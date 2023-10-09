// Uncomment this block to pass the first stage
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                let mut data = [0; 1024];
                let _data = stream.read(&mut data).unwrap();

                let data: Vec<u8> = data.into_iter().collect();
                let data = String::from_utf8(data).unwrap();
                let first_line = data.split("\r\n").next().unwrap();
                let path = first_line.split(" ").collect::<Vec<&str>>()[1];

                if path == "/" {
                    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
                } else {
                    stream
                        .write("HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes())
                        .unwrap();
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
