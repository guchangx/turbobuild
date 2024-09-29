
use std::io::Read;
use std::io::Write;

#[derive(Default, Clone)]
pub struct Receiver {
    port: u16,
    common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>,
}

impl Receiver {
    pub fn new(common: std::sync::Weak<std::sync::Mutex<crate::enter::Common>>) -> Self {
        return Self {
            port: 9301,
            common,
        }
    } 
    
    pub fn init(&self) {
        println!("init ipc socket");
        let listener = std::net::TcpListener::bind(format!("localhost:{}", self.port)).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("new connection socket");
                    std::thread::spawn(||{
                        Self::handle_ipc_socket(stream);

                    });
                },
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        }
    }
    
    fn handle_ipc_socket(mut stream: std::net::TcpStream) {
        let mut buffer = [0; 512];
        
        loop {
            match stream.read(&mut buffer) {
                Ok(size) => {
                    println!("Received {} bytes", size);
                    println!("{}", String::from_utf8_lossy(&buffer[..size]));
                    
                    stream.write_all(b"done").unwrap();
                },
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
    }
}