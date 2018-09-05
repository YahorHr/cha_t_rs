use std::net::{TcpListener, TcpStream};
use std::process::exit;
use std::io::{Read, Write, stdin};
use std::str;
use std::env;

fn handle_client(mut stream: TcpStream) {

    let mut buf = [0; 256];
    loop {
        let _ = stream.read(&mut buf);
        println!("{}",str::from_utf8(&buf).unwrap());
        println!("{:?}", stream.peer_addr() );
    }
}

fn main() {
    
    match env::args().last() {
        Some(arg) => {
            if arg == "s" {
                start_server();
            } else if arg == "c" {
                start_client();
            } else {
                println!("wrong argument: use 'c' for start client, 's' for start server" );
                exit(1);
            }
        },
        None => {},
    }
}

fn start_server() {
    println!("Starting server...");

    let mut vec: Vec<[u8; 256]> = Vec::new();


    let listener = match TcpListener::bind("127.0.0.1:5858") {
        Ok(stream) => stream,
        Err(_) => {
            println!("error create new listener");
            exit(1);
        }
    };

    for stream in listener.incoming() {
        std::thread::spawn(move || {
            handle_client(stream.unwrap());
        });
    }
}

fn start_client() {
    let addr = "127.0.0.1:5858";
    let mut stream = TcpStream::connect(addr).unwrap();
    let mut buf = String::new();

    loop {
        let _ = stdin().read_line(&mut buf);
        let _ = stream.write(buf.as_bytes());
        println!("sent!!!")
    }
}


