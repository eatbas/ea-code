-- Reverts the swapped-path repair when values match the repaired defaults.

UPDATE settings
SET copilot_path = 'opencode',
    opencode_path = 'gh'
WHERE id = 1
  AND copilot_path = 'gh'
  AND opencode_path = 'opencode';

UPDATE project_settings
SET setting_value = '"opencode"'
WHERE setting_key = 'copilotPath'
  AND setting_value = '"gh"'
  AND EXISTS (
    SELECT 1
    FROM project_settings ps2
    WHERE ps2.project_id = project_settings.project_id
      AND ps2.setting_key = 'opencodePath'
      AND ps2.setting_value = '"opencode"'
  );

UPDATE project_settings
SET setting_value = '"gh"'
WHERE setting_key = 'opencodePath'
  AND setting_value = '"opencode"'
  AND EXISTS (
    SELECT 1
    FROM project_settings ps2
    WHERE ps2.project_id = project_settings.project_id
      AND ps2.setting_key = 'copilotPath'
      AND ps2.setting_value = '"opencode"'
  );
