// Copyright (c) 2019 libechoexec developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `libechoexec` Error Handling

use std::error::Error;
use std::fmt;

/// A result that includes a `Error`
pub type Result<T> = std::result::Result<T, Err>;

/// An error thrown by `libechoexec`
#[derive(Debug)]
pub struct Err {
    /// The kind of error
    inner: ErrKind,
}

impl Error for Err {
    fn description(&self) -> &str {
        "libechoexec error"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}

impl fmt::Display for Err {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())?;

        if let Some(source) = self.inner.source() {
            write!(f, ": {}", source)?;
        }
        write!(f, ": {}", self.inner)
    }
}

macro_rules! external_error {
    ($error:ty, $kind:expr) => {
        impl From<$error> for Err {
            fn from(inner: $error) -> Self {
                Self {
                    inner: $kind(inner),
                }
            }
        }
    };
}

impl From<ErrKind> for Err {
    fn from(inner: ErrKind) -> Self {
        Self { inner }
    }
}

impl From<&str> for Err {
    fn from(inner: &str) -> Self {
        Self {
            inner: ErrKind::Str(inner.to_string()),
        }
    }
}

external_error!(hyper::Error, ErrKind::Hyper);
external_error!(hyper::http::Error, ErrKind::HyperHTTP);
external_error!(hyper_tls::Error, ErrKind::HyperTLS);
external_error!(serde_json::Error, ErrKind::SerdeJson);
external_error!(std::io::Error, ErrKind::Io);
external_error!(String, ErrKind::Str);
external_error!(std::env::VarError, ErrKind::Var);
external_error!(uuid::parser::ParseError, ErrKind::ParseUuid);

/// The error kind of an error thrown by `libechoexec`
#[derive(Debug)]
pub enum ErrKind {
    /// An error from the `hyper` library
    Hyper(hyper::Error),
    /// An HTTP error from the `hyper` library
    HyperHTTP(hyper::http::Error),
    /// An error from the `hyper-tls` library
    HyperTLS(hyper_tls::Error),
    /// An Io error
    Io(std::io::Error),
    /// An error parsing a UUID
    ParseUuid(uuid::parser::ParseError),
    /// An error from the `serde_json` library
    SerdeJson(serde_json::Error),
    /// An error string
    Str(String),
    /// An env `VarError`
    Var(std::env::VarError),
    /// Error during `Runnable` run
    Run,
}

impl Error for ErrKind {
    fn description(&self) -> &str {
        match self {
            ErrKind::Hyper(inner) => inner.description(),
            ErrKind::HyperHTTP(inner) => inner.description(),
            ErrKind::HyperTLS(inner) => inner.description(),
            ErrKind::Io(inner) => inner.description(),
            ErrKind::ParseUuid(inner) => inner.description(),
            ErrKind::SerdeJson(inner) => inner.description(),
            ErrKind::Str(inner) => &inner[..],
            ErrKind::Var(inner) => inner.description(),
            ErrKind::Run => "An error has occurred during run",
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ErrKind::Hyper(inner) => inner.source(),
            ErrKind::HyperHTTP(inner) => inner.source(),
            ErrKind::HyperTLS(inner) => inner.source(),
            ErrKind::Io(inner) => inner.source(),
            ErrKind::ParseUuid(inner) => inner.source(),
            ErrKind::SerdeJson(inner) => inner.source(),
            ErrKind::Var(inner) => inner.source(),
            _ => None,
        }
    }
}

impl fmt::Display for ErrKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())?;
        match self {
            ErrKind::Io(inner) => write!(f, ": {}", inner),
            ErrKind::Var(inner) => write!(f, ": {}", inner),
            _ => write!(f, ""),
        }
    }
}
