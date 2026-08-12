use std::{fmt, net::IpAddr, str::FromStr};

use colored::Colorize;
use serde::{Deserialize, Serialize};
use tabled::{Table, Tabled, settings::Style};
use treetop_client::{
    AttrValue, AuthorizeDecisionBrief, AuthorizeDecisionDetailed, AuthorizeResponse, BatchResult,
    DecisionBrief, Metadata, PoliciesDownload, PoliciesMetadata, PolicyMatchReason, SchemaDownload,
    UserPolicies, ValidationError,
};

use crate::style::{error, success, warning};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum TableStyle {
    Ascii,
    #[default]
    Rounded,
    Unicode,
    Markdown,
}

impl FromStr for TableStyle {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "ascii" => Ok(Self::Ascii),
            "rounded" => Ok(Self::Rounded),
            "unicode" => Ok(Self::Unicode),
            "markdown" => Ok(Self::Markdown),
            _ => Err(format!("unknown table style: {value}")),
        }
    }
}

impl fmt::Display for TableStyle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Ascii => "ascii",
            Self::Rounded => "rounded",
            Self::Unicode => "unicode",
            Self::Markdown => "markdown",
        })
    }
}

impl TableStyle {
    fn apply_to_table(self, mut table: Table) -> Table {
        match self {
            Self::Ascii => table.with(Style::ascii()),
            Self::Rounded => table.with(Style::rounded()),
            Self::Unicode => table.with(Style::modern()),
            Self::Markdown => table.with(Style::markdown()),
        };
        table
    }
}

#[derive(Default, Clone)]
pub struct LastUsedValues {
    pub principal: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub attrs: Vec<(String, InputAttrValue)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputAttrValue {
    Ip(IpAddr),
    Long(i64),
    Bool(bool),
    String(String),
}

impl InputAttrValue {
    pub fn to_client_value(&self) -> Result<AttrValue, ValidationError> {
        match self {
            Self::Ip(ip) => AttrValue::ip(ip.to_string()),
            Self::Long(value) => Ok(AttrValue::Long(*value)),
            Self::Bool(value) => Ok(AttrValue::Bool(*value)),
            Self::String(value) => Ok(AttrValue::String(value.clone())),
        }
    }
}

impl fmt::Display for InputAttrValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ip(value) => write!(formatter, "{value}"),
            Self::Long(value) => write!(formatter, "{value}"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::String(value) => formatter.write_str(value),
        }
    }
}

impl FromStr for InputAttrValue {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if let Ok(ip) = value.parse::<IpAddr>() {
            return Ok(Self::Ip(ip));
        }
        if let Ok(integer) = value.parse::<i64>() {
            return Ok(Self::Long(integer));
        }
        if let Ok(boolean) = value.parse::<bool>() {
            return Ok(Self::Bool(boolean));
        }
        let unquoted = value
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
            .unwrap_or(value);
        Ok(Self::String(unquoted.to_string()))
    }
}

pub trait CliDisplay {
    fn display(&self) -> String;
}

fn display_metadata(metadata: &Metadata) -> String {
    let mut output = format!(
        "Hash: {}\nUpdated: {}\nEntries: {}\nSize: {} bytes",
        metadata.sha256, metadata.timestamp, metadata.entries, metadata.size
    );
    if let Some(source) = &metadata.source {
        output.push_str(&format!("\nSource: {source}"));
    }
    if let Some(frequency) = metadata.refresh_frequency {
        output.push_str(&format!("\nRefresh: every {frequency}s"));
    }
    output
}

impl CliDisplay for PoliciesMetadata {
    fn display(&self) -> String {
        let schema = self
            .schema
            .as_ref()
            .map(display_metadata)
            .unwrap_or_else(|| "Not loaded".to_string());
        format!(
            "Allow upload: {}\nSchema validation mode: {}\nPolicies:\n{}\nHost labels:\n{}\nSchema:\n{}",
            self.allow_upload,
            self.schema_validation_mode,
            display_metadata(&self.policies),
            display_metadata(&self.labels),
            schema,
        )
    }
}

impl CliDisplay for PoliciesDownload {
    fn display(&self) -> String {
        format!(
            "Metadata:\n{}\nContent:\n{}",
            display_metadata(&self.policies),
            self.policies.content
        )
    }
}

impl CliDisplay for SchemaDownload {
    fn display(&self) -> String {
        format!(
            "Metadata:\n{}\nContent:\n{}",
            display_metadata(&self.schema),
            self.schema.content
        )
    }
}

fn format_match_reason(reason: &PolicyMatchReason) -> &'static str {
    match reason {
        PolicyMatchReason::PrincipalEq => "PrincipalEq",
        PolicyMatchReason::PrincipalIn => "PrincipalIn",
        PolicyMatchReason::PrincipalAny => "PrincipalAny",
        PolicyMatchReason::PrincipalIs => "PrincipalIs",
        PolicyMatchReason::PrincipalIsIn => "PrincipalIsIn",
        PolicyMatchReason::ActionEq => "ActionEq",
        PolicyMatchReason::ActionIn => "ActionIn",
        PolicyMatchReason::ActionAny => "ActionAny",
        PolicyMatchReason::ResourceEq => "ResourceEq",
        PolicyMatchReason::ResourceIn => "ResourceIn",
        PolicyMatchReason::ResourceAny => "ResourceAny",
        PolicyMatchReason::ResourceIs => "ResourceIs",
        PolicyMatchReason::ResourceIsIn => "ResourceIsIn",
        PolicyMatchReason::Unknown => "Unknown",
        _ => "Unknown",
    }
}

