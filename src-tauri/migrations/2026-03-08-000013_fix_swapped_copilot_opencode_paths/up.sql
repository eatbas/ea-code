-- Repairs swapped Copilot/OpenCode path defaults introduced in some persisted settings rows.

UPDATE settings
SET copilot_path = 'gh',
    opencode_path = 'opencode'
WHERE id = 1
  AND copilot_path = 'opencode'
  AND opencode_path = 'gh';

-- Repairs project overrides only when both values are present and swapped.
UPDATE project_settings
SET setting_value = '"gh"'
WHERE setting_key = 'copilotPath'
  AND setting_value = '"opencode"'
  AND EXISTS (
    SELECT 1
    FROM project_settings ps2
    WHERE ps2.project_id = project_settings.project_id
      AND ps2.setting_key = 'opencodePath'
      AND ps2.setting_value = '"gh"'
  );

UPDATE project_settings
SET setting_value = '"opencode"'
WHERE setting_key = 'opencodePath'
  AND setting_value = '"gh"'
  AND EXISTS (
    SELECT 1
    FROM project_settings ps2
    WHERE ps2.project_id = project_settings.project_id
      AND ps2.setting_key = 'copilotPath'
      AND ps2.setting_value = '"gh"'
  );
