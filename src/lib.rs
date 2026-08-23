use std::sync::{Arc, Mutex};

use guardgen_lib::{
    IncludeGuardGenerator, Language as GuardLanguage, LineEnding as GuardLineEnding, UuidKind,
};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    schemars,
    schemars::JsonSchema,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};

fn default_prefix() -> String {
    "UUID".to_owned()
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    None,
    C,
    Cxx,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    #[default]
    None,
    Lf,
    Crlf,
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum UuidVersion {
    #[default]
    V7,
    V4,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateIncludeGuardArguments {
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    #[schemars(default)]
    pub language: Language,
    #[serde(default)]
    #[schemars(default)]
    pub line_ending: LineEnding,
    #[serde(default)]
    #[schemars(default)]
    pub uuid_version: UuidVersion,
}

#[derive(Clone)]
pub struct GuardGenServer {
    generator: Arc<Mutex<IncludeGuardGenerator>>,
}

impl Default for GuardGenServer {
    fn default() -> Self {
        Self {
            generator: Arc::new(Mutex::new(IncludeGuardGenerator::new())),
        }
    }
}

fn is_c_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_suffix_segment(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn invalid_params(message: impl Into<String>) -> McpError {
    McpError::invalid_params(message.into(), None)
}

#[tool_router]
impl GuardGenServer {
    #[tool(
        description = "Generate a UUID-based C/C++ include guard source.",
        input_schema = rmcp::handler::server::common::schema_for_input::<GenerateIncludeGuardArguments>().unwrap()
    )]
    async fn generate_include_guard(
        &self,
        Parameters(raw_arguments): Parameters<serde_json::Value>,
    ) -> Result<CallToolResult, McpError> {
        let arguments: GenerateIncludeGuardArguments = serde_json::from_value(raw_arguments)
            .map_err(|error| invalid_params(format!("invalid arguments: {error}")))?;
        if !is_c_identifier(&arguments.prefix) {
            return Err(invalid_params(
                "prefix must be a non-empty ASCII C identifier",
            ));
        }
        if let Some(suffix) = &arguments.suffix
            && !is_suffix_segment(suffix)
        {
            return Err(invalid_params(
                "suffix must be a non-empty ASCII alphanumeric/underscore segment",
            ));
        }

        let language = match arguments.language {
            Language::None => GuardLanguage::None,
            Language::C => GuardLanguage::C,
            Language::Cxx => GuardLanguage::Cxx,
        };
        let line_ending = match arguments.line_ending {
            LineEnding::None => GuardLineEnding::None,
            LineEnding::Lf => GuardLineEnding::LF,
            LineEnding::Crlf => GuardLineEnding::CRLF,
        };
        let uuid_kind = match arguments.uuid_version {
            UuidVersion::V7 => UuidKind::V7,
            UuidVersion::V4 => UuidKind::V4,
        };

        let text = self
            .generator
            .lock()
            .map_err(|_| McpError::internal_error("generator lock poisoned", None))?
            .generate(
                arguments.prefix,
                arguments.suffix,
                language,
                line_ending,
                uuid_kind,
            );
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
}

#[tool_handler(
    name = "guardgen_mcp",
    version = "0.1.0",
    instructions = "Generate UUID-based C/C++ include guards with GuardGen."
)]
impl ServerHandler for GuardGenServer {}

pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = GuardGenServer::default().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
