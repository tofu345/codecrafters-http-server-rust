use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::{env, fs, thread};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut dir = String::new();

    println!("{args:?}");

    if let Some(v) = args.get(3) {
        dir.push_str(v.as_str());
    };

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    while let Ok((stream, addr)) = listener.accept() {
        handle(stream, addr, dir.clone());
    }
}

fn handle(mut stream: TcpStream, _addr: SocketAddr, dir: String) {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        let read_bytes = stream.read(&mut buffer).unwrap();
        println!("read {} bytes", read_bytes);

        let buffer = String::from_utf8(buffer.to_vec()).expect("invalid utf8");
        let req = Request::parse(&buffer).unwrap();

        let mut _file_contents = String::new();

        let (code, body) = match req.path {
            "/" => (200, None),
            x if x.starts_with("/echo") => (
                200,
                x.strip_prefix("/echo/").map(|x| Body {
                    contents: x.as_bytes(),
                    mime: "text/plain",
                }),
            ),
            "/user-agent" => (
                200,
                Some(Body {
                    contents: req.headers.get("User-Agent").unwrap().as_bytes(),
                    mime: "text/plain",
                }),
            ),
            x if x.starts_with("/files") => {
                let filename = x.strip_prefix("/files/").unwrap();
                let file_path = format!("{}/{}", dir, filename);
                let contents = fs::read_to_string(file_path);

                if let Err(_) = contents {
                    (404, None)
                } else {
                    _file_contents = contents.unwrap();
                    (
                        200,
                        Some(Body {
                            contents: _file_contents.as_bytes(),
                            mime: "application/octet-stream",
                        }),
                    )
                }
            }
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
    headers: HashMap<&'a str, &'a str>,
}

impl<'a> Request<'a> {
    fn parse(data: &'a String) -> Result<Request<'a>, Box<dyn Error>> {
        let mut lines = data.split("\r\n");

        let line: Vec<&str> = lines
            .next()
            .expect("invalid http data")
            .split(" ")
            .collect();

        let method = line.get(0).expect("invalid http data");
        let path = line.get(1).expect("invalid http data");
        let mut headers = HashMap::new();

        for line in lines {
            if let Some((k, v)) = line.split_once(": ") {
                headers.insert(k, v);
            }
        }

        Ok(Request {
            method,
            path,
            headers,
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
