// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::cli::WorkflowAction;
use crate::commands::common::emit_ok_summary;
use crate::error::CliError;

pub(crate) fn handle_workflow(action: WorkflowAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        WorkflowAction::Run { manifest, journal } => {
            crate::workflow_local::workflow_run(&manifest, journal.as_deref())?
        }
        WorkflowAction::Resume { manifest, journal } => {
            crate::workflow_local::workflow_resume(&manifest, journal.as_deref())?
        }
        WorkflowAction::Status { journal, name } => {
            crate::workflow_local::workflow_status(journal.as_deref(), name.as_deref())?
        }
    };
    emit_ok_summary(data, json, "workflow")
}
