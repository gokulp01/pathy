use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextGating {
    Off,
    Smart,
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseDirStrategy {
    FileDir,
    WorkspaceRoot,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRootStrategy {
    LspRootUri,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatStrategy {
    None,
    Lazy,
    Eager,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub enable: bool,
    pub path_prefix_fallback: bool,
    pub context_gating: ContextGating,
    pub base_dir: BaseDirStrategy,
    pub workspace_root_strategy: WorkspaceRootStrategy,
    pub max_results: usize,
    pub show_hidden: bool,
    pub include_files: bool,
    pub include_directories: bool,
    pub directory_trailing_slash: bool,
    pub ignore_globs: Vec<String>,
    pub prefer_forward_slashes: bool,
    pub expand_tilde: bool,
    pub windows_enable_drive_prefix: bool,
    pub windows_enable_unc: bool,
    pub cache_ttl_ms: u64,
    pub cache_max_dirs: usize,
    pub stat_strategy: StatStrategy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable: true,
            path_prefix_fallback: true,
            context_gating: ContextGating::Smart,
            base_dir: BaseDirStrategy::FileDir,
            workspace_root_strategy: WorkspaceRootStrategy::LspRootUri,
            max_results: 80,
            show_hidden: false,
            include_files: true,
            include_directories: true,
            directory_trailing_slash: true,
            ignore_globs: vec![
                "**/.git/**".into(),
                "**/.venv/**".into(),
                "**/venv/**".into(),
                "**/__pycache__/**".into(),
                "**/.pytest_cache/**".into(),
                "**/.mypy_cache/**".into(),
                "**/.ruff_cache/**".into(),
                "**/node_modules/**".into(),
            ],
            prefer_forward_slashes: true,
            expand_tilde: true,
            windows_enable_drive_prefix: true,
            windows_enable_unc: true,
            cache_ttl_ms: 500,
            cache_max_dirs: 64,
            stat_strategy: StatStrategy::Lazy,
        }
    }
}

pub fn select_settings_root<'a>(value: &'a Value) -> Option<&'a Value> {
    let mut current = value;
    if let Some(settings) = current.get("settings") {
        current = settings;
    }
    if let Some(lsp) = current.get("lsp") {
        if let Some(server) = lsp.get("pathy") {
            if let Some(settings) = server.get("settings") {
                return Some(settings);
            }
            return Some(server);
        }
    }
    if let Some(pathy) = current.get("pathy") {
        return Some(pathy);
    }
    current.as_object().map(|_| current)
}

