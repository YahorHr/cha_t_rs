use std::net::{TcpListener, TcpStream, SocketAddr};
use std::process::exit;
use std::io::{self, Read, Write, stdin, BufReader, BufRead};
use std::env;
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

extern crate chrono;
use chrono::Local;

extern crate getopts;
use getopts::Options;


const MSG_BUF_LEN: usize = 4096;

enum Event {
    Message{
        name: String,
        text: String,
        sock_addr: SocketAddr,
    },
    Disconnected{
        sock_addr: SocketAddr,
    },
}

fn print_usage(prog: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", prog);
    println!("{}", opts.usage(&brief));
}

fn main() {
    let mut ip = String::from("[::]");
    let mut port = String::from("5858");

    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    let prog = args[0].clone();

    opts.optflag("h", "help", "Print help menu.");
    opts.optflag("s", "server", "Start chat in server mode.");
    opts.optflag("c", "client", "Start chat in client mode.");
    opts.optopt("a", "address", "Ip address. Default: [::]", "<IP>");
    opts.optopt("p", "port", "Port. Default: 5858", "<PORT>");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            println!("{}", e.to_string());
            exit(1);
        },
    };

    if args.len() < 2 {
        println!("Wrong arguments. For more details use -h or --help. Please try again.");
        exit(1);
    }

    if matches.opt_present("h") {
        print_usage(&prog, &opts);
        exit(0);
    }

    if matches.opt_present("s") && matches.opt_present("c") {
        println!("Chat can be started only in one mode (server or client). For more details use -h or --help. Please try again.");
        exit(1);
    }

    if matches.opt_present("a") {
        ip = match matches.opt_str("a") {
            Some(a) => a.to_string(),
            None => String::from("[::]"),
        }
    } else if matches.opt_present("p") {
        port = match matches.opt_str("p") {
            Some(p) => p.to_string(),
            None => String::from("5858"),
        }
    } else {
         println!("Chat can be started in -c or -s mode. For more details use -h or --help. Please try again.");
    }

    let sock_addr = format!("{}:{}", ip, port);

    if matches.opt_present("s") {
        match start_server(sock_addr) {
                Ok(_) => {},
                Err(e) => {
                    println!("server error: {}", e);
            },
        } 
    } else if matches.opt_present("c") {
        match start_client(sock_addr) {
            Ok(_) => {},
                Err(_) => {
                    println!("client error");
                    exit(1);
            },
        } 
    }
}

// server part

fn start_server(sock_addr: String) -> Result<(), std::io::Error> {
    println!("{:?}", SystemTime::now());
    println!("Starting server...");
    
    let clients: HashMap<SocketAddr, TcpStream> = HashMap::new();
    let arc = Arc::new(Mutex::new(clients));
    let arc2 = arc.clone();
    let (tx, rx): (Sender<Event>, Receiver<Event>) = std::sync::mpsc::channel();


    std::thread::spawn(move || {
        
        loop {
            let event = rx.recv().unwrap();
            {
                let mut m_guard = arc2.lock().unwrap();
                
                match event {
                    Event::Message {name, text, sock_addr} => {
                        for addr in m_guard.keys() {
                    if addr == &sock_addr {
                        continue;
                    }
                    
                    if let Some(mut stream) = m_guard.get(addr) {
                        let time = Local::now();

                        let current_time = &time.format("[%H:%M:%S]");
                        let answer = format!("{} {} > {}", current_time, name, text);
                        let _ = stream.write(answer.as_bytes());
                    }
                }
                    },
                    Event::Disconnected {sock_addr} => {
                        m_guard.remove(&sock_addr);
                        println!("client disconnected: {}", sock_addr);
                    },
                }
            }
        }
    });

    let listener = match TcpListener::bind(sock_addr) {
        Ok(stream) => stream,
        Err(_) => {
            println!("error create new listener");
            exit(1);
        }
    };

    for stream in listener.incoming() {
        let result = stream.and_then(|stream| add_client(stream, &tx, &arc));

        match result {
            Ok(_) => {},
            Err(e) => println!("adding client error: {}", e)
        }
    }

    Ok(())
}

fn add_client(stream: TcpStream, tx: &Sender<Event>, arc: &Arc<Mutex<HashMap<SocketAddr, TcpStream>>>) -> Result<(), ::io::Error> {
    let tx_copy = tx.clone();
    let stream_cp = stream.try_clone()?;
    let sock_addr = stream.peer_addr()?;
    println!("client connectded: {}", sock_addr);
    {   
        let mut m_guard = arc.lock().unwrap();
        m_guard.insert(sock_addr, stream);
    }
    std::thread::spawn(move || {
        match handle_client(&stream_cp, &tx_copy) {
            Ok(_) => {},
            Err(_) => {
                let event = Event::Disconnected {
                    sock_addr: stream_cp.peer_addr().unwrap(),
                };
                let _ = tx_copy.send(event);
            }
        }
    });
    Ok(())
}

fn handle_client(stream: &TcpStream, tx: &Sender<Event>) -> Result<(), ::io::Error> {

    let mut buf_reader = BufReader::new(stream);
    let mut client_name = String::new();
    let mut text_msg = String::new();
    buf_reader.read_line(&mut client_name)?;

    loop {
        buf_reader.read_line(&mut text_msg)?;
        if text_msg.is_empty() {
            break;
        }
        let event = Event::Message {
            name: client_name.trim().to_string(),
            text: text_msg.trim().to_string(),
            sock_addr: stream.peer_addr()?,
        };
        let _ = tx.send(event);
        text_msg.clear();
    }
    Err(io::Error::new(io::ErrorKind::Other, "msg_len == 0"))
}

// client part

fn start_client(sock_addr: String) -> io::Result<()> {
    let mut stream = TcpStream::connect(sock_addr)?;
    let mut stream_rv = stream.try_clone()?;
    let mut buf = String::new();
    

    std::thread::spawn(move || {
        match handle_incomming_events(&mut stream_rv) {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
                exit(2);
            },
        }
    });
    
    println!("What is your name?");
    loop {
        let name_len = stdin().read_line(&mut buf)?;
        if name_len > 3 {
            break;
        }
        println!("name length must be more then 3 characters. Try again")
    }
    let _ = writeln!(&mut stream, "{}", buf.trim());
    loop {
        buf.clear();
        let _ = stdin().read_line(&mut buf);
        let _ = writeln!(&mut stream, "{}", buf.trim());
    }
}

fn handle_incomming_events(stream: &mut TcpStream) -> io::Result<()> {
    let mut rx_buf = [0; MSG_BUF_LEN];
    loop {
        let str_len = stream.read(&mut rx_buf)?;
        if str_len == 0 {
            println!("disconnected");
            exit(0);
        }
        println!("{}", std::str::from_utf8(&rx_buf[0..str_len]).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?);
    }
}


