use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::{
    ClientCapabilities, ClientNotification, ClientRequest, CustomNotification, CustomRequest,
    Extensions, Implementation, JsonObject, JsonRpcMessage, LoggingLevel, NumberOrString,
    ProgressToken, ProtocolVersion, ServerNotification, ServerRequest,
};

pub trait GetMeta {
    fn get_meta_mut(&mut self) -> &mut Meta;
    fn get_meta(&self) -> &Meta;
}

pub trait GetExtensions {
    fn extensions(&self) -> &Extensions;
    fn extensions_mut(&mut self) -> &mut Extensions;
}

/// Trait for request params that contain the `_meta` field.
///
/// Per the MCP 2025-11-25 spec, all request params should have an optional `_meta`
/// field that can contain a `progressToken` for tracking long-running operations.
pub trait RequestParamsMeta {
    /// Get a reference to the meta field
    fn meta(&self) -> Option<&Meta>;
    /// Get a mutable reference to the meta field
    fn meta_mut(&mut self) -> &mut Option<Meta>;
    /// Set the meta field
    fn set_meta(&mut self, meta: Meta) {
        *self.meta_mut() = Some(meta);
    }
    /// Get the progress token from meta, if present
    fn progress_token(&self) -> Option<ProgressToken> {
        self.meta().and_then(|m| m.get_progress_token())
    }
    /// Set a progress token in meta
    fn set_progress_token(&mut self, token: ProgressToken) {
        match self.meta_mut() {
            Some(meta) => meta.set_progress_token(token),
            none => {
                let mut meta = Meta::new();
                meta.set_progress_token(token);
                *none = Some(meta);
            }
        }
    }
}

/// Trait for task-augmented request params that contain both `_meta` and `task` fields.
///
/// Per the MCP 2025-11-25 spec, certain requests (like `tools/call` and `sampling/createMessage`)
/// can include a `task` field to signal that the caller wants task-augmented execution.
pub trait TaskAugmentedRequestParamsMeta: RequestParamsMeta {
    /// Get a reference to the task field
    fn task(&self) -> Option<&JsonObject>;
    /// Get a mutable reference to the task field
    fn task_mut(&mut self) -> &mut Option<JsonObject>;
    /// Set the task field
    fn set_task(&mut self, task: JsonObject) {
        *self.task_mut() = Some(task);
    }
}

