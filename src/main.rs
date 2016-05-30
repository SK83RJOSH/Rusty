extern crate irc;

mod lib;

use std::default::Default;
use std::io::Result;
use std::process::exit;

use irc::client::prelude::Config;

fn main() {
	if let Ok(config) = handle_config() {
		let bot = lib::Bot::new(config);

		if let Ok(bot) = bot {
			bot.run()
		} else if let Err(err) = bot {
			println!("Failed to spawn bot instance: {}", err);
		}
	} else {
		println!("Failed to access config file, do you have read/write permissions?");
	}
}

fn handle_config() -> Result<Config> {
	let config = Config::load(lib::CONFIG_PATH);

	match config {
		Ok(result) => Ok(result),
		Err(err) => {
			let config = Config {
				nickname: Some("PleaseConfigureMe".into()),
				alt_nicks: Some(vec!["WhyDidYou".into(), "ForgetTo".into(), "ConfigureMe".into()]),
				server: Some("default.com".into()),
				owners: Some(vec!["SK83RJOSH".into()]),
				..Default::default()
			};

			println!("Failed to open config: {}", err);
			try!(config.save(lib::CONFIG_PATH));
			println!("Wrote default config to {}!", lib::CONFIG_PATH);
			println!("Now exiting...");

			exit(0);
		}
	}
}
