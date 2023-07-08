use log::info;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize, Serialize)]
pub struct Config {
    #[serde(rename = "Windows.UI.Core.CoreWindow")]
    pub windows_ui_core_corewindow: Option<Vec<String>>,
    pub class_names: Option<Vec<String>>,
    pub process_names: Option<Vec<String>>,
    pub titles: Option<Vec<String>>,
}

impl Config {
    pub fn load_default() -> Result<Self, &'static str> {
        let mut config_path = env::current_exe().expect("Failed to get current executable path");
        config_path.set_file_name("default.yaml");
        info!("Reading config file from {:?}", config_path);
        let config_file = std::fs::File::open(config_path).expect("Could not open config file");
        let config: Config = serde_yaml::from_reader(config_file).expect("Could not read config");
        Ok(config)
    }
}
