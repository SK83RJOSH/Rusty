extern crate irc;

mod lib;

use std::default::Default;
use irc::client::prelude::Config;

fn main() {
	let config = Config {
		nickname: Some(format!("RustRGood")),
		alt_nicks: Some(vec![format!("RustRReallyGood")]),
		server: Some(format!("irc.jc-mp.com")),
		channels: Some(vec![format!("#gibbed")]),
		// port: Some(6697),
		// use_ssl: Some(true),
		..Default::default()
	};
	let bot = lib::Bot::new(config);

	if let Ok(mut bot) = bot {
		bot.run()
	} else if let Err(err) = bot {
		println!("Failed to spawn bot instance: {}", err);
	}
}
