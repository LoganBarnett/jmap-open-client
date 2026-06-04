//! Open, extensible JMAP client for Rust.
//!
//! This crate models the JMAP wire protocol (RFC 8620) as a generic
//! `(method-name, arguments, call-id)` envelope, leaving the object set
//! deliberately open.  Callers add their own typed wrappers for whatever
//! JMAP extension they need — RFC 8621 mail, the JMAP Calendars draft,
//! Stalwart's `urn:stalwart:jmap` management surface, anything — without
//! forking the crate.
//!
//! Most consumers will start with [`protocol::Request`] and
//! [`protocol::MethodCall`].  See the [`protocol`] module documentation
//! for the open-extension pattern.

pub mod protocol;
