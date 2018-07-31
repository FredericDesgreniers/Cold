use std::fs::File;
use std::io::Read;
use toml::from_str;
use failure::Error;

/// General config
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub twitch: TwitchConfig
}

/// Twitch specific config
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TwitchConfig {
	pub username: String,
	pub token: String,
	pub irc_server: String,
	pub channels: Vec<String>,
}

/// Loads a config from a file path
pub fn load_config_toml(path: &str) -> Result<Config, Error> {
	let mut file = File::open(path)?;
	let mut content = String::new();
	file.read_to_string(&mut content)?;
	let config = from_str::<Config>(&content)?;

	Ok(config)
}
