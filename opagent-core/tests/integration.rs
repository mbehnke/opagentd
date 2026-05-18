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
