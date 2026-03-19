//! Skill selection prompt utilities and output parsing.

use std::collections::HashSet;



#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSkillSelection {
    selected_skill_ids: Vec<String>,
    reason: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SkillSelectionDecision {
    pub selected_skill_ids: Vec<String>,
    pub reason: String,
}

pub fn build_skill_selector_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plan: Option<&str>,
    previous_judge_output: Option<&str>,
    skill_catalog_json: &str,
) -> String {
    let mut sections = vec![
        format!("USER PROMPT (ORIGINAL):\n{original_prompt}"),
        if enhanced_prompt != original_prompt {
            format!("ENHANCED EXECUTION PROMPT:\n{enhanced_prompt}")
        } else {
            String::new()
        },
        match plan {
            Some(plan_text) if !plan_text.trim().is_empty() => {
                format!("APPROVED EXECUTION PLAN:\n{}", plan_text.trim())
            }
            _ => "APPROVED EXECUTION PLAN:\n(No approved plan. Use the enhanced prompt as fallback context.)".to_string(),
        },
        match previous_judge_output {
            Some(judge) if !judge.trim().is_empty() => judge.trim().to_string(),
            _ => String::new(),
        },
        format!("SKILL CATALOG (JSON, metadata only):\n{skill_catalog_json}"),
    ];
    sections.retain(|section| !section.is_empty());
    sections.join("\n\n")
}

pub fn build_selected_skills_section(skills: &[crate::models::SkillFile]) -> String {
    if skills.is_empty() {
        return String::new();
    }

    let mut blocks = Vec::with_capacity(skills.len() + 1);
    blocks.push("SELECTED SKILLS (apply relevant guidance below):".to_string());

    for skill in skills {
        let instructions = skill.prompt.trim();
        blocks.push(format!(
            "### [{}] {}\nDescription: {}\nInstructions:\n{}",
            skill.id,
            skill.name,
            skill.description,
            if instructions.is_empty() {
                "(No instructions provided.)"
            } else {
                instructions
            }
        ));
    }

    blocks.join("\n\n")
}

pub fn parse_skill_selection_output(
    output: &str,
    known_skill_ids: &HashSet<String>,
    max_selections: usize,
) -> Result<SkillSelectionDecision, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("Skill selector returned an empty response.".to_string());
    }

    let parsed = parse_selection_json(trimmed)
        .or_else(|| extract_json_object(trimmed).and_then(parse_selection_json))
        .ok_or_else(|| {
            "Skill selector response did not contain valid JSON fields: selectedSkillIds, reason."
                .to_string()
        })?;

    let bounded = max_selections.max(1);
    let mut selected = Vec::new();

    for raw_id in parsed.selected_skill_ids {
        let id = raw_id.trim().to_string();
        if id.is_empty() || !known_skill_ids.contains(&id) || selected.contains(&id) {
            continue;
        }
        selected.push(id);
        if selected.len() >= bounded {
            break;
        }
    }

    Ok(SkillSelectionDecision {
        selected_skill_ids: selected,
        reason: parsed.reason.unwrap_or_default().trim().to_string(),
    })
}

fn parse_selection_json(raw_json: &str) -> Option<RawSkillSelection> {
    serde_json::from_str::<RawSkillSelection>(raw_json).ok()
}

fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&text[start..=end])
}
