//! GenExpress's MCP server (layer L6).
//!
//! [Model Context Protocol](https://modelcontextprotocol.io) over stdio: newline-delimited
//! JSON-RPC 2.0, hand-written (no async runtime). The tools forward to a [`Backend`]:
//!
//! - [`Remote`] talks to a running desktop app through its loopback JSON-lines control channel
//!   (`genexpress --control 7979`): one `{"id","method","params"}` line in, one
//!   `{"id","ok","result"|"error"}` line out.
//! - [`Headless`] hosts an in-process [`genexpress_engine::Session`] and answers the same
//!   control-channel methods itself, so agents can design without a window.
#![forbid(unsafe_code)]

mod backend;
mod server;
mod tools;

pub use backend::{Backend, Headless, Remote, RenderHook};
pub use server::{PROTOCOL_VERSION, Server};
pub use tools::tool_definitions;

/// Default control-channel address of the desktop app.
pub const DEFAULT_ADDR: &str = "127.0.0.1:7979";

#[cfg(test)]
mod tests;
