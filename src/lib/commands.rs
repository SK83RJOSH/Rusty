use std::io::Error;
use std::io::Result;
use std::io::ErrorKind::InvalidInput;
use std::collections::HashMap;

use irc::client::server::IrcServer;
use irc::client::server::Server;
use irc::client::prelude::ServerExt;

pub struct CommandArg {
	pub required: bool,
	pub name: String,
}

pub struct CommandParameters<'a> {
	pub command: &'a Command,
	pub args: HashMap<String, String>,
	pub server: &'a IrcServer,
	pub target: String,
	pub sender: String,
}

pub struct Command {
	pub owner_only: bool,
	pub group: String,
	pub args: Vec<CommandArg>,
	pub handler: Box<Fn(CommandParameters) -> Result<()>>,
}

impl Command {
	pub fn new(owner_only: bool,
	           group: String,
	           args: Vec<CommandArg>,
	           handler: Box<Fn(CommandParameters) -> Result<()>>)
	           -> Self {
		Command {
			owner_only: owner_only,
			group: group,
			args: args,
			handler: handler,
		}
	}

	pub fn arguments(&self, input: String) -> Result<HashMap<String, String>> {
		let mut map = HashMap::new();
		let mut args = self.args.iter();
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

	pub fn help(&self) -> String {
		let mut help: String = "USAGE".into();
		let mut args = self.args.iter();

		while let Some(arg) = args.next() {
			help.push_str(" ");
			help.push_str(if arg.required { "<" } else { "[" });
			help.push_str(&arg.name);
			help.push_str(if arg.required { ">" } else { "]" });
		}

		return help;
	}

	pub fn execute(&self,
	               input: String,
	               server: &IrcServer,
	               target: String,
	               sender: String)
	               -> Result<()> {
		// TODO: Restrict by group (groups.json?)
		if !self.owner_only || server.config().is_owner(&sender) {
			let args = self.arguments(input);

			if let Ok(args) = args {
				let ref handler = self.handler;

				try!(handler(CommandParameters {
					command: self,
					args: args,
					server: server,
					target: target,
					sender: sender,
				}));
			} else if let Err(_) = args {
				try!(server.send_notice(&sender, &self.help()));
			}
		} else {
			try!(server.send_notice(&sender, "You don't have permission to use that command."));
		}

		Ok(())
	}
}
