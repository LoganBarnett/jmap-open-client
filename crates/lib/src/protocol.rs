//! JMAP wire-protocol envelope, per RFC 8620 §3.
//!
//! The types here model the request and response *shapes* the protocol
//! requires while keeping the method-call argument set deliberately
//! open.  A [`MethodCall`] carries an arbitrary [`serde_json::Value`]
//! as its arguments, so any JMAP method — standard mail, Sieve,
//! Calendars, vendor extensions — can be invoked through the same
//! envelope without modification to this crate.
//!
//! # Open-extension pattern
//!
//! Define typed argument and response structs in your own crate, then
//! use [`MethodCall::new`] to serialize the typed value into the
//! envelope:
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use jmap_open_client_lib::protocol::{MethodCall, Request};
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct MailboxGetArgs {
//!     #[serde(rename = "accountId")]
//!     account_id: String,
//!     ids: Option<Vec<String>>,
//! }
//!
//! let args = MailboxGetArgs {
//!     account_id: "u1".into(),
//!     ids: None,
//! };
//!
//! let mut request = Request::new(["urn:ietf:params:jmap:mail"]);
//! request.push(MethodCall::new("Mailbox/get", &args, "0")?);
//! # Ok(())
//! # }
//! ```
//!
//! The same pattern carries Stalwart's management types
//! (`SpamTag/set`, `DkimSignature/set`, …) — the envelope does not
//! know or care which extension a method belongs to, only that the
//! corresponding capability URI appears in [`Request::using`].

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A JMAP request envelope.
///
/// See RFC 8620 §3.3.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Request {
  /// JMAP capability URIs the caller is using.  The server must
  /// support every URI in this list or the entire request is
  /// rejected with `unknownCapability`.
  pub using: Vec<String>,

  /// Ordered list of method invocations.  The server executes them
  /// in sequence and result references resolve against earlier
  /// responses in the same request.
  #[serde(rename = "methodCalls")]
  pub method_calls: Vec<MethodCall>,

  /// Client-supplied creation identifiers, used when subsequent
  /// calls in the same request need to reference objects created by
  /// earlier calls.
  #[serde(rename = "createdIds", skip_serializing_if = "Option::is_none")]
  pub created_ids: Option<HashMap<String, String>>,
}

impl Request {
  /// Build a new request that advertises the given capability URIs
  /// and has no method calls yet.  Append calls with [`push`].
  ///
  /// [`push`]: Self::push
  pub fn new(using: impl IntoIterator<Item = impl Into<String>>) -> Self {
    Self {
      using: using.into_iter().map(Into::into).collect(),
      method_calls: Vec::new(),
      created_ids: None,
    }
  }

  /// Append a method call to the request.
  pub fn push(&mut self, call: MethodCall) -> &mut Self {
    self.method_calls.push(call);
    self
  }
}

/// A single method invocation: `(method-name, arguments, call-id)`.
///
/// Serialized as the three-element JSON array required by RFC 8620
/// §3.2.  Construct via [`new`] when working from a typed argument
/// value, or by wrapping a tuple directly when the arguments are
/// already a [`Value`].
///
/// [`new`]: Self::new
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct MethodCall(pub (String, Value, String));

impl MethodCall {
  /// Build a method call from a typed argument value.  Returns an
  /// error when `args` fails to serialize to JSON.
  pub fn new<T: Serialize>(
    method: impl Into<String>,
    args: &T,
    call_id: impl Into<String>,
  ) -> Result<Self, serde_json::Error> {
    let args_value = serde_json::to_value(args)?;
    Ok(Self((method.into(), args_value, call_id.into())))
  }

  /// The method name (for example `"Mailbox/get"`).
  pub fn method(&self) -> &str {
    &self.0 .0
  }

  /// The argument payload.
  pub fn args(&self) -> &Value {
    &self.0 .1
  }

  /// The client-chosen call identifier echoed back in the matching
  /// response.
  pub fn call_id(&self) -> &str {
    &self.0 .2
  }
}

