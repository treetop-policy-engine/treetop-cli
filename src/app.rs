use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context as AnyContext, Result};
use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use colored::Colorize;
use serde::Deserialize;
use serde::Serialize;
use treetop_client::{
    Action, AttrValue, AuthRequest, AuthorizeRequest, Client, Group, PoliciesDownload,
    PoliciesMetadata, Principal, ReadOnly, Request, Resource, SchemaDownload, StatusResponse,
    UploadToken, User, UserPolicies,
};
use uuid::Uuid;

use crate::cli_config::CliConfig;
use crate::matrix::expand_matrix;
use crate::models::{
    CliDisplay, InputAttrValue, LastUsedValues, TableStyle, display_authorization,
};
use crate::paths::{cli_config_path, cli_history_path};
use crate::repl::run_repl;
use crate::style::{error, help_line, settings_line, status_flag, title, version, warning, yes_no};

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:9999";

struct ExecContext {
    client: Client<ReadOnly>,
    server_url: String,
    connection_label: String,
    show_json: bool,
    show_debug: bool,
    show_timing: bool,
    last_used: LastUsedValues,
    correlation_id: String,
    table_style: TableStyle,
}

impl ExecContext {
    fn set_correlation_id(&mut self, correlation_id: String) -> Result<()> {
        self.client = self.client.with_correlation_id(correlation_id.clone())?;
        self.correlation_id = correlation_id;
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "treetop-cli",
    about = "CLI and interactive REPL for the Treetop API",
    version = env!("TREETOP_CLI_VERSION")
)]
struct Cli {
    /// Complete server base URL, including scheme and optional port.
    #[arg(long, env = "TREETOP_CLI_SERVER_URL", global = true)]
    server_url: Option<String>,
    /// Legacy server host setting, combined with --port using HTTP.
    #[arg(long, env = "TREETOP_CLI_SERVER_ADDRESS", global = true)]
    host: Option<String>,
    /// Legacy server port setting, combined with --host using HTTP.
    #[arg(long, env = "TREETOP_CLI_SERVER_PORT", global = true)]
    port: Option<u16>,
    /// Print validated responses as JSON.
    #[arg(long, env = "TREETOP_CLI_JSON", global = true)]
    json: bool,
    /// Print typed requests and validated results or errors.
    #[arg(long, env = "TREETOP_CLI_DEBUG", global = true)]
    debug: bool,
    /// Print command execution timing.
    #[arg(long, env = "TREETOP_CLI_TIMING", global = true)]
    timing: bool,
    /// Table style: rounded (default), ascii, unicode, or markdown.
    #[arg(
        long,
        value_parser = clap::value_parser!(TableStyle),
        env = "TREETOP_CLI_TABLE_STYLE",
        global = true
    )]
    table_style: Option<TableStyle>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the interactive REPL.
    Repl,
    /// Get service status.
    Status,
    /// Check one or more matrix-expanded requests against the loaded policies.
    Check {
        /// Principal to evaluate. Supports alternatives such as alice|bob.
        #[arg(long)]
        principal: Option<String>,
        /// Action to evaluate. Supports alternatives such as create|delete.
        #[arg(long)]
        action: Option<String>,
        /// Resource type. Supports alternatives such as Host|Document.
        #[arg(long = "resource-type")]
        resource_type: Option<String>,
        /// Resource ID. Supports alternatives such as host1|host2.
        #[arg(long = "resource-id")]
        resource_id: Option<String>,
        /// Repeatable resource attribute in key=value form.
        #[arg(long = "resource-attribute", value_parser = parse_kv)]
        attrs: Vec<(String, InputAttrValue)>,
        /// Repeatable request-context attribute in key=value form.
        #[arg(long = "context-attribute", value_parser = parse_kv)]
        context_attrs: Vec<(String, InputAttrValue)>,
        /// JSON file containing request-context values.
        #[arg(long = "context-file")]
        context_file: Option<PathBuf>,
        /// Request detailed matching-policy results.
        #[arg(long)]
        detailed: bool,
        /// Display results as a table.
        #[arg(long)]
        table: bool,
    },
    /// View or download policies.
    Policies {
        /// User with optional Namespace::User::name[group1,group2] syntax.
        #[arg(long)]
        user: Option<String>,
        /// Download raw Cedar text.
        #[arg(long)]
        raw: bool,
    },
    /// View or download the Cedar schema.
    Schema {
        /// Download the raw schema document.
        #[arg(long)]
        raw: bool,
    },
    /// Upload policies or a schema from a file.
    Upload {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        raw: bool,
        /// Upload the file as a Cedar schema instead of policies.
        #[arg(long)]
        schema: bool,
        #[arg(long)]
        token: String,
        /// Permit an upload token over non-loopback HTTP.
        #[arg(long, env = "TREETOP_CLI_DANGER_ALLOW_INSECURE_UPLOADS")]
        danger_allow_insecure_uploads: bool,
    },
    /// Toggle JSON output in the REPL.
    Json,
    /// Toggle debug output in the REPL.
    Debug,
    /// Toggle timing output in the REPL.
    Timing,
    /// Show current settings and persistence paths.
    Show,
    /// Show CLI and server versions.
    Version,
    /// Fetch Prometheus metrics.
    Metrics,
}

