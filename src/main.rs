// Uncomment this block to pass the first stage
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut data = [0; 1024];
                stream.read(&mut data).unwrap();

                let data = parse_http_request(&data);

                let first_line = data.split("\r\n").next().unwrap();
                let path = first_line.split(" ").collect::<Vec<&str>>()[1];

                if path == "/" {
                    stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).unwrap();
                } else if path.starts_with("/echo/") {
                    let message = path.split("/echo/").last().unwrap();
                    stream.write(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", message.len(), message).as_bytes()).unwrap();
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

fn parse_http_request(request: &[u8]) -> String {
    String::from_utf8(request.to_vec()).unwrap()
}