pub fn load_config(value: &Value, warned: &mut bool) -> Config {
    let mut config = Config::default();
    let Some(root) = select_settings_root(value) else {
        return config;
    };
    let Some(map) = root.as_object() else {
        return config;
    };

    let mut warnings = Vec::new();
    for (key, val) in map {
        match key.as_str() {
            "enable" => set_bool(&mut config.enable, val, key, &mut warnings),
            "path_prefix_fallback" => {
                set_bool(&mut config.path_prefix_fallback, val, key, &mut warnings)
            }
            "context_gating" => {
                if let Some(s) = val.as_str() {
                    config.context_gating = match s {
                        "off" => ContextGating::Off,
                        "smart" => ContextGating::Smart,
                        "strict" => ContextGating::Strict,
                        _ => {
                            warnings.push(format!("invalid context_gating: {s}"));
                            config.context_gating
                        }
                    };
                } else {
                    warnings.push(format!("invalid context_gating type"));
                }
            }
            "base_dir" => {
                if let Some(s) = val.as_str() {
                    config.base_dir = match s {
                        "file_dir" => BaseDirStrategy::FileDir,
                        "workspace_root" => BaseDirStrategy::WorkspaceRoot,
                        "both" => BaseDirStrategy::Both,
                        _ => {
                            warnings.push(format!("invalid base_dir: {s}"));
                            config.base_dir
                        }
                    };
                } else {
                    warnings.push(format!("invalid base_dir type"));
                }
            }
            "workspace_root_strategy" => {
                if let Some(s) = val.as_str() {
                    config.workspace_root_strategy = match s {
                        "lsp_root_uri" => WorkspaceRootStrategy::LspRootUri,
                        "disabled" => WorkspaceRootStrategy::Disabled,
                        _ => {
                            warnings.push(format!("invalid workspace_root_strategy: {s}"));
                            config.workspace_root_strategy
                        }
                    };
                } else {
                    warnings.push(format!("invalid workspace_root_strategy type"));
                }
            }
            "max_results" => set_usize(&mut config.max_results, val, key, &mut warnings),
            "show_hidden" => set_bool(&mut config.show_hidden, val, key, &mut warnings),
            "include_files" => set_bool(&mut config.include_files, val, key, &mut warnings),
            "include_directories" => {
                set_bool(&mut config.include_directories, val, key, &mut warnings)
            }
            "directory_trailing_slash" => set_bool(
                &mut config.directory_trailing_slash,
                val,
                key,
                &mut warnings,
            ),
            "ignore_globs" => {
                if let Some(list) = val.as_array() {
                    let mut globs = Vec::new();
                    for entry in list {
                        if let Some(s) = entry.as_str() {
                            globs.push(s.to_string());
                        } else {
                            warnings.push("invalid ignore_globs entry".into());
                        }
                    }
                    if !globs.is_empty() {
                        config.ignore_globs = globs;
                    }
                } else {
                    warnings.push("invalid ignore_globs type".into());
                }
            }
            "prefer_forward_slashes" => {
                set_bool(&mut config.prefer_forward_slashes, val, key, &mut warnings)
            }
            "expand_tilde" => set_bool(&mut config.expand_tilde, val, key, &mut warnings),
            "windows_enable_drive_prefix" => set_bool(
                &mut config.windows_enable_drive_prefix,
                val,
                key,
                &mut warnings,
            ),
            "windows_enable_unc" => {
                set_bool(&mut config.windows_enable_unc, val, key, &mut warnings)
            }
            "cache_ttl_ms" => set_u64(&mut config.cache_ttl_ms, val, key, &mut warnings),
            "cache_max_dirs" => set_usize(&mut config.cache_max_dirs, val, key, &mut warnings),
            "stat_strategy" => {
                if let Some(s) = val.as_str() {
                    config.stat_strategy = match s {
                        "none" => StatStrategy::None,
                        "lazy" => StatStrategy::Lazy,
                        "eager" => StatStrategy::Eager,
                        _ => {
                            warnings.push(format!("invalid stat_strategy: {s}"));
                            config.stat_strategy
                        }
                    };
                } else {
                    warnings.push("invalid stat_strategy type".into());
                }
            }
            _ => {}
        }
    }

    if !warnings.is_empty() && !*warned {
        eprintln!("pathy-server: config warnings: {}", warnings.join("; "));
        *warned = true;
    }

    config
}

fn set_bool(target: &mut bool, value: &Value, key: &str, warnings: &mut Vec<String>) {
    if let Some(v) = value.as_bool() {
        *target = v;
    } else {
        warnings.push(format!("invalid {key} type"));
    }
}

fn set_usize(target: &mut usize, value: &Value, key: &str, warnings: &mut Vec<String>) {
    if let Some(v) = value.as_u64() {
        *target = v as usize;
    } else {
        warnings.push(format!("invalid {key} type"));
    }
}

fn set_u64(target: &mut u64, value: &Value, key: &str, warnings: &mut Vec<String>) {
    if let Some(v) = value.as_u64() {
        *target = v;
    } else {
        warnings.push(format!("invalid {key} type"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn loads_defaults_when_missing() {
        let mut warned = false;
        let cfg = load_config(&json!({}), &mut warned);
        assert!(cfg.enable);
        assert_eq!(cfg.max_results, 80);
    }

    #[test]
    fn applies_overrides() {
        let mut warned = false;
        let cfg = load_config(
            &json!({
                "enable": false,
                "max_results": 20,
                "context_gating": "strict",
                "ignore_globs": ["**/.git/**"]
            }),
            &mut warned,
        );
        assert!(!cfg.enable);
        assert_eq!(cfg.max_results, 20);
        assert_eq!(cfg.context_gating, ContextGating::Strict);
        assert_eq!(cfg.ignore_globs.len(), 1);
    }

    #[test]
    fn handles_nested_settings() {
        let mut warned = false;
        let cfg = load_config(
            &json!({
                "settings": {
                    "lsp": {
                        "pathy": {
                            "settings": {
                                "show_hidden": true
                            }
                        }
                    }
                }
            }),
            &mut warned,
        );
        assert!(cfg.show_hidden);
    }
}