struct ResolvedServer {
    url: String,
    label: String,
}

fn explicit_source(matches: &clap::ArgMatches, id: &str) -> Option<ValueSource> {
    matches
        .value_source(id)
        .filter(|source| matches!(source, ValueSource::CommandLine | ValueSource::EnvVariable))
}

fn host_port_url(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("http://[{host}]:{port}")
    } else {
        format!("http://{host}:{port}")
    }
}

fn resolve_server(matches: &clap::ArgMatches, cli: &Cli, config: &CliConfig) -> ResolvedServer {
    if explicit_source(matches, "server_url").is_some() {
        let url = cli
            .server_url
            .clone()
            .expect("an explicit server URL has a value");
        return ResolvedServer {
            label: url.clone(),
            url,
        };
    }

    let host_is_explicit = explicit_source(matches, "host").is_some();
    let port_is_explicit = explicit_source(matches, "port").is_some();
    if host_is_explicit || port_is_explicit {
        let host = cli
            .host
            .clone()
            .or_else(|| config.host.clone())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port = cli.port.or(config.port).unwrap_or(9999);
        return ResolvedServer {
            url: host_port_url(&host, port),
            label: format!("{host}:{port}"),
        };
    }

    if let Some(url) = &config.server_url {
        return ResolvedServer {
            label: url.clone(),
            url: url.clone(),
        };
    }

    if config.host.is_some() || config.port.is_some() {
        let host = config
            .host
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port = config.port.unwrap_or(9999);
        return ResolvedServer {
            url: host_port_url(&host, port),
            label: format!("{host}:{port}"),
        };
    }

    ResolvedServer {
        url: DEFAULT_SERVER_URL.to_string(),
        label: "127.0.0.1:9999".to_string(),
    }
}

fn parse_kv(value: &str) -> Result<(String, InputAttrValue), String> {
    let (key, value) = value
        .split_once('=')
        .ok_or_else(|| format!("missing '=' in `{value}`"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err("attribute key is empty".to_string());
    }
    Ok((key.to_string(), value.parse()?))
}

fn sanitize_command(command: &str) -> String {
    command
        .chars()
        .filter_map(|character| {
            if !character.is_ascii() {
                None
            } else if character.is_whitespace() {
                Some('_')
            } else {
                Some(character)
            }
        })
        .collect()
}

fn make_correlation_id(command: &str) -> String {
    let sanitized = sanitize_command(command);
    let uuid = Uuid::new_v4();
    if sanitized.is_empty() {
        uuid.to_string()
    } else {
        format!("{uuid}-{sanitized}")
    }
}

fn json_value_to_attr_value(value: serde_json::Value) -> Result<AttrValue> {
    if let Ok(attribute) = serde_json::from_value::<AttrValue>(value.clone()) {
        return Ok(attribute);
    }

    match value {
        serde_json::Value::String(value) => Ok(AttrValue::String(value)),
        serde_json::Value::Bool(value) => Ok(AttrValue::Bool(value)),
        serde_json::Value::Number(value) => value
            .as_i64()
            .map(AttrValue::Long)
            .ok_or_else(|| anyhow::anyhow!("context numeric values must fit in an i64")),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(json_value_to_attr_value)
            .collect::<Result<Vec<_>>>()
            .map(AttrValue::Set),
        serde_json::Value::Null => anyhow::bail!("context value cannot be null"),
        serde_json::Value::Object(_) => anyhow::bail!(
            "context objects must use typed Cedar format {{\"type\": ..., \"value\": ...}}"
        ),
    }
}

fn load_context_file(path: &Path) -> Result<HashMap<String, AttrValue>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read context file `{}`", path.display()))?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse `{}` as JSON", path.display()))?;
    let object = parsed
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("context file must contain a JSON object"))?;
    object
        .iter()
        .map(|(key, value)| Ok((key.clone(), json_value_to_attr_value(value.clone())?)))
        .collect()
}

