use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use toml;


#[derive(Clone, Deserialize, Serialize)]
pub struct PathConfig {
    pub config_home: PathBuf,
    pub bin_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl PathConfig {
    pub fn new<P1, P2, P3>(
        config_home: P1, bin_dir: P2, data_dir: P3) -> PathConfig
        where P1: AsRef<Path>,
              P2: AsRef<Path>,
              P3: AsRef<Path> {

        PathConfig {
            config_home: PathBuf::from(config_home.as_ref()),
            bin_dir: PathBuf::from(bin_dir.as_ref()),
            data_dir: PathBuf::from(data_dir.as_ref()),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FileConfig {
    pub log_file: String,
    pub shader_path: PathBuf,
    pub shader_version: PathBuf,
    pub asset_path: PathBuf,
    platform: Platform,
}

#[derive(Clone, Deserialize, Serialize)]
struct Platform {
    macos: MacOS,
    windows: Windows,
    linux: Linux,
}

#[derive(Clone, Deserialize, Serialize)]
struct MacOS {
    shader_version: PathBuf
}

#[derive(Clone, Deserialize, Serialize)]
struct Windows {
    shader_version: PathBuf,
}

#[derive(Clone, Deserialize, Serialize)]
struct Linux {
    shader_version: PathBuf,
}

#[derive(Clone, Debug)]
pub enum Error {
    ConfigFileNotFound(String),
    CouldNotReadConfig(String),
    Deserialize(toml::de::Error),
}

fn get_content<P: AsRef<Path>>(path: &P) -> Result<String, Error> {
    let mut file = match File::open(path) {
        Ok(val) => val,
        Err(_) => {
            return Err(Error::ConfigFileNotFound(format!("{}", path.as_ref().display())));
        }
    };

    let mut content = String::new();
    let _bytes_read = match file.read_to_string(&mut content) {
        Ok(val) => val,
        Err(_) => {
            return Err(Error::CouldNotReadConfig(format!("{}", path.as_ref().display())));
        }
    };

    Ok(content)
}


#[cfg(target_os = "macos")]
#[inline]
fn __platform_config(config: &mut FileConfig) {
    config.shader_version = config.platform.macos.shader_version.clone();
}

#[cfg(target_os = "windows")]
#[inline]
fn __platform_config(config: &mut FileConfig) {
    config.shader_version = config.platform.windows.shader_version.clone();
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[inline]
fn __platform_config(config: &mut FileConfig) {
    config.shader_version = config.platform.linux.shader_version.clone();
}

pub fn load<P: AsRef<Path>>(path: P) -> Result<FileConfig, Error> {
    let content = get_content(&path)?;
    let mut config = match toml::from_str(&content) {
        Ok(val) => val,
        Err(e) => return Err(Error::Deserialize(e)),
    };

    __platform_config(&mut config);

    Ok(config)
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ProgramConfig {
    pub config_home: PathBuf,
    pub bin_dir: PathBuf,
    pub data_dir: PathBuf,
    pub log_file: PathBuf,
    pub shader_path: PathBuf,
    pub shader_version: PathBuf,
    pub asset_path: PathBuf,
}

impl ProgramConfig {
    pub fn new(path_config: PathConfig, file_config: FileConfig) -> Self {
        Self {
            config_home: path_config.config_home,
            bin_dir: path_config.bin_dir,
            data_dir: path_config.data_dir.clone(),
            log_file: path_config.data_dir.join(Path::new(&file_config.log_file)),
            shader_path: path_config.data_dir.join(file_config.shader_path),
            shader_version: file_config.shader_version,
            asset_path: path_config.data_dir.join(file_config.asset_path),
        }
    }
}
