ALTER TABLE settings DROP COLUMN executive_summary_model;
ALTER TABLE settings DROP COLUMN executive_summary_agent;

ALTER TABLE runs DROP COLUMN executive_summary_generated_at;
ALTER TABLE runs DROP COLUMN executive_summary_model;
ALTER TABLE runs DROP COLUMN executive_summary_agent;
ALTER TABLE runs DROP COLUMN executive_summary_error;
ALTER TABLE runs DROP COLUMN executive_summary_status;
ALTER TABLE runs DROP COLUMN executive_summary;

ALTER TABLE iterations DROP COLUMN fix_answer;
ALTER TABLE iterations DROP COLUMN fix_question;
ALTER TABLE iterations DROP COLUMN generate_answer;
ALTER TABLE iterations DROP COLUMN generate_question;
ALTER TABLE iterations DROP COLUMN judge_output;
ALTER TABLE iterations DROP COLUMN fix_output;
ALTER TABLE iterations DROP COLUMN review_user_guidance;
ALTER TABLE iterations DROP COLUMN review_output;
ALTER TABLE iterations DROP COLUMN audited_plan;
ALTER TABLE iterations DROP COLUMN audit_reasoning;
ALTER TABLE iterations DROP COLUMN audit_verdict;
ALTER TABLE iterations DROP COLUMN planner_plan;
ALTER TABLE iterations DROP COLUMN enhanced_prompt;