/// A JMAP response envelope.
///
/// See RFC 8620 §3.4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Response {
  /// One response per method call in the request, in invocation
  /// order.
  #[serde(rename = "methodResponses")]
  pub method_responses: Vec<MethodResponse>,

  /// Server-assigned identifiers for objects the request created
  /// with `createdIds` placeholders.
  #[serde(
    rename = "createdIds",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  pub created_ids: Option<HashMap<String, String>>,

  /// Session state at the time the request was processed.
  /// Consumers can compare this against later responses to detect
  /// changes that warrant re-fetching the session object.
  #[serde(rename = "sessionState")]
  pub session_state: String,
}

/// A single method response: `(name, payload, call-id)`.
///
/// When `name` is the literal string `"error"`, `payload` is a
/// [`MethodError`].  Otherwise the payload is method-specific.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct MethodResponse(pub (String, Value, String));

impl MethodResponse {
  /// The response name — either the original method name on
  /// success or the literal string `"error"`.
  pub fn name(&self) -> &str {
    &self.0 .0
  }

  /// The response payload.
  pub fn payload(&self) -> &Value {
    &self.0 .1
  }

  /// The call identifier copied from the original [`MethodCall`].
  pub fn call_id(&self) -> &str {
    &self.0 .2
  }

  /// Whether this response carries an error.
  pub fn is_error(&self) -> bool {
    self.name() == "error"
  }

  /// Decode the payload as a [`MethodError`] when the response is
  /// an error.  Returns `Ok(None)` for non-error responses and
  /// propagates any deserialization failure when the payload does
  /// not match the expected error shape.
  pub fn as_error(&self) -> Result<Option<MethodError>, serde_json::Error> {
    if self.is_error() {
      serde_json::from_value(self.payload().clone()).map(Some)
    } else {
      Ok(None)
    }
  }
}

/// An error returned in place of a method response.
///
/// See RFC 8620 §3.6.2.  The `type` field is mandatory; additional
/// fields defined by individual method specifications are captured
/// in [`extra`].
///
/// [`extra`]: Self::extra
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MethodError {
  /// Error type identifier — for example `"invalidArguments"`,
  /// `"unknownMethod"`, `"tooManyChanges"`.
  #[serde(rename = "type")]
  pub error_type: String,

  /// Optional human-readable description.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,

  /// Any additional fields the error specification carries.  For
  /// example a `"tooManyChanges"` error includes a `maxChanges`
  /// field that ends up here.
  #[serde(flatten)]
  pub extra: HashMap<String, Value>,
}

