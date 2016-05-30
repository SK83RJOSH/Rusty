use std::io::Error;
use std::io::Result;
use std::io::ErrorKind::InvalidInput;
use std::collections::HashMap;

use irc::client::server::IrcServer;
use irc::client::server::Server;
use irc::client::prelude::ServerExt;
// use irc::client::prelude::Config;
use irc::client::data;

pub struct CommandArg {
	required: bool,
	name: String,
}

pub struct CommandParameters<'a> {
	args: HashMap<String, String>,
	server: &'a IrcServer,
	target: String,
	sender: String,
}

pub trait Command {
	fn owner_only(&self) -> &bool;
	fn group(&self) -> &String;
	fn args(&self) -> &Vec<CommandArg>;

	fn build_arguments(&self, input: String) -> Result<HashMap<String, String>> {
		let mut map = HashMap::new();
		let mut args = self.args().iter();
		let col = input.split_whitespace().map(|word| word.into()).collect::<Vec<String>>();
		let mut words = col.iter();
		let mut last_arg: Option<String> = None;

		while let Some(arg) = args.next() {
			last_arg = Some(arg.name.clone());

			if let Some(word) = words.next() {
				map.insert(arg.name.clone(), word.clone());
			} else if arg.required {
				return Err(Error::new(InvalidInput, format!("{} is required.", arg.name)));
			}
		}

		// Append anything additional to the last argument (general use-case?)
		if let Some(arg) = last_arg {
			if let Some(suffix) = map.to_owned().get(&arg) {
				let mut suffix = suffix.clone();

				while let Some(word) = words.next() {
					suffix.push_str(" ");
					suffix.push_str(word);
				}

				map.insert(arg.clone(), suffix.clone());
			}
		}

		Ok(map)
	}

	fn build_help(&self) -> String {
		let mut help: String = "USAGE".into();
		let mut args = self.args().iter();

		while let Some(arg) = args.next() {
			help.push_str(" ");
			help.push_str(if arg.required { "<" } else { "[" });
			help.push_str(&arg.name);
			help.push_str(if arg.required { ">" } else { "]" });
		}

		return help;
	}

	fn execute(&self,
	           input: String,
	           server: &IrcServer,
	           target: String,
	           sender: String)
	           -> Result<()> {

		// TODO Restrict by group (groups.json?)
		if !self.owner_only() || server.config().is_owner(&sender) {
			let args = self.build_arguments(input);

			if let Ok(args) = args {
				try!(self.handle(CommandParameters {
					args: args,
					server: server,
					target: target,
					sender: sender,
				}));
			} else if let Err(_) = args {
				try!(server.send_notice(&sender, &self.build_help()));
			}
		} else {
			try!(server.send_notice(&sender, "You don't have permission to use that command."));
		}

		Ok(())
	}

	fn handle(&self, parameters: CommandParameters) -> Result<()>;
}

pub struct EchoCommand {
	owner_only: bool,
	group: String,
	args: Vec<CommandArg>,
}

impl EchoCommand {
	pub fn new() -> EchoCommand {
		EchoCommand {
			owner_only: false,
			group: "".into(),
			args: vec![CommandArg {
				           required: true,
				           name: "message".into(),
			           }],
		}
	}
}

impl Command for EchoCommand {
	fn owner_only(&self) -> &bool { return &self.owner_only; }
	fn group(&self) -> &String { return &self.group; }
	fn args(&self) -> &Vec<CommandArg> { return &self.args; }

	fn handle(&self, parameters: CommandParameters) -> Result<()> {
		if let Some(message) = parameters.args.get("message") {
			try!(parameters.server.send_privmsg(&parameters.target, message));
		}

		Ok(())
	}
}

pub struct KickCommand {
	owner_only: bool,
	group: String,
	args: Vec<CommandArg>,
}

