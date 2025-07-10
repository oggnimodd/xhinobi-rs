use std::env;

pub fn is_gitpod() -> bool {
    env::var("GITPOD_WORKSPACE_ID").is_ok()
}

pub fn is_codespace() -> bool {
    env::var("CODESPACE_NAME").is_ok() && env::var("CLOUDENV_ENVIRONMENT_ID").is_ok()
}

pub fn is_google_cloud() -> bool {
    env::var("DEVSHELL_GCLOUD_CONFIG").is_ok() || env::var("BASHRC_GOOGLE_PATH").is_ok()
}

pub fn is_cloud_environment() -> bool {
    is_gitpod() || is_codespace() || is_google_cloud()
}

pub const TEMP_FILE_NAME: &str = "tempfile.txt";