use std::error::Error;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    while let Ok((stream, addr)) = listener.accept() {
        handle(stream, addr);
    }
}

fn handle(mut stream: TcpStream, _addr: SocketAddr) {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        let read_bytes = stream.read(&mut buffer).unwrap();
        println!("read {} bytes", read_bytes);

        let buffer = String::from_utf8(buffer.to_vec()).expect("invalid utf8");
        let req = Request::parse(&buffer).unwrap();

        let (code, body) = match req.path {
            "/" => (200, None),
            x if x.starts_with("/echo") => (
                200,
                x.strip_prefix("/echo").map(|x| Body {
                    contents: x.as_bytes(),
                    mime: "text/plain",
                }),
            ),
            "/user-agent" => (
                200,
                Some(Body {
                    contents: req.agent.as_bytes(),
                    mime: "text/plain",
                }),
            ),
            _ => (404, None),
        };

        write!(
            stream,
            "HTTP/1.1 {code} {}\r\n",
            if code == 200 { "OK" } else { " " },
        )
        .unwrap();

        if let Some(body) = body {
            body.write_headers(&mut stream)
                .expect("failure writing headers");
            stream.write_all(body.contents).unwrap();
        } else {
            write!(stream, "\r\n").expect("failure writing newline");
        }
    });
}

pub struct Request<'a> {
    path: &'a str,
    method: &'a str,
    host: &'a str,
    agent: &'a str,
}

impl<'a> Request<'a> {
    fn parse(data: &'a String) -> Result<Request<'a>, Box<dyn Error>> {
        let mut lines = data.split("\r\n");

        let line: Vec<&str> = lines
            .next()
            .expect("invalid http data")
            .split(" ")
            .collect();
        let method = line[0];
        let path = line[1];

        lines.next();
        let line: Vec<&str> = lines
            .next()
            .expect("invalid http data")
            .split(" ")
            .collect();
        let host = line[1];

        let line: Vec<&str> = lines
            .next()
            .expect("invalid http data")
            .split(" ")
            .collect();
        let agent = line[1];

        Ok(Request {
            method,
            path,
            host,
            agent,
        })
    }
}

pub struct Body<'a> {
    contents: &'a [u8],
    mime: &'a str,
}

impl<'a> Body<'a> {
    pub fn write_headers(&self, f: &mut impl io::Write) -> io::Result<()> {
        write!(
            f,
            "Content-Type: {}\r\nContent-Length: {}\r\n\r\n",
            self.mime,
            self.contents.len()
        )
    }
}
