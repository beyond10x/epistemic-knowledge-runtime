//! A consumer's client for an EKR store (`epic:consumer-sdk`).
//!
//! It talks to a store only through a child `ekr session`, behind [`transport`], and builds the
//! documents a consumer writes with [`document`]. It depends on `ekr-core` alone: the kernel, the
//! store and the graph are the child process's, never linked into a consumer.

pub mod binary;
pub mod document;
pub mod reply;
pub mod session;
pub mod transport;
pub mod viewer;