/// A back-reference to a previous method response's output.
///
/// Per RFC 8620 §3.7, an argument value can be replaced by a
/// `ResultReference` by prefixing the corresponding argument-object
/// field name with `#`.  For example, to use the `created` list from
/// a prior `Mailbox/changes` call as the `ids` argument of a later
/// `Mailbox/get` call, include `"#ids"` in the second call's argument
/// object with a `ResultReference` value.
///
/// The wrapping field-name prefix is *not* part of this struct; it
/// lives in the surrounding argument object the caller constructs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResultReference {
  /// Call identifier of the method whose response should be used.
  #[serde(rename = "resultOf")]
  pub result_of: String,

  /// Method name of the referenced response.
  pub name: String,

  /// JSON Pointer (RFC 6901) into the referenced response payload.
  pub path: String,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn request_round_trips_through_serde() {
    // Shape from RFC 8620 §3.3, lightly trimmed to drop the
    // `accountId` placeholder for compactness while preserving the
    // structural elements the envelope must round-trip cleanly.
    let wire = json!({
      "using": [
        "urn:ietf:params:jmap:core",
        "urn:ietf:params:jmap:mail"
      ],
      "methodCalls": [
        ["Mailbox/get", {"accountId": "u1", "ids": null}, "0"],
        ["Mailbox/changes", {"accountId": "u1", "sinceState": "1"}, "1"]
      ]
    });
    let request: Request = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(request.using.len(), 2);
    assert_eq!(request.method_calls.len(), 2);
    assert_eq!(request.method_calls[0].method(), "Mailbox/get");
    assert_eq!(request.method_calls[0].call_id(), "0");
    assert!(request.created_ids.is_none());
    assert_eq!(serde_json::to_value(&request).unwrap(), wire);
  }

  #[test]
  fn request_builder_appends_calls_in_order() {
    let mut request = Request::new(["urn:ietf:params:jmap:core"]);
    request
      .push(MethodCall::new("Foo/get", &json!({"ids": ["x"]}), "0").unwrap())
      .push(
        MethodCall::new("Foo/changes", &json!({"sinceState": "1"}), "1")
          .unwrap(),
      );
    assert_eq!(request.method_calls.len(), 2);
    assert_eq!(request.method_calls[0].call_id(), "0");
    assert_eq!(request.method_calls[1].call_id(), "1");
  }

  #[test]
  fn response_round_trips_through_serde() {
    let wire = json!({
      "methodResponses": [
        ["Mailbox/get", {"accountId": "u1", "state": "abc", "list": []}, "0"]
      ],
      "sessionState": "abc-session"
    });
    let response: Response = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(response.session_state, "abc-session");
    assert_eq!(response.method_responses.len(), 1);
    assert!(!response.method_responses[0].is_error());
    assert_eq!(serde_json::to_value(&response).unwrap(), wire);
  }

  #[test]
  fn method_response_decodes_error() {
    let wire = json!(["error", {"type": "invalidArguments", "description": "bad ids"}, "0"]);
    let resp: MethodResponse = serde_json::from_value(wire).unwrap();
    assert!(resp.is_error());
    let err = resp
      .as_error()
      .expect("error decode")
      .expect("error present");
    assert_eq!(err.error_type, "invalidArguments");
    assert_eq!(err.description.as_deref(), Some("bad ids"));
    assert!(err.extra.is_empty());
  }

  #[test]
  fn method_response_non_error_yields_none() {
    let wire = json!(["Mailbox/get", {"foo": "bar"}, "x"]);
    let resp: MethodResponse = serde_json::from_value(wire).unwrap();
    assert!(!resp.is_error());
    assert!(resp.as_error().unwrap().is_none());
  }

  #[test]
  fn method_error_captures_extra_fields() {
    // `tooManyChanges` (RFC 8620 §5.2) carries a `maxChanges` field
    // beyond the standard `type`/`description` pair.  Extras must
    // round-trip via `#[serde(flatten)]`.
    let wire = json!({
      "type": "tooManyChanges",
      "description": "limit exceeded",
      "maxChanges": 1000
    });
    let err: MethodError = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(err.error_type, "tooManyChanges");
    assert_eq!(err.extra.get("maxChanges").unwrap(), &json!(1000));
    assert_eq!(serde_json::to_value(&err).unwrap(), wire);
  }

  #[test]
  fn method_call_builder_serializes_typed_args() {
    #[derive(Serialize)]
    struct Args {
      #[serde(rename = "accountId")]
      account_id: String,
    }
    let call = MethodCall::new(
      "Mailbox/get",
      &Args {
        account_id: "u1".into(),
      },
      "0",
    )
    .unwrap();
    assert_eq!(call.method(), "Mailbox/get");
    assert_eq!(call.call_id(), "0");
    assert_eq!(call.args(), &json!({"accountId": "u1"}));
  }

  #[test]
  fn result_reference_round_trips_through_serde() {
    let wire = json!({
      "resultOf": "0",
      "name": "Mailbox/changes",
      "path": "/created"
    });
    let r: ResultReference = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(r.result_of, "0");
    assert_eq!(r.name, "Mailbox/changes");
    assert_eq!(r.path, "/created");
    assert_eq!(serde_json::to_value(&r).unwrap(), wire);
  }

  #[test]
  fn result_reference_in_args_uses_hash_prefixed_field() {
    // The `#`-prefixed field name lives in the surrounding argument
    // object the caller builds, not in `ResultReference` itself.
    // Demonstrate that the envelope round-trips the construction.
    let args = json!({
      "accountId": "u1",
      "#ids": {
        "resultOf": "0",
        "name": "Mailbox/changes",
        "path": "/created"
      }
    });
    let call = MethodCall(("Email/get".into(), args.clone(), "1".into()));
    let wire = json!(["Email/get", args, "1"]);
    assert_eq!(serde_json::to_value(&call).unwrap(), wire);
  }
}
