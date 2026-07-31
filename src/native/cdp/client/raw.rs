// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dynamic CDP command wrapper for chromiumoxide execute paths.

use std::borrow::Cow;

use chromiumoxide::types::{Command, Method, MethodId};
use serde::Serialize;
use serde_json::Value;

/// Dynamic CDP command for `Browser::execute` / `Page::execute`.
#[derive(Debug, Clone)]
pub(crate) struct RawCdpCommand {
    pub(crate) method: String,
    pub(crate) params: Value,
}

impl Serialize for RawCdpCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.params {
            Value::Null => {
                use serde::ser::SerializeMap;
                let map = serializer.serialize_map(Some(0))?;
                map.end()
            }
            other => other.serialize(serializer),
        }
    }
}

impl Method for RawCdpCommand {
    fn identifier(&self) -> MethodId {
        Cow::Owned(self.method.clone())
    }
}

impl Command for RawCdpCommand {
    type Response = Value;
}