pub async fn run() -> Result<()> {
    run_from(std::env::args_os()).await
}

pub async fn run_from<I, T>(arguments: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = Cli::command().get_matches_from(arguments);
    let cli = Cli::from_arg_matches(&matches)?;
    let (config, _) = CliConfig::load();
    let server = resolve_server(&matches, &cli, &config);

    let is_explicit = |id: &str| explicit_source(&matches, id).is_some();
    let show_debug = if is_explicit("debug") {
        cli.debug
    } else {
        config.debug.unwrap_or(cli.debug)
    };
    let show_json = (if is_explicit("json") {
        cli.json
    } else {
        config.json.unwrap_or(cli.json)
    }) || show_debug;
    let show_timing = if is_explicit("timing") {
        cli.timing
    } else {
        config.timing.unwrap_or(cli.timing)
    };
    let table_style = if is_explicit("table_style") {
        cli.table_style
    } else {
        config.table_style.or(cli.table_style)
    }
    .unwrap_or_default();

    let cli_command = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let correlation_id = make_correlation_id(&cli_command);
    let client = Client::builder(&server.url)
        .correlation_id(correlation_id.clone())
        .build()?;
    let mut context = ExecContext {
        client,
        server_url: server.url,
        connection_label: server.label,
        show_json,
        show_debug,
        show_timing,
        last_used: LastUsedValues::default(),
        correlation_id,
        table_style,
    };

    if matches!(cli.command, Commands::Repl) {
        let label = context.connection_label.clone();
        let context = Arc::new(tokio::sync::Mutex::new(context));
        run_repl(
            &label,
            {
                let context = Arc::clone(&context);
                move |input: String| {
                    let context = Arc::clone(&context);
                    async move {
                        let arguments =
                            std::iter::once("treetop-cli").chain(input.split_whitespace());
                        match Cli::try_parse_from(arguments) {
                            Ok(parsed) => {
                                let mut guard = context.lock().await;
                                guard.set_correlation_id(make_correlation_id(&input))?;
                                execute_command(parsed.command, &mut guard).await
                            }
                            Err(parse_error) => {
                                eprintln!("{}: {parse_error}", error("Error"));
                                Ok(())
                            }
                        }
                    }
                }
            },
            print_help,
        )
        .await
    } else {
        execute_command(cli.command, &mut context).await
    }
}

fn print_help() {
    let mut command = Cli::command();
    let _ = command.print_long_help();
    println!();
    println!("{}:", title("REPL-only commands"));
    help_line("json", "Toggle JSON response output");
    help_line("debug", "Toggle typed request/result diagnostics");
    help_line("timing", "Toggle command timing display");
    help_line("history", "Show command history");
    help_line("show", "Show current settings");
    help_line("version", "Show version information");
    help_line("metrics", "Fetch Prometheus metrics");
    help_line("exit, quit", "Exit the REPL");
    help_line("help", "Show this help");
}

struct CheckParams {
    principal: Option<String>,
    action: Option<String>,
    resource_type: Option<String>,
    resource_id: Option<String>,
    attrs: Vec<(String, InputAttrValue)>,
    context_attrs: Vec<(String, InputAttrValue)>,
    context_file: Option<PathBuf>,
    detailed: bool,
    table: bool,
}

fn parse_principal_with_groups(value: &str) -> Result<(String, Vec<String>)> {
    let Some(open) = value.find('[') else {
        return Ok((value.to_string(), Vec::new()));
    };
    let close = value
        .rfind(']')
        .filter(|close| *close > open)
        .ok_or_else(|| anyhow::anyhow!("principal has an unmatched '['"))?;
    if close + 1 != value.len() {
        anyhow::bail!("principal contains characters after its group list");
    }
    let groups = value[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|group| !group.is_empty())
        .map(str::to_string)
        .collect();
    Ok((value[..open].to_string(), groups))
}

