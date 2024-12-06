use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    time::Duration,
};

use anyhow::Result;

use gtk::gio;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

pub trait Remote {
    fn ip_addr(&self) -> String;

    fn port(&self) -> u16;

    async fn check_port_reachability(&self) -> Result<()> {
        let parsed_ip_addr = self.ip_addr().parse::<Ipv4Addr>()?;
        let addr = SocketAddrV4::new(parsed_ip_addr, self.port());
        gio::spawn_blocking(move || TcpStream::connect_timeout(&addr.into(), CONNECT_TIMEOUT))
            .await
            .unwrap()?;
        Ok(())
    }
}
