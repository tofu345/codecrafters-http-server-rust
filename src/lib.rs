use std::io::Write;
use std::net::TcpStream;
use std::{collections::HashMap, error::Error, io, net::TcpListener, thread};

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
    ///     Response::new(200, "hi")
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
            // todo: put code here into thread

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
                res.write_headers(&mut stream)
                    .expect("failure writing headers");
                stream.write_all(data.as_bytes()).unwrap();
            } else {
                write!(stream, "\r\n").expect("failure writing newline");
            }

            stream.flush().unwrap();
        });
    }
}

fn default_method_not_allowed_handler(_req: &Request) -> Response {
    Response::new(404, "method not allowed")
}

fn default_not_found_handler(_req: &Request) -> Response {
    Response::new(404, "page not found")
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
        routes.iter().find(|r| {
            if r.path.contains(":?") {
                let prefix = r.path.strip_suffix(":?").unwrap();
                path.starts_with(prefix)
            } else {
                r.path == path
            }
        })
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

impl ToBytes for &str {
    fn as_bytes(&self) -> &[u8] {
        self.to_owned().as_bytes()
    }
}

pub struct Response {
    code: u16,
    data: Option<Box<dyn ToBytes + 'static>>,
    headers: HashMap<String, String>,
}

impl Response {
    /// Returns new Response
    /// # Example
    ///
    /// ```
    /// use http_server_starter_rust::{Response, Request};
    ///
    /// fn test(_req: &Request) -> Response {
    ///     Response::new(200, "hi")
    /// }
    /// ```
    pub fn new(code: u16, data: impl ToBytes + 'static) -> Response {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_owned(), "text/plain".to_owned());
        headers.insert(
            "Content-Length".to_owned(),
            data.as_bytes().len().to_string(),
        );

        Response {
            code,
            data: Some(Box::new(data)),
            headers,
        }
    }

    /// Returns new response with no data
    ///
    /// # Example
    ///
    /// ```
    /// use http_server_starter_rust::{Request, Response};
    ///
    /// fn test(_req: &Request) -> Response {
    ///     Response::empty(200)
    /// }
    /// ```
    pub fn empty(code: u16) -> Response {
        Response {
            code,
            data: None,
            headers: HashMap::new(),
        }
    }

    /// Returns new response with specified headers
    ///
    /// # Example
    ///
    /// ```
    /// use http_server_starter_rust::{Request, Response};
    ///
    /// fn test(_req: &Request) -> Response {
    ///     Response::empty(200).add_header("foo", "bar")
    /// }
    /// ```
    pub fn add_header(mut self, key: &str, val: &str) -> Response {
        self.headers.insert(key.to_owned(), val.to_owned());
        self
    }

    /// Adds headers to current response with specified headers
    ///
    /// Handy when adding multiple headers
    ///
    /// # Example
    ///
    /// ```
    /// use http_server_starter_rust::{Request, Response};
    ///
    /// fn test(_req: &Request) -> Response {
    ///     let mut res = Response::empty(200);
    ///
    ///     res.add_headers("foo", "bar");
    ///     res.add_headers("foo2", "bar");
    ///     res
    /// }
    /// ```
    pub fn add_headers(&mut self, key: &str, val: &str) {
        self.headers.insert(key.to_owned(), val.to_owned());
    }

    fn write_headers(&self, f: &mut impl io::Write) -> io::Result<()> {
        let mut output = String::new();
        for (key, val) in self.headers.iter() {
            output.push_str(format!("{key}: {val}\r\n").as_str());
        }

        output.push_str("\r\n");
        write!(f, "{}", output)
    }
}
