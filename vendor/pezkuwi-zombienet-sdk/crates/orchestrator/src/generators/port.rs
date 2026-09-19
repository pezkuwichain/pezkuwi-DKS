use std::net::TcpListener;

use configuration::shared::types::Port;
use support::constants::THIS_IS_A_BUG;

use super::errors::GeneratorError;
use crate::shared::types::ParkedPort;

// TODO: (team), we want to continue support ws_port? No
enum PortTypes {
	Rpc,
	P2P,
	Prometheus,
}

pub fn generate(port: Option<Port>) -> Result<ParkedPort, GeneratorError> {
	let port = port.unwrap_or(0);
	let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
		.map_err(|_e| GeneratorError::PortGeneration(port, "Can't bind".into()))?;
	let port = listener
		.local_addr()
		.expect(&format!("We should always get the local_addr from the listener {THIS_IS_A_BUG}"))
		.port();
	Ok(ParkedPort::new(port, listener))
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn generate_random() {
		let port = generate(None).unwrap();
		let listener = port.1.write().unwrap();

		assert!(listener.is_some());
	}

	/// Asking for a specific port gives you that port.
	///
	/// The number is not the subject and must not be written down. This asserted on a hardcoded
	/// 33056, which sits inside Linux's default ephemeral range (32768-60999): the kernel hands
	/// those out to outgoing connections, so on any host with network traffic the test is a coin
	/// flip. It came up tails on 2026-09-17 and failed a twenty-hour release gate with
	/// `PortGeneration(33056, "Can't bind")` -- on a box running fourteen chain services, which
	/// is to say on a box doing exactly what the fleet does.
	///
	/// So the port is borrowed from the OS first. Binding zero returns something free, and the
	/// same machinery is then asked for it by number. There is a window between dropping the
	/// probe and rebinding where something else could take it, and that window is microseconds
	/// against the near-certainty of the old approach; more to the point, a failure here would
	/// now mean the mechanism is broken rather than that the host was busy.
	#[test]
	fn generate_fixed_port() {
		let free = TcpListener::bind("0.0.0.0:0")
			.expect("the OS can always give out a free port")
			.local_addr()
			.expect("a bound listener has a local address")
			.port();

		let port = generate(Some(free)).unwrap();
		let listener = port.1.write().unwrap();

		assert!(listener.is_some());
		assert_eq!(port.0, free, "generate(Some(p)) must return p, not a port of its choosing");
	}
}
