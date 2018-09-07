use std::net::{TcpListener, TcpStream, SocketAddr};
use std::process::exit;
use std::io::{self, Read, Write, stdin};
use std::str;
use std::env;
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

fn handle_client(mut stream: TcpStream, tx: Sender<Event>) {

    let mut buf = [0; MSG_BUF_LEN];
    let msg_len = stream.read(&mut buf).unwrap();
    let client_name = str::from_utf8(&buf[0..msg_len]).unwrap().to_string();
    
    loop {
        let msg_len = stream.read(&mut buf).unwrap();
        if msg_len == 0 {
            break;
        }
        let event = Event::Message {
            name: client_name.clone(),
            text: str::from_utf8(&buf[0.. msg_len]).unwrap().to_string(),
            sock_addr: stream.peer_addr().unwrap(),
        };
        let _ = tx.send(event);
    }
    let event = Event::Disconnected {
        sock_addr: stream.peer_addr().unwrap(),
    };
    let _ = tx.send(event);
}

fn main() {
    
    match env::args().last() {
        Some(arg) => {
            if arg == "s" {
                match start_server() {
                    Ok(_) => {},
                    Err(e) => {
                        println!("server error: {}", e);
                    },
                } 
            } else if arg == "c" {
                match start_client() {
                    Ok(_) => {},
                    Err(_) => {
                        println!("client error");
                        exit(1);
                    },
                } 
            } else {
                println!("wrong argument: use 'c' for start client, 's' for start server" );
                exit(1);
            }
        },
        None => {},
    }
}

fn start_server() -> Result<(), std::io::Error> {
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
                    match m_guard.get(addr) {
                        Some(mut stream) => {
                            let mut answer = String::new();
                            answer += &name;
                            answer += " > ";
                            answer += &text;
                            let _ = stream.write(answer.as_bytes());
                        
                        },
                        None => (),
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

    let listener = match TcpListener::bind("[::]:5858") {
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
    let sock_addr = stream.peer_addr()?.clone();
    println!("client connectded: {}", sock_addr);
    {   
        let mut m_guard = arc.lock().unwrap();
        m_guard.insert(sock_addr, stream);
    }
    std::thread::spawn(move || {
        handle_client(stream_cp, tx_copy);
    });
    Ok(())
}

fn start_client() -> io::Result<()> {
    let addr = "127.0.0.1:5858";
    let mut stream = TcpStream::connect(addr)?;
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
    let _ = stream.write(buf.trim().as_bytes());

    loop {
        buf.clear();
        let _ = stdin().read_line(&mut buf);
        let _ = stream.write(buf.trim().as_bytes());
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