fn split_entity(value: &str, marker: &str) -> Result<(Vec<String>, String)> {
    let mut parts = value.split("::").map(str::to_string).collect::<Vec<_>>();
    let id = parts
        .pop()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("entity identifier is empty"))?;
    if parts.last().is_some_and(|part| part == marker) {
        parts.pop();
    }
    Ok((parts, id))
}

fn parse_user(value: &str) -> Result<User> {
    let (entity, groups) = parse_principal_with_groups(value)?;
    let (namespace, id) = split_entity(&entity, "User")?;
    let groups = groups
        .into_iter()
        .map(Group::new)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(User::new(id)?
        .with_namespace(namespace)?
        .with_groups(groups))
}

fn parse_action(value: &str) -> Result<Action> {
    let (namespace, id) = split_entity(value, "Action")?;
    Ok(Action::new(id)?.with_namespace(namespace)?)
}

async fn handle_check(context: &mut ExecContext, params: CheckParams) -> Result<()> {
    let principal = params
        .principal
        .or_else(|| context.last_used.principal.clone())
        .ok_or_else(|| anyhow::anyhow!("--principal is required (no previous value)"))?;
    let action = params
        .action
        .or_else(|| context.last_used.action.clone())
        .ok_or_else(|| anyhow::anyhow!("--action is required (no previous value)"))?;
    let resource_type = params
        .resource_type
        .or_else(|| context.last_used.resource_type.clone())
        .ok_or_else(|| anyhow::anyhow!("--resource-type is required (no previous value)"))?;
    let resource_id = params
        .resource_id
        .or_else(|| context.last_used.resource_id.clone())
        .ok_or_else(|| anyhow::anyhow!("--resource-id is required (no previous value)"))?;
    let resolved_attrs = if params.attrs.is_empty() {
        context.last_used.attrs.clone()
    } else {
        params.attrs
    };

    let mut request_context = match params.context_file {
        Some(path) => load_context_file(&path)?,
        None => HashMap::new(),
    };
    for (key, value) in params.context_attrs {
        request_context.insert(key, value.to_client_value()?);
    }

    context.last_used.principal = Some(principal.clone());
    context.last_used.action = Some(action.clone());
    context.last_used.resource_type = Some(resource_type.clone());
    context.last_used.resource_id = Some(resource_id.clone());
    context.last_used.attrs = resolved_attrs.clone();

    let attrs = resolved_attrs
        .iter()
        .map(|(key, value)| (key.clone(), value.to_string()))
        .collect::<Vec<_>>();
    let attr_permutations_count = attrs
        .iter()
        .map(|(_, value)| value.split('|').count())
        .product::<usize>();
    let matrix_queries = expand_matrix(&principal, &action, &resource_type, &resource_id, attrs);
    if matrix_queries.len() > 1 {
        let mut dimensions = Vec::new();
        for (count, label) in [
            (principal.split('|').count(), "principals"),
            (action.split('|').count(), "actions"),
            (resource_type.split('|').count(), "resource-types"),
            (resource_id.split('|').count(), "resource-ids"),
            (attr_permutations_count, "attributes"),
        ] {
            if count > 1 {
                dimensions.push(format!("{count} {label}"));
            }
        }
        println!(
            "{} Generating {} permutations: {}",
            title("Matrix:"),
            matrix_queries.len(),
            dimensions.join(" × ")
        );
    }

    let mut auth_requests = Vec::with_capacity(matrix_queries.len());
    for query in &matrix_queries {
        let mut resource = Resource::new(&query.resource_type, &query.resource_id)?;
        for (key, value) in &query.attrs {
            let value = InputAttrValue::from_str(value).map_err(anyhow::Error::msg)?;
            resource = resource.with_attr(key, value.to_client_value()?)?;
        }
        let request = Request::new(
            Principal::User(parse_user(&query.principal)?),
            parse_action(&query.action)?,
            resource,
        );
        let mut auth_request = AuthRequest::new(request).with_id(&query.query_id)?;
        if !request_context.is_empty() {
            auth_request = auth_request.with_context(request_context.clone())?;
        }
        auth_requests.push(auth_request);
    }
    let request = AuthorizeRequest::from_auth_requests(auth_requests)?;
    debug_typed(context, "request", &request);

    let use_table = params.table || matrix_queries.len() > 1;
    if params.detailed {
        let response = context.client.authorize_detailed(&request).await;
        let response = checked_result(context, response)?;
        output_json(context, &response)?;
        if !context.show_json {
            println!(
                "{}",
                display_authorization(&response, use_table, context.table_style)
            );
        }
    } else {
        let response = context.client.authorize(&request).await;
        let response = checked_result(context, response)?;
        output_json(context, &response)?;
        if !context.show_json {
            println!(
                "{}",
                display_authorization(&response, use_table, context.table_style)
            );
        }
    }
    Ok(())
}

