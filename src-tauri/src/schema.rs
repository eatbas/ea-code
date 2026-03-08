// @generated — Diesel schema definition for EA Code.
// Manually authored to match migrations/2026-03-08-000001_create_initial_tables/up.sql.

diesel::table! {
    settings (id) {
        id -> Integer,
        claude_path -> Text,
        codex_path -> Text,
        gemini_path -> Text,
        kimi_path -> Text,
        opencode_path -> Text,
        copilot_path -> Text,
        prompt_enhancer_agent -> Text,
        skill_selector_agent -> Nullable<Text>,
        planner_agent -> Nullable<Text>,
        plan_auditor_agent -> Nullable<Text>,
        generator_agent -> Text,
        reviewer_agent -> Text,
        fixer_agent -> Text,
        final_judge_agent -> Text,
        max_iterations -> Integer,
        require_git -> Bool,
        updated_at -> Timestamp,
        claude_model -> Text,
        codex_model -> Text,
        gemini_model -> Text,
        kimi_model -> Text,
        opencode_model -> Text,
        copilot_model -> Text,
        prompt_enhancer_model -> Text,
        skill_selector_model -> Nullable<Text>,
        planner_model -> Nullable<Text>,
        plan_auditor_model -> Nullable<Text>,
        generator_model -> Text,
        reviewer_model -> Text,
        fixer_model -> Text,
        final_judge_model -> Text,
        executive_summary_agent -> Text,
        executive_summary_model -> Text,
        require_plan_approval -> Bool,
        plan_auto_approve_timeout_sec -> Integer,
        max_plan_revisions -> Integer,
        token_optimized_prompts -> Bool,
        agent_retry_count -> Integer,
        agent_timeout_ms -> Integer,
        agent_max_turns -> Integer,
        mode -> Text,
        update_cli_on_run -> Bool,
        fail_on_cli_update_error -> Bool,
        cli_update_timeout_ms -> Integer,
        skill_selection_mode -> Text,
    }
}

diesel::table! {
    mcp_servers (id) {
        id -> Text,
        name -> Text,
        description -> Text,
        command -> Text,
        args -> Text,
        env -> Text,
        is_enabled -> Bool,
        is_builtin -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    cli_mcp_bindings (id) {
        id -> Integer,
        cli_name -> Text,
        mcp_server_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    project_settings (id) {
        id -> Integer,
        project_id -> Integer,
        setting_key -> Text,
        setting_value -> Text,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    skills (id) {
        id -> Text,
        name -> Text,
        description -> Text,
        instructions -> Text,
        tags -> Text,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    projects (id) {
        id -> Integer,
        path -> Text,
        name -> Text,
        is_git_repo -> Bool,
        branch -> Nullable<Text>,
        last_opened -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    sessions (id) {
        id -> Text,
        project_id -> Integer,
        title -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    runs (id) {
        id -> Text,
        session_id -> Text,
        prompt -> Text,
        status -> Text,
        max_iterations -> Integer,
        final_verdict -> Nullable<Text>,
        error -> Nullable<Text>,
        executive_summary -> Nullable<Text>,
        executive_summary_status -> Nullable<Text>,
        executive_summary_error -> Nullable<Text>,
        executive_summary_agent -> Nullable<Text>,
        executive_summary_model -> Nullable<Text>,
        executive_summary_generated_at -> Nullable<Timestamp>,
        started_at -> Timestamp,
        completed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    iterations (id) {
        id -> Integer,
        run_id -> Text,
        number -> Integer,
        verdict -> Nullable<Text>,
        judge_reasoning -> Nullable<Text>,
        enhanced_prompt -> Nullable<Text>,
        planner_plan -> Nullable<Text>,
        audit_verdict -> Nullable<Text>,
        audit_reasoning -> Nullable<Text>,
        audited_plan -> Nullable<Text>,
        review_output -> Nullable<Text>,
        review_user_guidance -> Nullable<Text>,
        fix_output -> Nullable<Text>,
        judge_output -> Nullable<Text>,
        generate_question -> Nullable<Text>,
        generate_answer -> Nullable<Text>,
        fix_question -> Nullable<Text>,
        fix_answer -> Nullable<Text>,
        plan_approval -> Nullable<Text>,
        plan_revision_count -> Integer,
    }
}

diesel::table! {
    stages (id) {
        id -> Integer,
        iteration_id -> Integer,
        stage -> Text,
        status -> Text,
        output -> Text,
        duration_ms -> Integer,
        error -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    logs (id) {
        id -> Integer,
        run_id -> Text,
        stage -> Text,
        line -> Text,
        stream -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    artifacts (id) {
        id -> Integer,
        run_id -> Text,
        iteration -> Integer,
        kind -> Text,
        content -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    questions (id) {
        id -> Text,
        run_id -> Text,
        stage -> Text,
        iteration -> Integer,
        question_text -> Text,
        agent_output -> Text,
        optional -> Bool,
        answer -> Nullable<Text>,
        skipped -> Bool,
        asked_at -> Timestamp,
        answered_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(sessions -> projects (project_id));
diesel::joinable!(project_settings -> projects (project_id));
diesel::joinable!(cli_mcp_bindings -> mcp_servers (mcp_server_id));
diesel::joinable!(runs -> sessions (session_id));
diesel::joinable!(iterations -> runs (run_id));
diesel::joinable!(stages -> iterations (iteration_id));
diesel::joinable!(logs -> runs (run_id));
diesel::joinable!(artifacts -> runs (run_id));
diesel::joinable!(questions -> runs (run_id));

diesel::allow_tables_to_appear_in_same_query!(
    settings, mcp_servers, cli_mcp_bindings, skills, projects, project_settings, sessions, runs, iterations, stages, logs, artifacts, questions,
);
