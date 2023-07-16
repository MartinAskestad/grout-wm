use crate::win32;
use grout_wm::Result;
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{copy, create_dir, File},
    path::Path,
};

#[derive(Deserialize, Serialize)]
pub struct Config {
    #[serde(rename = "Windows.UI.Core.CoreWindow")]
    pub windows_ui_core_corewindow: Option<Vec<String>>,
    pub class_names: Option<Vec<String>>,
    pub process_names: Option<Vec<String>>,
    pub titles: Option<Vec<String>>,
    #[serde(rename = "layout")]
    pub default_layout: Option<String>,
}

impl std::ops::Add for Config {
    type Output = Config;
    fn add(self, other: Config) -> Config {
        Config {
            windows_ui_core_corewindow: self.windows_ui_core_corewindow,
            class_names: merge_option_vecs(self.class_names, other.class_names),
            process_names: merge_option_vecs(self.process_names, other.process_names),
            titles: merge_option_vecs(self.titles, other.titles),
            default_layout: merge_option_string(self.default_layout, other.default_layout),
        }
    }
}

impl Config {
    pub fn load_default() -> Result<Self> {
        let mut config_path = env::current_exe().expect("Failed to get current executable path");
        config_path.set_file_name("default.yaml");
        info!("Reading config file from {:?}", config_path);
        let config_file = File::open(config_path).expect("Could not open config file");
        let config: Config = serde_yaml::from_reader(config_file).expect("Could not read config");
        Ok(config)
    }

    pub fn load_or_create_user_config(self) -> Result<Self> {
        let mut app_data_path = win32::get_local_appdata_path()?;
        app_data_path.push(env!("CARGO_PKG_NAME"));
        if !Path::new(&app_data_path.clone().into_os_string()).exists() {
            create_dir(app_data_path.clone()).expect("Could not create directory in appdata");
        }
        let mut user_config_path = app_data_path.clone();
        user_config_path.push("config.yaml");
        if !Path::new(&user_config_path.clone().into_os_string()).exists() {
            let mut template_path = env::current_exe()?;
            template_path.set_file_name("user.yaml");
            copy(template_path, user_config_path.clone()).expect("Could not copy user.toml");
        }
        let user_config_file =
            File::open(user_config_path).expect("Could not open user config file");
        let user_config: Config =
            serde_yaml::from_reader(user_config_file).expect("Could not parse user config file");
        Ok(self + user_config)
    }
}

fn merge_option_vecs<T>(a: Option<Vec<T>>, b: Option<Vec<T>>) -> Option<Vec<T>> {
    match (a, b) {
        (Some(mut v1), Some(v2)) => {
            v1.extend(v2);
            Some(v1)
        }
        (Some(v1), None) => Some(v1),
        (None, Some(v2)) => Some(v2),
        (None, None) => None,
    }
}

fn merge_option_string(lhs: Option<String>, rhs: Option<String>) -> Option<String> {
    match (lhs, rhs) {
        (Some(_), Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        _ => None,
    }
}
