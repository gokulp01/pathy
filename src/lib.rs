use std::path::PathBuf;

use zed_extension_api as zed;

const LANGUAGE_SERVER_ID: &str = "pathy";

struct PathyExtension;

impl zed::Extension for PathyExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err("unknown language server".to_string());
        }

        let root = PathBuf::from(worktree.root_path());
        let server_path = server_binary_path(&root);

        let command = zed::Command::new(server_path.to_string_lossy())
            .envs(worktree.shell_env());

        Ok(command)
    }
}

fn server_binary_path(root: &PathBuf) -> PathBuf {
    let mut path = root
        .join("server")
        .join("target")
        .join("debug")
        .join("pathy-server");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

zed::register_extension!(PathyExtension);