fn parse_user_filter(value: &str) -> Result<(String, Vec<String>, Vec<String>)> {
    let (entity, groups) = parse_principal_with_groups(value)?;
    let (namespace, id) = split_entity(&entity, "User")?;
    for group in &groups {
        Group::new(group)?;
    }
    User::new(&id)?.with_namespace(namespace.clone())?;
    Ok((id, groups, namespace))
}

async fn handle_policies(context: &ExecContext, user: Option<String>, raw: bool) -> Result<()> {
    match user {
        None if raw || !context.show_json => {
            let response = checked_result(context, context.client.get_policies_raw().await)?;
            println!("{response}");
        }
        None => {
            let response: PoliciesDownload =
                checked_result(context, context.client.get_policies().await)?;
            output_typed(context, &response)?;
        }
        Some(user) => {
            let (user, groups, namespaces) = parse_user_filter(&user)?;
            if raw {
                let response = checked_result(
                    context,
                    context
                        .client
                        .get_user_policies_raw(&user, &groups, &namespaces)
                        .await,
                )?;
                println!("{response}");
            } else {
                let response: UserPolicies = checked_result(
                    context,
                    context
                        .client
                        .get_user_policies(&user, &groups, &namespaces)
                        .await,
                )?;
                output_typed(context, &response)?;
            }
        }
    }
    Ok(())
}

async fn handle_schema(context: &ExecContext, raw: bool) -> Result<()> {
    if raw || !context.show_json {
        let response = checked_result(context, context.client.get_schema_raw().await)?;
        println!("{response}");
    } else {
        let response: SchemaDownload = checked_result(context, context.client.get_schema().await)?;
        output_typed(context, &response)?;
    }
    Ok(())
}

async fn handle_upload(
    context: &ExecContext,
    file: PathBuf,
    raw: bool,
    schema: bool,
    token: String,
    danger_allow_insecure_uploads: bool,
) -> Result<()> {
    let content = fs::read_to_string(&file)
        .with_context(|| format!("failed to read upload file `{}`", file.display()))?;
    let upload_client = Client::builder(&context.server_url)
        .correlation_id(context.correlation_id.clone())
        .upload_token(UploadToken::new(token)?)
        .danger_allow_insecure_uploads(danger_allow_insecure_uploads)
        .build()?;
    let response: PoliciesMetadata = if schema {
        if raw {
            checked_result(context, upload_client.upload_schema_raw(&content).await)?
        } else {
            checked_result(context, upload_client.upload_schema_json(&content).await)?
        }
    } else if raw {
        checked_result(context, upload_client.upload_policies_raw(&content).await)?
    } else {
        checked_result(context, upload_client.upload_policies_json(&content).await)?
    };
    output_typed(context, &response)
}

fn debug_typed<T: Serialize>(context: &ExecContext, label: &str, value: &T) {
    if context.show_debug {
        match serde_json::to_string_pretty(value) {
            Ok(value) => eprintln!("{} {label}:\n{value}", warning("DEBUG")),
            Err(error) => eprintln!("{} serialization error: {error}", warning("DEBUG")),
        }
    }
}

fn checked_result<T>(context: &ExecContext, result: treetop_client::Result<T>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(client_error) => {
            if context.show_debug {
                eprintln!("{} error: {client_error:?}", warning("DEBUG"));
            }
            Err(client_error.into())
        }
    }
}

fn output_json<T: Serialize>(context: &ExecContext, response: &T) -> Result<()> {
    debug_typed(context, "validated response", response);
    if context.show_json {
        println!("{}", serde_json::to_string_pretty(response)?);
    }
    Ok(())
}

fn output_typed<T>(context: &ExecContext, response: &T) -> Result<()>
where
    T: Serialize + CliDisplay,
{
    output_json(context, response)?;
    if !context.show_json {
        println!("{}", response.display());
    }
    Ok(())
}