impl KickCommand {
	pub fn new() -> KickCommand {
		KickCommand {
			owner_only: true,
			group: "admin".into(),
			args: vec![CommandArg {
				           required: true,
				           name: "nick".into(),
			           },
			           CommandArg {
				           required: false,
				           name: "reason".into(),
			           }],
		}
	}
}

impl Command for KickCommand {
	fn owner_only(&self) -> &bool { return &self.owner_only; }
	fn group(&self) -> &String { return &self.group; }
	fn args(&self) -> &Vec<CommandArg> { return &self.args; }

	fn handle(&self, parameters: CommandParameters) -> Result<()> {
		if parameters.sender != parameters.target {
			if let Some(nick) = parameters.args.get("nick") {
				if let Some(reason) = parameters.args
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
}

pub struct JoinCommand {
	owner_only: bool,
	group: String,
	args: Vec<CommandArg>,
}

impl JoinCommand {
	pub fn new() -> JoinCommand {
		JoinCommand {
			owner_only: false,
			group: "".into(),
			args: vec![CommandArg {
				           required: false,
				           name: "channel".into(),
			           }],
		}
	}
}

impl Command for JoinCommand {
	fn owner_only(&self) -> &bool { return &self.owner_only; }
	fn group(&self) -> &String { return &self.group; }
	fn args(&self) -> &Vec<CommandArg> { return &self.args; }

	fn handle(&self, parameters: CommandParameters) -> Result<()> {
		let sender = parameters.sender;
		let server = parameters.server;
		let target = parameters.target;

		if let Some(channel) = parameters.args.get("channel").or(Some(&target)) {
			if channel != &sender {
				if channel.starts_with("#") && !channel.contains(",") {
					try!(server.send_join(channel));

					// TODO: Is it worth writing a more generic function for updating the config?
					// let config = server.config().clone();
					//
					// if let Some(ref channels) = config.channels {
					// 	let mut channels = channels.clone();
					//
					// 	channels.retain(|element| element != channel);
					// 	channels.push(channel.clone());
					//
					// 	let config = Config { channels: Some(channels), ..config };
					//
					// 	try!(config.save(CONFIG_PATH));
					// }
				} else {
					try!(server.send_notice(&sender, &format!("{} is not a valid channel.", &channel)));
				}
			} else {
				try!(server.send_notice(&sender, &self.build_help()));
			}
		}

		Ok(())
	}
}

pub struct PartCommand {
	owner_only: bool,
	group: String,
	args: Vec<CommandArg>,
}

impl PartCommand {
	pub fn new() -> PartCommand {
		PartCommand {
			owner_only: false,
			group: "".into(),
			args: vec![CommandArg {
				           required: false,
				           name: "channel".into(),
			           }],
		}
	}
}

impl Command for PartCommand {
	fn owner_only(&self) -> &bool { return &self.owner_only; }
	fn group(&self) -> &String { return &self.group; }
	fn args(&self) -> &Vec<CommandArg> { return &self.args; }

	fn handle(&self, parameters: CommandParameters) -> Result<()> {
		let sender = parameters.sender;
		let server = parameters.server;
		let target = parameters.target;

		if let Some(channel) = parameters.args.get("channel").or(Some(&target)) {
			if channel != &sender {
				if channel.starts_with("#") && !channel.contains(",") {
					try!(server.send(data::Command::PART(channel.clone(), None)));

					// TODO: Is it worth writing a more generic function for updating the config?
					// let config = server.config().clone();
					//
					// if let Some(ref channels) = config.channels {
					// 	let mut channels = channels.clone();
					//
					// 	channels.retain(|element| element != channel);
					//
					// 	let config = Config { channels: Some(channels), ..config };
					//
					// 	try!(config.save(CONFIG_PATH));
					// }
				} else {
					try!(server.send_notice(&sender, &format!("{} is not a valid channel.", &channel)));
				}
			} else {
				try!(server.send_notice(&sender, &self.build_help()));
			}
		}

		Ok(())
	}
}
