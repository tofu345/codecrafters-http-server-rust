use std::io::Write;
use std::net::TcpStream;
use std::{collections::HashMap, error::Error, fmt::Display, io, net::TcpListener, thread};

pub struct Router<'a> {
    host: String,
    routes: Vec<Route<'a>>,
    method_not_allowed_handler: Handler,
    not_found_handler: Handler,
}

impl<'a> Router<'a> {
    /// # Examples
    /// ```
    /// use http_server_starter_rust::Router;
    ///
    /// let mut r = Router::new("127.0.0.1:12345");
    /// ```
    pub fn new(addr: &str) -> Router {
        Router {
            routes: vec![],
            host: addr.to_string(),
            method_not_allowed_handler: default_method_not_allowed_handler,
            not_found_handler: default_not_found_handler,
        }
    }

    /// Generates new route and adds to router
    ///
    /// Routes are matched in the order they are added
    ///
    /// # Examples
    /// ```
    /// use http_server_starter_rust::{Router, Request, Response};
    ///
    /// let mut r = Router::new("127.0.0.1:12345");
    ///
    /// r.handle_func("/hi", test, vec!["GET"]);
    ///
    /// // Wildcard
    /// r.handle_func("/te:?", test, vec!["GET"]);
    /// r.handle_func("/test", test, vec!["GET"]); // never reached because of wildcard
    ///
    /// fn test(_req: &Request) -> Response {
    ///     Response::text(200, "hi")
    /// }
    /// ```
    pub fn handle_func(&mut self, path: &'a str, handler: Handler, methods: Vec<&'a str>) {
        let route = Route {
            path,
            methods,
            handler,
        };

        self.routes.push(route);
    }

    /// Sets custom not found handler
    pub fn not_found_handler(&mut self, f: Handler) {
        self.not_found_handler = f;
    }

    /// Runs Tcp Server on specified port
    pub fn serve(&self) {
        let listener = TcpListener::bind(self.host.clone()).unwrap();

        while let Ok((mut stream, _addr)) = listener.accept() {
            let req = Request::from_stream(&mut stream);
            let route = Route::match_route(&self.routes, req.path.as_str());

            println!("-> {}", req.path);

            if let Some(route) = route {
                if !route.has_method(req.method.as_str()) {
                    Router::handle(self.method_not_allowed_handler, req, stream);
                    continue;
                }

                let handler = route.handler;
                Router::handle(handler, req, stream)
            } else {
                Router::handle(self.not_found_handler, req, stream);
            }
        }
    }

    /// Runs handler in seperate thread and writes data to stream
    fn handle(f: Handler, req: Request, mut stream: TcpStream) {
        thread::spawn(move || {
            let mut res = f(&req);

            write!(
                stream,
                "HTTP/1.1 {} {}\r\n",
                res.code,
                if res.code == 200 { "OK" } else { " " },
            )
            .unwrap();

            if let Some(data) = res.data.take() {
                res.write_headers(data.as_bytes().len(), &mut stream)
                    .expect("failure writing headers");
                stream.write_all(data.as_bytes()).unwrap();
            } else {
                write!(stream, "\r\n").expect("failure writing newline");
            }
        });
    }
}

fn default_method_not_allowed_handler(_req: &Request) -> Response {
    Response::text(404, "method not allowed")
}

fn default_not_found_handler(_req: &Request) -> Response {
    Response::text(404, "page not found")
}

#[derive(Debug)]
struct Route<'a> {
    path: &'a str,
    methods: Vec<&'a str>,
    handler: Handler,
}

impl<'a> Route<'a> {
    fn has_method(&self, method: &'a str) -> bool {
        self.methods.contains(&method)
    }

    fn match_route(routes: &'a Vec<Route<'a>>, path: &'a str) -> Option<&'a Route<'a>> {
        for r in routes.iter() {
            if r.path.contains(":?") {
                let prefix = r.path.strip_suffix(":?").unwrap();
                if path.starts_with(prefix) {
                    return Some(r);
                }
            } else if r.path == path {
                return Some(r);
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct Request {
    pub path: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    fn parse_from_utf8(data: &[u8]) -> Result<Request, Box<dyn Error>> {
        Request::parse(String::from_utf8(data.to_vec())?)
    }

    fn parse(data: String) -> Result<Request, Box<dyn Error>> {
        let data = data.replace("\0", "");
        let mut lines = data.split("\r\n");

        let line = lines.next().expect("invalid http data");
        let line: Vec<&str> = line.split(" ").collect();

        let method = line.get(0).expect("invalid http data").to_string();
        let path = line.get(1).expect("invalid http data").to_string();
        let mut headers = HashMap::new();

        for line in lines {
            if let Some((k, v)) = line.split_once(": ") {
                headers.insert(k.to_string(), v.to_string());
            }
        }

        let data: Vec<&str> = data.split("\r\n").collect();

        Ok(Request {
            method,
            path,
            headers,
            body: data[data.len() - 1].to_string(),
        })
    }

    fn from_stream(s: &mut impl io::Read) -> Request {
        let mut buffer = [0; 4096];
        s.read(&mut buffer).unwrap();

        Request::parse_from_utf8(&buffer).unwrap()
    }
}

pub type Handler = fn(&Request) -> Response;

pub trait ToBytes {
    fn as_bytes(&self) -> &[u8];
}

impl ToBytes for String {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

pub enum ResponseType {
    Text,
    File,
}

impl Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ResponseType::*;

        let text = match self {
            Text => "text/plain",
            File => "application/octet-stream",
        };

        write!(f, "{}", text)
    }
}

pub struct Response {
    code: u16,
    data: Option<Box<dyn ToBytes>>,
    mime: ResponseType,
}

impl Response {
    pub fn new(code: u16, data: Option<Box<dyn ToBytes>>) -> Response {
        Response {
            code,
            data,
            mime: ResponseType::Text,
        }
    }

    pub fn with_mime_type(
        code: u16,
        data: Option<Box<dyn ToBytes>>,
        mime: ResponseType,
    ) -> Response {
        Response { code, data, mime }
    }

    pub fn text(code: u16, text: &str) -> Response {
        Response {
            code,
            data: Some(Box::new(text.to_string())),
            mime: ResponseType::Text,
        }
    }

    fn write_headers(&self, content_len: usize, f: &mut impl io::Write) -> io::Result<()> {
        write!(
            f,
            "Content-Type: {}\r\nContent-Length: {}\r\n\r\n",
            self.mime, content_len
        )
    }
}
