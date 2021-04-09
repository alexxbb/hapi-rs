use hapi_rs::session::*;

pub fn start() -> Session {
    let addr: std::net::SocketAddrV4 = "127.0.0.1:41000".parse().unwrap();
    start_engine_socket_server(addr.port(), true, 2000).expect("Could not start HARS server");
    Session::connect_to_socket(addr).expect("Could not connect to session")
}