impl CliDisplay for UserPolicies {
    fn display(&self) -> String {
        let mut output = format!("User: {}\nPolicies: {}\n", self.user, self.policies.len());
        if self.matches.is_empty() {
            output.push_str("Match reasons: none\n");
            return output;
        }

        output.push_str("Match reasons:\n");
        for policy_match in &self.matches {
            let reasons = if policy_match.reasons.is_empty() {
                "none".to_string()
            } else {
                policy_match
                    .reasons
                    .iter()
                    .map(format_match_reason)
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            output.push_str(&format!("  - {}: {}\n", policy_match.cedar_id, reasons));
        }
        output
    }
}

pub(crate) trait DecisionView {
    fn decision(&self) -> DecisionBrief;
    fn policy_label(&self) -> String;
    fn display(&self) -> String;
}

impl DecisionView for AuthorizeDecisionBrief {
    fn decision(&self) -> DecisionBrief {
        self.decision
    }

    fn policy_label(&self) -> String {
        self.policy_id.clone()
    }

    fn display(&self) -> String {
        match self.decision {
            DecisionBrief::Allow => format!(
                "{} ({} @ {})",
                success("Allow"),
                self.policy_id,
                self.version.hash
            ),
            DecisionBrief::Deny => format!("{} ({})", error("Deny"), self.version.hash),
        }
    }
}

impl DecisionView for AuthorizeDecisionDetailed {
    fn decision(&self) -> DecisionBrief {
        self.decision
    }

    fn policy_label(&self) -> String {
        self.policy
            .first()
            .map(|policy| {
                policy
                    .annotation_id
                    .clone()
                    .unwrap_or_else(|| policy.cedar_id.clone())
            })
            .unwrap_or_default()
    }

    fn display(&self) -> String {
        match self.decision {
            DecisionBrief::Allow => {
                let policies = self
                    .policy
                    .iter()
                    .map(|policy| policy.annotation_id.as_deref().unwrap_or(&policy.cedar_id))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "{} ({})\n{}\n{}\n{}",
                    success("Allow"),
                    self.version.hash,
                    "--- Matching policies ---".cyan(),
                    policies,
                    "---".cyan()
                )
            }
            DecisionBrief::Deny => format!("{} ({})", error("Deny"), self.version.hash),
        }
    }
}

#[derive(Tabled)]
struct ResultRow {
    #[tabled(rename = "#")]
    index: String,
    #[tabled(rename = "QID")]
    id: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Decision")]
    decision: String,
    #[tabled(rename = "PolicyID")]
    policy: String,
}

pub(crate) fn display_authorization<T>(
    response: &AuthorizeResponse<T>,
    use_table: bool,
    style: TableStyle,
) -> String
where
    T: DecisionView,
{
    if response.results.is_empty() {
        return warning("Warning: No results in response").to_string();
    }

    if !use_table && response.results.len() == 1 {
        return match &response.results[0].result {
            BatchResult::Success { data } => data.display(),
            BatchResult::Failed { message } => format!("{}: {message}", error("Failed")),
        };
    }

    let rows = response
        .results
        .iter()
        .map(|result| match &result.result {
            BatchResult::Success { data } => ResultRow {
                index: result.index.to_string(),
                id: result.id.clone().unwrap_or_default(),
                status: "success".to_string(),
                decision: data.decision().to_string(),
                policy: data.policy_label(),
            },
            BatchResult::Failed { message } => ResultRow {
                index: result.index.to_string(),
                id: result.id.clone().unwrap_or_default(),
                status: "failed".to_string(),
                decision: message.clone(),
                policy: String::new(),
            },
        })
        .collect::<Vec<_>>();
    let table = style.apply_to_table(Table::new(rows)).to_string();
    format!("Version: {}\n{table}", response.version.hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use treetop_client::{IndexedResult, PolicyMatch, PolicyVersion};

    fn version() -> PolicyVersion {
        PolicyVersion {
            hash: "policy-hash".to_string(),
            loaded_at: "2026-08-12T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn brief_authorization_rendering_uses_validated_response() {
        let response = AuthorizeResponse {
            results: vec![IndexedResult {
                index: 0,
                id: Some("query-0".to_string()),
                result: BatchResult::Success {
                    data: AuthorizeDecisionBrief {
                        decision: DecisionBrief::Allow,
                        version: version(),
                        policy_id: "allow-view".to_string(),
                    },
                },
            }],
            version: version(),
            successful: 1,
            failed: 0,
        };

        let rendered = display_authorization(&response, false, TableStyle::Ascii);
        assert!(rendered.contains("Allow"));
        assert!(rendered.contains("allow-view"));
        assert!(rendered.contains("policy-hash"));
    }

    #[test]
    fn user_policy_rendering_includes_match_reasons() {
        let response = UserPolicies {
            user: "alice".to_string(),
            policies: vec![serde_json::json!({"effect": "permit"})],
            matches: vec![PolicyMatch {
                cedar_id: "policy0".to_string(),
                reasons: vec![
                    PolicyMatchReason::PrincipalEq,
                    PolicyMatchReason::ResourceIs,
                ],
            }],
        };

        let rendered = response.display();
        assert!(rendered.contains("User: alice"));
        assert!(rendered.contains("policy0"));
        assert!(rendered.contains("PrincipalEq"));
        assert!(rendered.contains("ResourceIs"));
    }
}
