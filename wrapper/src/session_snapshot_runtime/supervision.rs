use crate::session_prompt_status_active::format_elapsed;
use crate::state::AppState;
use crate::state::summarize_text;

pub(super) fn render_supervision_notice_runtime_lines(
    notice: &crate::state::SupervisionNotice,
    state: &AppState,
    cwd: &str,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "supervision     {} {}",
            notice.classification.label(),
            notice.tool
        ),
        format!("supervision pol {}", notice.recovery_policy_kind().label()),
        format!("supervision act {}", notice.recommended_action()),
        format!("supervision auto {}", notice.automation_ready()),
        format!("supervision req {}", notice.request_id),
        format!(
            "supervision th  {}",
            summarize_text(&notice.worker_thread_name)
        ),
        format!("supervision ow  {}", notice.owner_kind.label()),
        format!("supervision sum {}", summarize_text(&notice.summary)),
        format!("supervision ob  {}", notice.observation_state.label()),
        format!("supervision os  {}", notice.output_state.label()),
    ];
    if let Some(source_call_id) = notice.source_call_id.as_deref() {
        lines.push(format!("supervision cl  {source_call_id}"));
    }
    if let Some(target_reference) = notice.target_background_shell_reference.as_deref() {
        lines.push(format!("supervision tr  {target_reference}"));
    }
    if let Some(target_job_id) = notice.target_background_shell_job_id.as_deref() {
        lines.push(format!("supervision tj  {target_job_id}"));
    }
    if let Some(job) = notice.observed_background_shell_job.as_ref() {
        lines.push(format!("supervision jb  {} {}", job.job_id, job.status));
        lines.push(format!("supervision cmd {}", summarize_text(&job.command)));
        lines.push(format!("supervision ln  {}", job.total_lines));
        if let Some(age) = job.last_output_age {
            lines.push(format!(
                "supervision oa  {}",
                format_elapsed(Some(std::time::Instant::now() - age))
            ));
        }
        if let Some(output) = job.latest_output_preview() {
            lines.push(format!("supervision ot  {}", summarize_text(output)));
        }
    }
    for option in crate::supervision_recovery::supervision_recovery_options(
        state,
        None,
        cwd,
        notice.classification,
    ) {
        let command = option
            .terminal_command
            .or(option.cli_command)
            .unwrap_or_else(|| option.label.to_string());
        lines.push(format!(
            "supervision opt {} {}",
            option.kind,
            summarize_text(&command)
        ));
    }
    lines
}
