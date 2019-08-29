macro_rules! concat_path {
    ($fragment:expr) => {
        concat!($fragment, "/")
    };
    ($fragment:expr, $($fragments:expr),+) => {
        concat!($fragment, "/", concat_path!($($fragments),+))
    }
}

macro_rules! asset_file {
    ($asset:expr) => {
        concat!(concat_path!("..", "assets"), $asset)
    }
}

macro_rules! include_asset {
    ($asset:expr) => {
        include_bytes!(asset_file!($asset))
    }
}

#[cfg(target_os = "mac_os")]
macro_rules! shader_file {
    ($asset:expr) => {
        concat!(concat_path!("..", "shaders", "330"), $asset)
    }
}

#[cfg(target_os = "windows")]
macro_rules! shader_file {
    ($asset:expr) => {
        concat!(concat_path!("..", "assets"), $asset)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
macro_rules! shader_file {
    ($shader:expr) => {
        concat!(concat_path!("..", "shaders", "420"), $shader)
    }
}

macro_rules! include_shader {
    ($shader:expr) => {
        include_str!(shader_file!($shader))
    }
}

macro_rules! include_code {
    ($code_fragment:expr) => {
        include!(concat!(concat_path!("..", ".code_fragments"), $code_fragment))
    }
}