fn show_settings(context: &ExecContext) {
    println!("\n{}", title("Current Settings:"));
    settings_line("Server:", &context.connection_label);
    settings_line("Server URL:", &context.server_url);
    settings_line("JSON output:", status_flag(context.show_json));
    settings_line("Debug mode:", status_flag(context.show_debug));
    settings_line("Timing:", status_flag(context.show_timing));
    settings_line("Table style:", &context.table_style.to_string());

    if context.last_used.principal.is_some() || context.last_used.action.is_some() {
        println!("\n{}", title("Last Used Values:"));
        if let Some(value) = &context.last_used.principal {
            settings_line("Principal:", value);
        }
        if let Some(value) = &context.last_used.action {
            settings_line("Action:", value);
        }
        if let Some(value) = &context.last_used.resource_type {
            settings_line("Resource Type:", value);
        }
        if let Some(value) = &context.last_used.resource_id {
            settings_line("Resource ID:", value);
        }
    }

    println!("\n{}", title("Files:"));
    if let Some(path) = cli_history_path() {
        settings_line("History:", &path.display().to_string());
    }
    if let Some(path) = cli_config_path() {
        settings_line("Config:", &path.display().to_string());
    }
}

fn toggle_json(context: &mut ExecContext) {
    context.show_json = !context.show_json;
    println!("JSON responses: {}", status_flag(context.show_json));
}

fn toggle_debug(context: &mut ExecContext) {
    context.show_debug = !context.show_debug;
    if context.show_debug {
        context.show_json = true;
    }
    println!("Debug mode: {}", status_flag(context.show_debug));
}

fn toggle_timing(context: &mut ExecContext) {
    context.show_timing = !context.show_timing;
    println!("Timing display: {}", status_flag(context.show_timing));
}

async fn execute_command(command: Commands, context: &mut ExecContext) -> Result<()> {
    let started = std::time::Instant::now();
    match command {
        Commands::Repl => unreachable!("REPL is handled by run_from"),
        Commands::Status | Commands::Version => show_status_and_version(context).await?,
        Commands::Check {
            principal,
            action,
            resource_type,
            resource_id,
            attrs,
            context_attrs,
            context_file,
            detailed,
            table,
        } => {
            handle_check(
                context,
                CheckParams {
                    principal,
                    action,
                    resource_type,
                    resource_id,
                    attrs,
                    context_attrs,
                    context_file,
                    detailed,
                    table,
                },
            )
            .await?;
        }
        Commands::Policies { user, raw } => handle_policies(context, user, raw).await?,
        Commands::Schema { raw } => handle_schema(context, raw).await?,
        Commands::Upload {
            file,
            raw,
            schema,
            token,
            danger_allow_insecure_uploads,
        } => {
            handle_upload(
                context,
                file,
                raw,
                schema,
                token,
                danger_allow_insecure_uploads,
            )
            .await?;
        }
        Commands::Json => toggle_json(context),
        Commands::Debug => toggle_debug(context),
        Commands::Timing => toggle_timing(context),
        Commands::Show => show_settings(context),
        Commands::Metrics => {
            let metrics = checked_result(context, context.client.metrics().await)?;
            if context.show_debug {
                eprintln!(
                    "{} validated metrics response: {} bytes",
                    warning("DEBUG"),
                    metrics.len()
                );
            }
            println!("{metrics}");
        }
    }
    if context.show_timing {
        println!(
            "Time: {} milliseconds",
            started.elapsed().as_micros() as f64 / 1000.0
        );
    }
    Ok(())
}

#[derive(Default, Deserialize)]
struct ParallelDisplay {
    cpu_count: Option<usize>,
    workers: Option<usize>,
    allow_parallel: Option<bool>,
    rayon_threads: Option<usize>,
    par_threshold: Option<usize>,
}

fn optional_number(value: Option<usize>) -> String {
    value.map_or_else(|| "unavailable".to_string(), |value| value.to_string())
}

