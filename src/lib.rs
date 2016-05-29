extern crate irc;

use std::io::Result;
use std::collections::HashMap;
use irc::client::prelude::*;

pub struct Bot {
	cmds: HashMap<String, Box<Fn(&Vec<&str>, &IrcServer, &String) -> Result<()>>>,
	server: IrcServer,
}

impl Bot {
	pub fn new(config: Config) -> Result<Bot> {
		let mut bot = Bot {
			cmds: HashMap::new(),
			server: try!(IrcServer::from_config(config)),
		};

		bot.cmds.insert("echo".to_string(), Box::new(handle_cmd_echo));
		bot.cmds.insert("say".to_string(), Box::new(handle_cmd_echo));

		Ok(bot)
	}

	pub fn run(&mut self) {
		let server = self.server.clone();

		server.identify().unwrap();

		for message in server.iter() {
			let message = message.unwrap();

			println!("{}", message);

			match message.command {
				Command::PRIVMSG(ref target, ref text) => {
					if let Some(sender) = message.source_nickname() {
						self.handle_privmsg(&target, &text, &sender.to_string())
					}
				}
				_ => {}
			}
		}
	}

	fn handle_privmsg(&mut self, target: &String, text: &String, sender: &String) {
		let is_private = self.server.current_nickname() == target;
		let is_bang = text.starts_with("!");
		let is_command = is_private || is_bang;

		if !is_command || text.len() == 1 {
			return;
		}

		let text = if is_bang { &text[1..] } else { text };
		let target = if is_private { sender } else { target };

		if let Err(err) = self.handle_command(&text.to_string(), &target.to_string()) {
			println!("Failed to process command: {:?}", err);
		}
	}

	fn handle_command(&mut self, text: &String, target: &String) -> Result<()> {
		let mut strs = text.split_whitespace();
		let command = strs.next().unwrap();
		let args = strs.collect::<Vec<&str>>();

		match self.cmds.get(command) {
			Some(command) => try!(command(&args, &self.server, target)),
			_ => {
				if !target.starts_with("#") {
					try!(self.server.send_privmsg(target, "Unknown command."));
				}
			}
		}

		Ok(())
	}
}

fn handle_cmd_echo(args: &Vec<&str>, server: &IrcServer, target: &String) -> Result<()> {
	try!(server.send_privmsg(target, &args.join(" ")));
	Ok(())
}
