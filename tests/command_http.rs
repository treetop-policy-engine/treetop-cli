use std::fs;

use assert_cmd::Command;
use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn command() -> Command {
    Command::cargo_bin("treetop-cli").unwrap()
}

fn status_response() -> serde_json::Value {
    json!({
        "policy_configuration": {
            "allow_upload": true,
            "schema_validation_mode": "permissive",
            "policies": {
                "timestamp": "2026-08-12T00:00:00Z",
                "sha256": "policy-hash",
                "size": 10,
                "entries": 1,
                "content": "permit();"
            },
            "labels": {
                "timestamp": "2026-08-12T00:00:00Z",
                "sha256": "label-hash",
                "size": 0,
                "entries": 0,
                "content": ""
            },
            "schema": null
        },
        "parallel_configuration": {
            "cpu_count": 4,
            "workers": 2,
            "allow_parallel": true,
            "rayon_threads": 2,
            "par_threshold": 8
        },
        "request_limits": {
            "max_batch_size": 1024,
            "max_context_bytes": 16384,
            "max_context_depth": 8,
            "max_context_keys": 64
        },
        "request_context": {
            "supported": true,
            "schema_backed": false,
            "fallback_reason": "no_schema"
        }
    })
}

fn version_response() -> serde_json::Value {
    json!({
        "version": "0.0.10",
        "core": {"version": "0.0.19", "cedar": "4.12.0"},
        "policies": {
            "hash": "policy-hash",
            "loaded_at": "2026-08-12T00:00:00Z"
        },
        "schema": null
    })
}

#[tokio::test]
async fn status_json_comes_from_validated_client_types() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_response()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(version_response()))
        .mount(&server)
        .await;

    command()
        .args(["--server-url", &server.uri(), "--json", "status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"max_batch_size\": 1024"))
        .stdout(predicates::str::contains("Server"))
        .stderr(predicates::str::is_empty());
}

#[tokio::test]
async fn authorization_uses_validated_batch_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/authorize"))
        .and(query_param("detail", "brief"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{
                "index": 0,
                "id": "query-0",
                "status": "success",
                "result": {
                    "decision": "Allow",
                    "version": {"hash": "policy-hash", "loaded_at": "2026-08-12T00:00:00Z"},
                    "policy_id": "allow-view"
                }
            }],
            "version": {"hash": "policy-hash", "loaded_at": "2026-08-12T00:00:00Z"},
            "successful": 1,
            "failed": 0
        })))
        .mount(&server)
        .await;

    command()
        .args([
            "--server-url",
            &server.uri(),
            "check",
            "--principal",
            "alice",
            "--action",
            "view",
            "--resource-type",
            "Document",
            "--resource-id",
            "doc-1",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Allow"))
        .stdout(predicates::str::contains("allow-view"));
}

#[tokio::test]
async fn upload_debug_errors_redact_tokens() {
    let server = MockServer::start().await;
    let token = "never-print-this-upload-token";
    Mock::given(method("POST"))
        .and(path("/api/v1/policies"))
        .and(header("x-upload-token", token))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": format!("server reflected {token}")
        })))
        .mount(&server)
        .await;
    let directory = tempdir().unwrap();
    let policy = directory.path().join("policy.cedar");
    fs::write(&policy, "permit(principal, action, resource);").unwrap();

    let output = command()
        .args([
            "--server-url",
            &server.uri(),
            "--debug",
            "upload",
            "--file",
            policy.to_str().unwrap(),
            "--raw",
            "--token",
            token,
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(!stdout.contains(token));
    assert!(!stderr.contains(token));
    assert!(stderr.contains("[REDACTED]"));
}

#[tokio::test]
async fn server_url_precedence_is_cli_then_env_then_config() {
    let config_server = MockServer::start().await;
    let env_server = MockServer::start().await;
    let cli_server = MockServer::start().await;
    for (server, marker) in [
        (&config_server, "config-source"),
        (&env_server, "env-source"),
        (&cli_server, "cli-source"),
    ] {
        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_string(marker))
            .mount(server)
            .await;
    }

    let directory = tempdir().unwrap();
    let config_directory = directory.path().join("treetop-cli");
    fs::create_dir_all(&config_directory).unwrap();
    fs::write(
        config_directory.join("config.toml"),
        format!("server_url = {:?}\n", config_server.uri()),
    )
    .unwrap();

    command()
        .env("XDG_CONFIG_HOME", directory.path())
        .env_remove("TREETOP_CLI_SERVER_URL")
        .arg("metrics")
        .assert()
        .success()
        .stdout(predicates::str::contains("config-source"));

    command()
        .env("XDG_CONFIG_HOME", directory.path())
        .env("TREETOP_CLI_SERVER_URL", env_server.uri())
        .arg("metrics")
        .assert()
        .success()
        .stdout(predicates::str::contains("env-source"));

    command()
        .env("XDG_CONFIG_HOME", directory.path())
        .env("TREETOP_CLI_SERVER_URL", env_server.uri())
        .args(["--server-url", &cli_server.uri(), "metrics"])
        .assert()
        .success()
        .stdout(predicates::str::contains("cli-source"));
}
