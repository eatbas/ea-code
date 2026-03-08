// @generated — Diesel schema definition for EA Code.
// Manually authored to match migrations/2026-03-08-000001_create_initial_tables/up.sql.

diesel::table! {
    settings (id) {
        id -> Integer,
        claude_path -> Text,
        codex_path -> Text,
        gemini_path -> Text,
        prompt_enhancer_agent -> Text,
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
        prompt_enhancer_model -> Text,
        planner_model -> Nullable<Text>,
        plan_auditor_model -> Nullable<Text>,
        generator_model -> Text,
        reviewer_model -> Text,
        fixer_model -> Text,
        final_judge_model -> Text,
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
diesel::joinable!(runs -> sessions (session_id));
diesel::joinable!(iterations -> runs (run_id));
diesel::joinable!(stages -> iterations (iteration_id));
diesel::joinable!(logs -> runs (run_id));
diesel::joinable!(artifacts -> runs (run_id));
diesel::joinable!(questions -> runs (run_id));

diesel::allow_tables_to_appear_in_same_query!(
    settings, projects, sessions, runs, iterations, stages, logs, artifacts, questions,
);