impl GetExtensions for CustomNotification {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

impl GetMeta for CustomNotification {
    fn get_meta_mut(&mut self) -> &mut Meta {
        self.extensions_mut().get_or_insert_default()
    }
    fn get_meta(&self) -> &Meta {
        self.extensions()
            .get::<Meta>()
            .unwrap_or(Meta::static_empty())
    }
}

impl GetExtensions for CustomRequest {
    fn extensions(&self) -> &Extensions {
        &self.extensions
    }
    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

impl GetMeta for CustomRequest {
    fn get_meta_mut(&mut self) -> &mut Meta {
        self.extensions_mut().get_or_insert_default()
    }
    fn get_meta(&self) -> &Meta {
        self.extensions()
            .get::<Meta>()
            .unwrap_or(Meta::static_empty())
    }
}

macro_rules! variant_extension {
    (
        $Enum: ident {
            $($variant: ident)*
        }
    ) => {
        impl GetExtensions for $Enum {
            fn extensions(&self) -> &Extensions {
                match self {
                    $(
                        $Enum::$variant(v) => &v.extensions,
                    )*
                }
            }
            fn extensions_mut(&mut self) -> &mut Extensions {
                match self {
                    $(
                        $Enum::$variant(v) => &mut v.extensions,
                    )*
                }
            }
        }
        impl GetMeta for $Enum {
            fn get_meta_mut(&mut self) -> &mut Meta {
                self.extensions_mut().get_or_insert_default()
            }
            fn get_meta(&self) -> &Meta {
                self.extensions().get::<Meta>().unwrap_or(Meta::static_empty())
            }
        }
    };
}

variant_extension! {
    ClientRequest {
        PingRequest
        InitializeRequest
        CompleteRequest
        SetLevelRequest
        GetPromptRequest
        ListPromptsRequest
        ListResourcesRequest
        ListResourceTemplatesRequest
        ReadResourceRequest
        SubscribeRequest
        UnsubscribeRequest
        CallToolRequest
        ListToolsRequest
        DiscoverRequest
        CustomRequest
        GetTaskInfoRequest
        UpdateTaskRequest
        ListTasksRequest
        GetTaskResultRequest
        CancelTaskRequest
    }
}

variant_extension! {
    ServerRequest {
        PingRequest
        CreateMessageRequest
        ListRootsRequest
        CreateElicitationRequest
        CustomRequest
    }
}

variant_extension! {
    ClientNotification {
        CancelledNotification
        ProgressNotification
        InitializedNotification
        RootsListChangedNotification
        CustomNotification
    }
}

variant_extension! {
    ServerNotification {
        CancelledNotification
        ProgressNotification
        LoggingMessageNotification
        ResourceUpdatedNotification
        ResourceListChangedNotification
        ToolListChangedNotification
        PromptListChangedNotification
        ElicitationCompletionNotification
        CustomNotification
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(transparent)]
#[expect(clippy::exhaustive_structs, reason = "intentionally exhaustive")]
pub struct Meta(pub JsonObject);
const PROGRESS_TOKEN_FIELD: &str = "progressToken";
pub const RC_PROTOCOL_VERSION_FIELD: &str = "io.modelcontextprotocol/protocolVersion";
pub const RC_CLIENT_INFO_FIELD: &str = "io.modelcontextprotocol/clientInfo";
pub const RC_CLIENT_CAPABILITIES_FIELD: &str = "io.modelcontextprotocol/clientCapabilities";
pub const RC_LOG_LEVEL_FIELD: &str = "io.modelcontextprotocol/logLevel";
pub const TRACEPARENT_FIELD: &str = "traceparent";
pub const TRACESTATE_FIELD: &str = "tracestate";
pub const BAGGAGE_FIELD: &str = "baggage";

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RcRequestMeta {
    pub protocol_version: ProtocolVersion,
    pub client_info: Implementation,
    pub client_capabilities: ClientCapabilities,
    pub log_level: Option<LoggingLevel>,
    pub extra: JsonObject,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RcMetaError {
    #[error("missing required RC _meta field {0}")]
    MissingField(&'static str),
    #[error("invalid RC _meta field {field}: {message}")]
    InvalidField {
        field: &'static str,
        message: String,
    },
}

impl Meta {
    pub fn new() -> Self {
        Self(JsonObject::new())
    }

    /// Create a new Meta with a progress token set
    pub fn with_progress_token(token: ProgressToken) -> Self {
        let mut meta = Self::new();
        meta.set_progress_token(token);
        meta
    }

    pub(crate) fn static_empty() -> &'static Self {
        static EMPTY: std::sync::OnceLock<Meta> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Default::default)
    }

    pub fn get_progress_token(&self) -> Option<ProgressToken> {
        self.0.get(PROGRESS_TOKEN_FIELD).and_then(|v| match v {
            Value::String(s) => Some(ProgressToken(NumberOrString::String(s.to_string().into()))),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(ProgressToken(NumberOrString::Number(i)))
                } else if let Some(u) = n.as_u64() {
                    if u <= i64::MAX as u64 {
                        Some(ProgressToken(NumberOrString::Number(u as i64)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    pub fn set_progress_token(&mut self, token: ProgressToken) {
        match token.0 {
            NumberOrString::String(ref s) => self.0.insert(
                PROGRESS_TOKEN_FIELD.to_string(),
                Value::String(s.to_string()),
            ),
            NumberOrString::Number(n) => self
                .0
                .insert(PROGRESS_TOKEN_FIELD.to_string(), Value::Number(n.into())),
        };
    }

    pub fn extend(&mut self, other: Meta) {
        for (k, v) in other.0.into_iter() {
            self.0.insert(k, v);
        }
    }

    pub fn rc_request_meta(&self) -> Result<RcRequestMeta, RcMetaError> {
        RcRequestMeta::from_meta(self)
    }

    pub fn set_rc_request_meta(&mut self, meta: RcRequestMeta) {
        meta.write_to_meta(self);
    }

    pub fn traceparent(&self) -> Option<&str> {
        self.0.get(TRACEPARENT_FIELD).and_then(Value::as_str)
    }

    pub fn tracestate(&self) -> Option<&str> {
        self.0.get(TRACESTATE_FIELD).and_then(Value::as_str)
    }

    pub fn baggage(&self) -> Option<&str> {
        self.0.get(BAGGAGE_FIELD).and_then(Value::as_str)
    }
}

impl RcRequestMeta {
    pub fn from_meta(meta: &Meta) -> Result<Self, RcMetaError> {
        fn required<T: for<'de> Deserialize<'de>>(
            meta: &Meta,
            field: &'static str,
        ) -> Result<T, RcMetaError> {
            let value = meta
                .0
                .get(field)
                .cloned()
                .ok_or(RcMetaError::MissingField(field))?;
            serde_json::from_value(value).map_err(|e| RcMetaError::InvalidField {
                field,
                message: e.to_string(),
            })
        }

        let protocol_version = required(meta, RC_PROTOCOL_VERSION_FIELD)?;
        let client_info = required(meta, RC_CLIENT_INFO_FIELD)?;
        let client_capabilities = required(meta, RC_CLIENT_CAPABILITIES_FIELD)?;
        let log_level = meta
            .0
            .get(RC_LOG_LEVEL_FIELD)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| RcMetaError::InvalidField {
                field: RC_LOG_LEVEL_FIELD,
                message: e.to_string(),
            })?;

        let extra = meta
            .0
            .iter()
            .filter(|(key, _)| {
                !matches!(
                    key.as_str(),
                    RC_PROTOCOL_VERSION_FIELD
                        | RC_CLIENT_INFO_FIELD
                        | RC_CLIENT_CAPABILITIES_FIELD
                        | RC_LOG_LEVEL_FIELD
                )
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        Ok(Self {
            protocol_version,
            client_info,
            client_capabilities,
            log_level,
            extra,
        })
    }

    pub fn write_to_meta(self, meta: &mut Meta) {
        meta.0.extend(self.extra);
        meta.0.insert(
            RC_PROTOCOL_VERSION_FIELD.to_string(),
            Value::String(self.protocol_version.to_string()),
        );
        meta.0.insert(
            RC_CLIENT_INFO_FIELD.to_string(),
            serde_json::to_value(self.client_info)
                .expect("Implementation serializes to a JSON value"),
        );
        meta.0.insert(
            RC_CLIENT_CAPABILITIES_FIELD.to_string(),
            serde_json::to_value(self.client_capabilities)
                .expect("ClientCapabilities serializes to a JSON value"),
        );
        if let Some(log_level) = self.log_level {
            meta.0.insert(
                RC_LOG_LEVEL_FIELD.to_string(),
                serde_json::to_value(log_level).expect("LoggingLevel serializes to a JSON value"),
            );
        }
    }
}

impl Deref for Meta {
    type Target = JsonObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Meta {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Req, Resp, Noti> JsonRpcMessage<Req, Resp, Noti>
where
    Req: GetExtensions,
    Noti: GetExtensions,
{
    pub fn insert_extension<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
        match self {
            JsonRpcMessage::Request(json_rpc_request) => {
                json_rpc_request.request.extensions_mut().insert(value);
            }
            JsonRpcMessage::Notification(json_rpc_notification) => {
                json_rpc_notification
                    .notification
                    .extensions_mut()
                    .insert(value);
            }
            _ => {}
        }
    }
}
