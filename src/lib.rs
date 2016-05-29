extern crate irc;

use std::io::Result;
use std::collections::HashMap;
use irc::client::prelude::*;

pub static CONFIG_PATH: &'static str = "config.json";

pub struct Bot {
	cmds: HashMap<String, Box<Fn(Vec<String>, IrcServer, String, String) -> Result<()>>>,
	owner_cmds: HashMap<String, Box<Fn(Vec<String>, IrcServer, String, String) -> Result<()>>>,
	server: IrcServer,
}

// TODO: Look into extending irc::client::server
impl Bot {
	pub fn new(config: Config) -> Result<Bot> {
		let mut bot = Bot {
			cmds: HashMap::new(),
			owner_cmds: HashMap::new(),
			server: try!(IrcServer::from_config(config)),
		};

		// TODO: Ideally each command should be a generic struct so we can store that in the hashmap instead
		// {owner_only: bool, group: String, handler: Fn, args: Vec<(required: bool, name: Str)> }
		bot.cmds.insert("echo".into(), Box::new(cmd_echo));
		bot.cmds.insert("say".into(), Box::new(cmd_echo));

		// TODO: Restrict commands to groups (users.json?), owners should override all restrictions
		bot.owner_cmds.insert("kick".into(), Box::new(cmd_kick));
		bot.owner_cmds.insert("join".into(), Box::new(cmd_join));
		bot.owner_cmds.insert("part".into(), Box::new(cmd_part));

		Ok(bot)
	}

	pub fn run(&mut self) {
		let server = self.server.clone();

		server.identify().unwrap();

		for message in server.iter() {
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

	fn handle_privmsg(&mut self, target: String, text: String, sender: String) {
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

	fn handle_command(&mut self, text: String, target: String, sender: String) -> Result<()> {
		let mut strs = text.split_whitespace();
		let command = strs.next().unwrap();
		let args = strs.map(|s| s.into()).collect::<Vec<String>>();

		// TODO: Pack command data into a struct to avoid unused warnings and benefit from typed variable
		if let Some(command) = self.cmds.get(command) {
			try!(command(args, self.server.clone(), target, sender));
		} else if let Some(command) = self.owner_cmds.get(command) {
			if self.server.config().is_owner(&sender) {
				try!(command(args, self.server.clone(), target, sender));
			} else {
				try!(self.server.send_notice(&sender, "You don't have permission to do that!"))
			}
		} else if !target.starts_with("#") {
			try!(self.server.send_privmsg(&target, "Unknown command."));
		}

		Ok(())
	}
}

#[allow(unused)]
fn cmd_echo(args: Vec<String>, server: IrcServer, target: String, sender: String) -> Result<()> {
	try!(server.send_privmsg(&target, &args.join(" ")));
	Ok(())
}

fn cmd_kick(args: Vec<String>, server: IrcServer, target: String, sender: String) -> Result<()> {
	if sender != target {
		let who = args.first();

		if let Some(who) = who {
			let mut cmd_args = args.clone();
			try!(server.send_kick(&target, &who, &cmd_args.split_off(1).join(" ")))
		} else {
			try!(server.send_notice(&sender, "Command arguments are !kick <nick> [reason]"))
		}
	} else {
		try!(server.send_privmsg(&sender, "You can't kick people from a private chat..."))
	}

	Ok(())
}

#[allow(unused)]
fn cmd_join(args: Vec<String>, server: IrcServer, target: String, sender: String) -> Result<()> {
	let channel = args.first();

	if let Some(channel) = channel {
		if channel.starts_with("#") && !channel.contains(",") {
			try!(server.send_join(channel));

			let config = server.config().clone();

			if let Some(ref channels) = config.channels {
				let mut channels = channels.clone();

				channels.retain(|element| element != channel);
				channels.push(channel.to_string());

				let config = Config { channels: Some(channels), ..config };

				try!(config.save(CONFIG_PATH));
			}
		} else {
			try!(server.send_notice(&sender, &format!("{} is not a valid channel.", &channel)));
		}
	} else {
		try!(server.send_notice(&sender, "Command arguments are !join <channel>"));
	}

	Ok(())
}

fn cmd_part(args: Vec<String>, server: IrcServer, target: String, sender: String) -> Result<()> {
	let mut channel = args.first();

	if channel.is_none() && target != sender {
		channel = Some(&target);
	}

	if let Some(channel) = channel {
		if channel.starts_with("#") {
			try!(server.send(Command::PART(channel.to_string(), None)));

			let config = server.config().clone();

			if let Some(ref channels) = config.channels {
				let mut channels = channels.clone();

				channels.retain(|element| element != channel);

				let config = Config { channels: Some(channels), ..config };

				try!(config.save(CONFIG_PATH));
			}
		} else {
			try!(server.send_notice(&sender, &format!("{} is not a valid channel.", &channel)));
		}
	} else {
		try!(server.send_notice(&sender, "Command arguments are !part <channel>"));
	}

	Ok(())
}
