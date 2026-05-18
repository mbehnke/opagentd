use opagent_core::config::{Config, SecurityLevel};
use opagent_core::security::Operation;

#[test]
fn test_config_defaults() {
    let config = Config::default();
    assert_eq!(config.security.default_level, SecurityLevel::Confirm);
    assert!(config.agent.shell_enabled);
    assert!(config.agent.file_enabled);
    assert!(config.agent.git_enabled);
}

#[test]
fn test_shell_deny_dangerous() {
    let mut config = Config::default();
    config
        .security
        .denied_commands
        .push("rm -rf /".into());

    let op = Operation::Shell {
        command: "rm".into(),
        args: vec!["-rf".into(), "/".into()],
    };

    assert_eq!(op.security_level(&config), SecurityLevel::Deny);
}

#[test]
fn test_shell_default_confirm() {
    let config = Config::default();
    let op = Operation::Shell {
        command: "ls".into(),
        args: vec!["-la".into()],
    };

    assert_eq!(op.security_level(&config), SecurityLevel::Confirm);
}

#[test]
fn test_git_dangerous_confirm() {
    let config = Config::default();
    let op = Operation::Git {
        command: "push --force origin main".into(),
        repo_path: "/tmp/test".into(),
    };

    assert_eq!(op.security_level(&config), SecurityLevel::Confirm);
}

#[test]
fn test_git_safe_auto() {
    let config = Config::default();
    let op = Operation::Git {
        command: "status".into(),
        repo_path: "/tmp/test".into(),
    };

    assert_eq!(op.security_level(&config), SecurityLevel::Auto);
}

#[test]
fn test_file_read_allowed_path() {
    let mut config = Config::default();
    config.security.allowed_paths = vec!["/home".into()];

    let op = Operation::FileRead {
        path: "/home/user/file.txt".into(),
    };

    let level = op.security_level(&config);
    assert!(
        level != SecurityLevel::Deny,
        "File read for allowed path should not be Deny"
    );
}

#[test]
fn test_file_read_disallowed_path() {
    let mut config = Config::default();
    config.security.allowed_paths = vec!["/home".into()];

    let op = Operation::FileRead {
        path: "/etc/shadow".into(),
    };

    assert_eq!(op.security_level(&config), SecurityLevel::Deny);
}

#[test]
fn test_operation_describe() {
    let op = Operation::Shell {
        command: "echo".into(),
        args: vec!["hello".into()],
    };
    assert!(op.describe().contains("shell"));
    assert!(op.describe().contains("echo"));
}

#[test]
fn test_agent_name() {
    let shell = Operation::Shell {
        command: "x".into(),
        args: vec![],
    };
    assert_eq!(shell.agent_name(), "shell");

    let file = Operation::FileRead {
        path: "f".into(),
    };
    assert_eq!(file.agent_name(), "file");

    let git = Operation::Git {
        command: "status".into(),
        repo_path: ".".into(),
    };
    assert_eq!(git.agent_name(), "git");
}

#[test]
fn test_config_load_invalid_path() {
    let result = Config::load("/nonexistent/path/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_config_load_or_default() {
    let config = Config::load_or_default(&[]);
    assert_eq!(config.security.default_level, SecurityLevel::Confirm);
}

#[test]
fn test_serialize_operation() {
    let op = Operation::Shell {
        command: "ls".into(),
        args: vec!["-l".into()],
    };
    let json = serde_json::to_string(&op).unwrap();
    let parsed: Operation = serde_json::from_str(&json).unwrap();
    match parsed {
        Operation::Shell { command, args } => {
            assert_eq!(command, "ls");
            assert_eq!(args, vec!["-l"]);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_llm_config_defaults() {
    let llm = opagent_core::config::LlmConfig::default();
    assert_eq!(llm.provider, opagent_core::config::LlmProvider::Deepseek);
    assert_eq!(llm.model, "deepseek-chat");
    assert!(llm.enabled);
    assert_eq!(llm.temperature, 0.7);
    assert!(llm.api_key.is_none());
    assert!(llm.max_tokens.is_none());
}

#[test]
fn test_llm_provider_base_urls() {
    use opagent_core::config::LlmProvider;
    assert!(LlmProvider::Deepseek.default_base_url().contains("deepseek"));
    assert!(LlmProvider::OpenAI.default_base_url().contains("openai"));
    assert!(LlmProvider::Ollama.default_base_url().contains("localhost"));
    assert!(LlmProvider::Custom.default_base_url().is_empty());
}

#[test]
fn test_llm_config_base_url_or_default() {
    let mut llm = opagent_core::config::LlmConfig::default();
    assert_eq!(llm.base_url_or_default(), "https://api.deepseek.com");

    llm.base_url = Some("https://custom.api.example.com".into());
    assert_eq!(llm.base_url_or_default(), "https://custom.api.example.com");
}

#[test]
fn test_llm_config_debug_masks_api_key() {
    let mut llm = opagent_core::config::LlmConfig::default();
    llm.api_key = Some("sk-secret-key".into());
    let debug_str = format!("{:?}", llm);
    assert!(!debug_str.contains("sk-secret-key"));
    assert!(debug_str.contains("***"));
}

#[test]
fn test_llm_serialize_skips_api_key() {
    let mut llm = opagent_core::config::LlmConfig::default();
    llm.api_key = Some("sk-secret-key".into());
    let json = serde_json::to_string(&llm).unwrap();
    assert!(!json.contains("sk-secret-key"));
    assert!(!json.contains("api_key"));
}

#[test]
fn test_llm_deserialize_from_toml() {
    let toml_str = r#"
[llm]
provider = "openai"
model = "gpt-4o"
api_key = "sk-test123"
temperature = 0.3
max_tokens = 2048
enabled = true
"#;
    #[derive(serde::Deserialize)]
    struct Wrapper {
        llm: opagent_core::config::LlmConfig,
    }
    let wrapper: Wrapper = toml::from_str(toml_str).unwrap();
    assert_eq!(wrapper.llm.provider, opagent_core::config::LlmProvider::OpenAI);
    assert_eq!(wrapper.llm.model, "gpt-4o");
    assert_eq!(wrapper.llm.api_key.as_deref(), Some("sk-test123"));
    assert_eq!(wrapper.llm.temperature, 0.3);
    assert_eq!(wrapper.llm.max_tokens, Some(2048));
}
