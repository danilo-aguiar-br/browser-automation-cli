// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::cli::ConfigAction;
use crate::commands::common::emit_ok;
use crate::error::CliError;

pub(crate) fn handle_config(action: ConfigAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        ConfigAction::Path => crate::xdg::paths_snapshot()?,
        ConfigAction::Init => crate::xdg::init_layout()?,
        ConfigAction::Show => crate::xdg::config_get(None)?,
        ConfigAction::Set { key, value } => crate::xdg::config_set(&key, &value)?,
        ConfigAction::Get { key } => crate::xdg::config_get(key.as_deref())?,
        ConfigAction::ListKeys => crate::xdg::config_list_keys()?,
    };
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok config {d}"))
    })
}
