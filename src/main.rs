//
use std::{env, fs};

use http_server_starter_rust::{Request, Response, ResponseType, Router};

fn main() {
    let port = "127.0.0.1:4221";
    let mut r = Router::new(port);

    r.handle_func("/", base_handler, vec!["GET"]);
    r.handle_func("/echo/:?", echo_handler, vec!["GET"]);
    r.handle_func("/user-agent", user_agent_handler, vec!["GET"]);
    r.handle_func("/files/:?", files_handler, vec!["GET", "POST"]);

    println!("Listening on port {}", port);
    r.serve();
}

fn base_handler(_req: &Request) -> Response {
    Response::new(200, None)
}

fn echo_handler(req: &Request) -> Response {
    let x = req.path.strip_prefix("/echo/").unwrap();

    Response::text(200, x)
}

fn user_agent_handler(req: &Request) -> Response {
    let agent = req.headers.get("User-Agent").unwrap();

    Response::text(200, &agent)
}

fn files_handler(req: &Request) -> Response {
    let filename = req.path.strip_prefix("/files/").unwrap();
    let args: Vec<String> = env::args().collect();
    let directory = env::current_dir()
        .expect("missing directory param")
        .join(&args[2]);
    let file_path = directory.join(filename);
    let contents = fs::read_to_string(file_path.clone());

    if req.method == "POST" {
        fs::write(file_path, req.body.clone()).expect("unable to write");
        return Response::new(201, None);
    }

    if let Err(_) = contents {
        return Response::new(404, None);
    }

    let contents = contents.unwrap();
    Response::with_mime_type(200, Some(Box::new(contents)), ResponseType::File)
}
