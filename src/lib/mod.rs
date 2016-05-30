extern crate irc;

mod commands;

use std::io::Result;
use std::collections::HashMap;
use irc::client::prelude::*;

pub static CONFIG_PATH: &'static str = "config.json";

pub struct Bot {
	cmds: HashMap<String, Box<commands::Command>>,
	server: IrcServer,
}

// TODO: Look into extending irc::client::server
impl Bot {
	pub fn new(config: Config) -> Result<Bot> {
		let mut bot = Bot {
			cmds: HashMap::new(),
			server: try!(IrcServer::from_config(config)),
		};

		bot.cmds.insert("echo".into(), Box::new(commands::EchoCommand::new()));
		bot.cmds.insert("say".into(), Box::new(commands::EchoCommand::new()));
		bot.cmds.insert("kick".into(), Box::new(commands::KickCommand::new()));
		bot.cmds.insert("join".into(), Box::new(commands::JoinCommand::new()));
		bot.cmds.insert("part".into(), Box::new(commands::PartCommand::new()));

		Ok(bot)
	}

	pub fn run(&self) {
		self.server.identify().unwrap();

		for message in self.server.iter() {
			if let Ok(message) = message {
				println!("{}", message);

				match message.command {
					// TODO: Handle kicks, bans, ctcp, and invites
					Command::PRIVMSG(ref target, ref text) => {
						if let Some(sender) = message.source_nickname() {
							self.handle_privmsg(target.clone(), text.clone(), sender.into())
						}
					}
					_ => {}
				}
			}
		}
	}

	fn handle_privmsg(&self, target: String, text: String, sender: String) {
		let is_private = self.server.current_nickname() == target;
		let is_bang = text.starts_with("!");
		let is_command = is_private || is_bang;

		if !is_command || text.len() == 1 {
			return;
		}

		let text = if is_bang { text[1..].into() } else { text };
		let target = if is_private { sender.clone() } else { target };

		if let Err(err) = self.handle_command(text, target, sender.clone()) {
			println!("Failed to process command: {:?}", err);
		}
	}

	fn handle_command(&self, text: String, target: String, sender: String) -> Result<()> {
		let mut strs = text.split_whitespace();
		let command = strs.next().unwrap();
		let args = strs.map(|s| s.into()).collect::<Vec<String>>();
		let input = args.join(" ");

		if let Some(command) = self.cmds.get(command) {
			try!(command.execute(input, &self.server, target, sender));
		} else if !target.starts_with("#") {
			try!(self.server.send_privmsg(&target, "Unknown command."));
		}

		Ok(())
	}
}
