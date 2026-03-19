use super::super::runtime::validate_alias;
#[cfg(test)]
use crate::background_shells::BackgroundShellIntent;
use crate::background_shells::BackgroundShellManager;

impl BackgroundShellManager {
    pub(crate) fn terminate_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_terminate expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_terminate requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        self.terminate_job(&resolved_job_id)?;
        Ok(format!(
            "Termination requested for background shell job {resolved_job_id}."
        ))
    }

    #[cfg(test)]
    pub(crate) fn clean_from_tool(&self, arguments: &serde_json::Value) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_clean expects an object argument".to_string())?;
        let scope = object
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("all");
        let capability = object.get("capability").and_then(serde_json::Value::as_str);
        let terminated = match scope {
            "all" => {
                if capability.is_some() {
                    return Err(
                        "background_shell_clean `capability` is only valid with `scope=blockers` or `scope=services`"
                            .to_string(),
                    );
                }
                self.terminate_all_running()
            }
            "blockers" => match capability {
                Some(capability) => self.terminate_running_blockers_by_capability(capability)?,
                None => self.terminate_running_by_intent(BackgroundShellIntent::Prerequisite),
            },
            "shells" => {
                if capability.is_some() {
                    return Err(
                        "background_shell_clean `capability` is only valid with `scope=blockers` or `scope=services`"
                            .to_string(),
                    );
                }
                self.terminate_all_running()
            }
            "services" => match capability {
                Some(capability) => self.terminate_running_services_by_capability(capability)?,
                None => self.terminate_running_by_intent(BackgroundShellIntent::Service),
            },
            other => {
                return Err(format!(
                    "background_shell_clean `scope` must be one of `all`, `blockers`, `shells`, or `services`, got `{other}`"
                ));
            }
        };
        let summary = match (scope, capability) {
            ("blockers", Some(capability)) => format!(
                "Terminated {terminated} blocking prerequisite background shell job{} for reusable capability dependency @{}.",
                if terminated == 1 { "" } else { "s" },
                capability.trim_start_matches('@')
            ),
            ("services", Some(capability)) => format!(
                "Terminated {terminated} background shell job{} for reusable service capability @{}.",
                if terminated == 1 { "" } else { "s" },
                capability.trim_start_matches('@')
            ),
            ("services", None) => format!(
                "Terminated {terminated} service background shell job{}.",
                if terminated == 1 { "" } else { "s" }
            ),
            ("blockers", None) => format!(
                "Terminated {terminated} blocking prerequisite background shell job{}.",
                if terminated == 1 { "" } else { "s" }
            ),
            ("all" | "shells", None) => format!(
                "Terminated {terminated} local background shell job{}.",
                if terminated == 1 { "" } else { "s" }
            ),
            _ => unreachable!(),
        };
        Ok(summary)
    }

    pub(crate) fn resolve_job_reference(&self, reference: &str) -> Result<String, String> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Err("background shell job reference cannot be empty".to_string());
        }
        if reference.starts_with("bg-") {
            self.lookup_job(reference)?;
            return Ok(reference.to_string());
        }
        if let Some(capability) = reference.strip_prefix('@') {
            return self.resolve_service_capability_reference(capability);
        }
        if let Some(job_id) = self
            .snapshots()
            .into_iter()
            .find(|job| job.alias.as_deref() == Some(reference))
            .map(|job| job.id)
        {
            return Ok(job_id);
        }
        let index = reference
            .parse::<usize>()
            .map_err(|_| format!("unknown background shell job `{reference}`"))?;
        if index == 0 {
            return Err("background shell job index must be 1-based".to_string());
        }
        let snapshots = self.snapshots();
        snapshots
            .get(index - 1)
            .map(|job| job.id.clone())
            .ok_or_else(|| format!("unknown background shell job `{reference}`"))
    }

    pub(crate) fn set_job_alias(&self, job_id: &str, alias: &str) -> Result<(), String> {
        let alias = validate_alias(alias)?;
        let jobs = self.inner.jobs.lock().expect("background shell jobs lock");
        for job in jobs.values() {
            let state = job.lock().expect("background shell job lock");
            if state.id != job_id && state.alias.as_deref() == Some(alias.as_str()) {
                return Err(format!(
                    "background shell alias `{alias}` is already in use"
                ));
            }
        }
        let job = jobs
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("unknown background shell job `{job_id}`"))?;
        drop(jobs);
        let mut state = job.lock().expect("background shell job lock");
        state.alias = Some(alias);
        Ok(())
    }

    pub(crate) fn clear_job_alias(&self, alias: &str) -> Result<String, String> {
        let alias = validate_alias(alias)?;
        let jobs = self.inner.jobs.lock().expect("background shell jobs lock");
        let job = jobs
            .values()
            .find_map(|job| {
                let state = job.lock().expect("background shell job lock");
                (state.alias.as_deref() == Some(alias.as_str())).then_some(job.clone())
            })
            .ok_or_else(|| format!("unknown background shell alias `{alias}`"))?;
        drop(jobs);
        let mut state = job.lock().expect("background shell job lock");
        let job_id = state.id.clone();
        state.alias = None;
        Ok(job_id)
    }

    pub(crate) fn clear_job_alias_for_job(&self, job_id: &str) -> Result<(), String> {
        let job = self.lookup_job(job_id)?;
        let mut state = job.lock().expect("background shell job lock");
        state.alias = None;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn update_alias_from_tool(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let object = arguments
            .as_object()
            .ok_or_else(|| "background_shell_set_alias expects an object argument".to_string())?;
        let job_id = object
            .get("jobId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "background_shell_set_alias requires `jobId`".to_string())?;
        let resolved_job_id = self.resolve_job_reference(job_id)?;
        match object.get("alias") {
            Some(serde_json::Value::Null) => {
                self.clear_job_alias_for_job(&resolved_job_id)?;
                Ok(format!(
                    "Cleared alias for background shell job {resolved_job_id}."
                ))
            }
            Some(serde_json::Value::String(alias)) => {
                self.set_job_alias(&resolved_job_id, alias)?;
                Ok(format!(
                    "Aliased background shell job {resolved_job_id} as {}.",
                    validate_alias(alias)?
                ))
            }
            Some(_) => {
                Err("background_shell_set_alias `alias` must be a string or null".to_string())
            }
            None => Err("background_shell_set_alias requires `alias`".to_string()),
        }
    }

    pub(crate) fn terminate_job_for_operator(&self, job_id: &str) -> Result<(), String> {
        self.terminate_job(job_id)
    }
}
