use std::io::Result;
use std::collections::HashMap;

mod commands;

use self::commands::*;

extern crate irc;

use irc::client::data::Command::*;
use irc::client::data::Config;
use irc::client::server::*;
use irc::client::prelude::ServerExt;

pub static CONFIG_PATH: &'static str = "config.json";

pub struct Bot {
	cmds: HashMap<String, Command>,
	server: IrcServer,
}

// TODO: Look into extending irc::client::server
impl Bot {
	pub fn new(config: Config) -> Result<Bot> {
		let mut bot = Bot {
			cmds: HashMap::new(),
			server: try!(IrcServer::from_config(config)),
		};

		bot.cmds.insert("say".into(),
		                Command::new(false,
		                             "".into(),
		                             vec![CommandArg {
			                                  required: true,
			                                  name: "text".into(),
		                                  }],
		                             Box::new(cmd_say)));

		bot.cmds.insert("echo".into(),
		                Command::new(false,
		                             "".into(),
		                             vec![CommandArg {
			                                  required: true,
			                                  name: "text".into(),
		                                  }],
		                             Box::new(cmd_say)));

		bot.cmds.insert("kick".into(),
		                Command::new(true,
		                             "admin".into(),
		                             vec![CommandArg {
			                                  required: true,
			                                  name: "nick".into(),
		                                  },
		                                  CommandArg {
			                                  required: false,
			                                  name: "reason".into(),
		                                  }],
		                             Box::new(cmd_kick)));

		bot.cmds.insert("join".into(),
		                Command::new(true,
		                             "admin".into(),
		                             vec![CommandArg {
			                                  required: false,
			                                  name: "channel".into(),
		                                  }],
		                             Box::new(cmd_join)));

		bot.cmds.insert("part".into(),
		                Command::new(true,
		                             "admin".into(),
		                             vec![CommandArg {
			                                  required: false,
			                                  name: "channel".into(),
		                                  }],
		                             Box::new(cmd_part)));

		Ok(bot)
	}

	pub fn run(&self) {
		self.server.identify().unwrap();

		for message in self.server.iter() {
			if let Ok(message) = message {
				println!("{}", message);

				if let Some(sender) = message.source_nickname() {
					match message.command {
						// TODO: Handle kicks, bans
						PRIVMSG(ref target, ref text) => {
							self.handle_privmsg(target.clone(), text.clone(), sender.into())
						}
						INVITE(ref nickname, ref channel) => {
							self.handle_privmsg(nickname.clone(),
							                    format!("join {}", channel),
							                    sender.into())
						}
						_ => {}
					}
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
			try!(self.server.send_privmsg(&target, &format!("{}: Unknown command", command)));
		}

		Ok(())
	}
}

fn cmd_say(parameters: CommandParameters) -> Result<()> {
	if let Some(text) = parameters.args.get("text") {
		try!(parameters.server.send_privmsg(&parameters.target, text));
	}

	Ok(())
}

fn cmd_kick(parameters: CommandParameters) -> Result<()> {
	if parameters.sender != parameters.target {
		if let Some(nick) = parameters.args.get("nick") {
			if nick == parameters.server.current_nickname() {
				try!(parameters.server.send_kick(&parameters.target, &parameters.sender, "No you."))
			} else if let Some(reason) = parameters.args
				.get("reason")
				.or(Some(&"Requested".to_string())) {
				try!(parameters.server.send_kick(&parameters.target, &nick, reason))
			}
		} else {
			try!(parameters.server.send_notice(&parameters.sender,
			                                   "Command arguments are !kick <nick> [reason]"))
		}
	} else {
		try!(parameters.server.send_privmsg(&parameters.sender,
		                                    "You can't kick people from a private chat..."))
	}

	Ok(())
}

fn cmd_join(parameters: CommandParameters) -> Result<()> {
	let sender = parameters.sender;
	let server = parameters.server;
	let target = parameters.target;

	if let Some(channel) = parameters.args.get("channel").or(Some(&target)) {
		if channel != &sender {
			if channel.starts_with("#") && !channel.contains(",") {
				try!(server.send_join(channel));

				// TODO: Is it worth writing a more generic function for updating the config?
				let config = server.config().clone();

				if let Some(ref channels) = config.channels {
					let mut channels = channels.clone();

					channels.retain(|element| element != channel);
					channels.push(channel.clone());

					let config = Config { channels: Some(channels), ..config };

					try!(config.save(CONFIG_PATH));
				}
			} else {
				try!(server.send_notice(&sender, &format!("{} is not a valid channel.", &channel)));
			}
		} else {
			try!(server.send_notice(&sender, &parameters.command.help()));
		}
	}

	Ok(())
}

fn cmd_part(parameters: CommandParameters) -> Result<()> {
	let sender = parameters.sender;
	let server = parameters.server;
	let target = parameters.target;

	if let Some(channel) = parameters.args.get("channel").or(Some(&target)) {
		if channel != &sender {
			if channel.starts_with("#") && !channel.contains(",") {
				// TODO: Is it worth writing a more generic function for updating the config?
				let config = server.config().clone();

				if let Some(ref channels) = config.channels {
					let mut channels = channels.clone();

					channels.retain(|element| element != channel);

					let config = Config { channels: Some(channels), ..config };

					try!(config.save(CONFIG_PATH));
				}

				try!(server.send(PART(channel.clone(), None)));
			} else {
				try!(server.send_notice(&sender, &format!("{} is not a valid channel.", &channel)));
			}
		} else {
			try!(server.send_notice(&sender, &parameters.command.help()));
		}
	}

	Ok(())
}