async fn show_status_and_version(context: &ExecContext) -> Result<()> {
    let status: StatusResponse = checked_result(context, context.client.status().await)?;
    let server_version = checked_result(context, context.client.version().await).ok();
    debug_typed(context, "validated status", &status);
    if let Some(server_version) = &server_version {
        debug_typed(context, "validated version", server_version);
    }
    if context.show_json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    }

    println!("\n{}", title("treetop-cli"));
    settings_line("Version:", &version(env!("TREETOP_CLI_VERSION")));
    settings_line("Built:", env!("TREETOP_CLI_BUILD_TIMESTAMP"));
    settings_line("Git:", env!("TREETOP_CLI_GIT_DESCRIBE"));
    settings_line("Target:", env!("TREETOP_CLI_BUILD_TARGET"));

    println!("\n{}", title("Server"));
    if let Some(info) = server_version {
        settings_line("Version:", &version(&info.version));
        settings_line("Core:", &info.core.version);
        settings_line("Cedar:", &info.core.cedar);
    } else {
        settings_line("Version:", &warning("unavailable"));
    }

    let policies = &status.policy_configuration.policies;
    println!("\n{}", title("Policies"));
    print_metadata(policies);
    settings_line(
        "Allow upload:",
        yes_no(status.policy_configuration.allow_upload),
    );
    settings_line(
        "Schema mode:",
        &status.policy_configuration.schema_validation_mode,
    );

    println!("\n{}", title("Labels"));
    print_metadata(&status.policy_configuration.labels);

    println!("\n{}", title("Schema"));
    if let Some(schema) = &status.policy_configuration.schema {
        print_metadata(schema);
    } else {
        settings_line("Status:", "not loaded");
    }

    let parallel: ParallelDisplay =
        serde_json::from_value(status.parallel_configuration.clone()).unwrap_or_default();
    println!("\n{}", title("Parallelism"));
    settings_line("CPU count:", &optional_number(parallel.cpu_count));
    settings_line("Worker threads:", &optional_number(parallel.workers));
    settings_line(
        "Parallelizing:",
        parallel.allow_parallel.map(yes_no).unwrap_or("unavailable"),
    );
    settings_line("Threads:", &optional_number(parallel.rayon_threads));
    settings_line("Cutoff:", &optional_number(parallel.par_threshold));

    let limits = &status.request_limits;
    println!("\n{}", title("Request Limits"));
    settings_line(
        "Batch size:",
        &limits.max_batch_size.map_or_else(
            || "Unlimited (legacy server)".to_string(),
            |value| value.to_string(),
        ),
    );
    settings_line("Context bytes:", &limits.max_context_bytes.to_string());
    settings_line("Context depth:", &limits.max_context_depth.to_string());
    settings_line("Context keys:", &limits.max_context_keys.to_string());

    println!("\n{}", title("Request Context"));
    settings_line("Supported:", yes_no(status.request_context.supported));
    settings_line(
        "Schema-backed:",
        yes_no(status.request_context.schema_backed),
    );
    if let Some(reason) = status.request_context.fallback_reason {
        settings_line("Fallback reason:", &format!("{reason:?}"));
    }
    Ok(())
}

fn print_metadata(metadata: &treetop_client::Metadata) {
    settings_line("Hash:", &metadata.sha256);
    settings_line("Updated:", &metadata.timestamp);
    settings_line("Entries:", &metadata.entries.to_string());
    settings_line("Size:", &format!("{} bytes", metadata.size).white());
    if let Some(source) = &metadata.source {
        settings_line("Source:", source.as_str());
    }
    if let Some(frequency) = metadata.refresh_frequency {
        settings_line("Refresh:", &format!("every {frequency}s"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_url_precedence_prefers_cli_url() {
        let matches = Cli::command().get_matches_from([
            "treetop-cli",
            "--server-url",
            "https://cli.example",
            "--host",
            "host.example",
            "status",
        ]);
        let cli = Cli::from_arg_matches(&matches).unwrap();
        let config = CliConfig {
            server_url: Some("https://config.example".to_string()),
            ..CliConfig::default()
        };
        assert_eq!(
            resolve_server(&matches, &cli, &config).url,
            "https://cli.example"
        );
    }

    #[test]
    fn explicit_host_skips_config_url() {
        let matches =
            Cli::command().get_matches_from(["treetop-cli", "--host", "host.example", "status"]);
        let cli = Cli::from_arg_matches(&matches).unwrap();
        let config = CliConfig {
            server_url: Some("https://config.example".to_string()),
            port: Some(8443),
            ..CliConfig::default()
        };
        assert_eq!(
            resolve_server(&matches, &cli, &config).url,
            "http://host.example:8443"
        );
    }

    #[test]
    fn parses_namespaced_user_with_groups() {
        let user = parse_user("DNS::User::alice[admins,viewers]").unwrap();
        assert_eq!(user.id(), "alice");
        assert_eq!(user.namespace(), &["DNS"]);
        assert_eq!(user.groups().len(), 2);
    }
}
