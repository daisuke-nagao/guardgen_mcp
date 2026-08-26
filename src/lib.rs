// SPDX-FileCopyrightText: 2026 Daisuke Nagao
// SPDX-License-Identifier: MIT OR Apache-2.0

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

/// Header linkage wrapper emitted inside the include guard.
///
/// `c` adds an `extern "C"` linkage section around the header body.
/// `none` (default) and `cxx` emit only the include-guard scaffold.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Recommended default: no language-linkage wrapper.
    #[default]
    None,
    C,
    Cxx,
}

/// Line ending used in the generated source.
///
/// Default (and recommended when unspecified): `None` — uses the
/// platform-native line ending instead of forcing LF or CRLF.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    /// Recommended default: platform-native line ending.
    #[default]
    None,
    Lf,
    Crlf,
}

/// UUID version used to make the guard macro name unique.
///
/// Default (and recommended when unspecified): `V7` — time-ordered UUIDs,
/// preferred over `V4` for readability and sortability across guards.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum UuidVersion {
    /// Recommended default: time-ordered, sortable UUIDs.
    #[default]
    V7,
    V4,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateIncludeGuardArguments {
    /// Macro name prefix.
    ///
    /// Defaults to `"UUID"`. GuardGen appends a UUID segment to every
    /// generated macro, so a custom prefix is only needed for a project naming
    /// convention.
    #[serde(default = "default_prefix")]
    pub prefix: String,
    /// Optional macro name suffix, appended after the UUID segment.
    /// Default (recommended when unspecified): none (no suffix).
    #[serde(default)]
    pub suffix: Option<String>,
    /// Default (recommended when unspecified): `none` — see [`Language`].
    #[serde(default)]
    #[schemars(default)]
    pub language: Language,
    /// Default (recommended when unspecified): `none` — see [`LineEnding`].
    #[serde(default)]
    #[schemars(default)]
    pub line_ending: LineEnding,
    /// Default (recommended when unspecified): `v7` — see [`UuidVersion`].
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
        description = "Generate a UUID-based C/C++ include guard source. \
            All arguments are optional; if unspecified, defaults to \
            prefix=\"UUID\", no suffix, language=none, line_ending=none \
            (platform-native), and uuid_version=v7 (time-ordered UUID).",
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
    version = "1.0.0",
    instructions = "Use this server when writing or editing a C/C++ header \
        and you need a UUID-based #ifndef/#define/#endif include guard. \
        Call generate_include_guard; it returns a complete include-guard \
        scaffold. Insert the header body between the opening and closing \
        sections, before the final include-guard #endif. With language=c, \
        insert the body inside the extern \"C\" linkage section."
)]
impl ServerHandler for GuardGenServer {}

pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = GuardGenServer::default().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
