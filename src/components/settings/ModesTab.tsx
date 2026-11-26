import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getBooleanSetting, getStringSetting } from "../../lib/settingsGuards";
import { usePlatform } from "../../lib/usePlatform";
import { useSettingsStore } from "../../stores/settingsStore";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { SettingsGroup, SettingsHeader, SettingsRow } from "./SettingsLayout";

export function ModesTab() {
  const t = useT();
  const platform = usePlatform();
  const commandModeSupported = platform !== "macos" && platform !== "linux";
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const translationOn = getBooleanSetting(settings.translation_enabled, false);

  return (
    <div>
      <SettingsHeader
        title={t("settings.modes.title")}
        description={t("settings.modes.desc")}
      />

      <SettingsGroup title={t("settings.modes.format_group")}>
        <SettingsRow
          label={t("settings.modes.active")}
          description={t("settings.modes.active_desc")}
        >
          <Select
            aria-label={t("settings.modes.active")}
            options={[
              { value: "dictation", label: t("settings.modes.dictation") },
              { value: "message", label: t("settings.modes.message") },
              { value: "email", label: t("settings.modes.email") },
              { value: "file", label: t("settings.modes.file") },
            ]}
            value={getStringSetting(settings.active_mode, "dictation")}
            onChange={(e) =>
              void invokeAction(() => update("active_mode", e.target.value))
            }
            className="w-44"
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.modes.trans_group")}>
        {commandModeSupported && (
          <SettingsRow
            label={t("settings.general.command")}
            description={t("settings.general.command.desc")}
          >
            <Switch
              checked={getBooleanSetting(settings.command_mode, false)}
              onChange={(v) =>
                void invokeAction(() => update("command_mode", v))
              }
              ariaLabel={t("settings.general.command")}
            />
          </SettingsRow>
        )}
        <SettingsRow
          label={t("settings.modes.trans_after")}
          description={t("settings.modes.trans_desc")}
        >
          <Switch
            checked={translationOn}
            onChange={(v) =>
              void invokeAction(() => update("translation_enabled", v))
            }
            ariaLabel={t("settings.modes.trans_after")}
          />
        </SettingsRow>
        <SettingsRow label={t("settings.modes.target")}>
          <Select
            aria-label={t("settings.modes.target")}
            disabled={!translationOn}
            options={[
              { value: "en", label: "English" },
              { value: "id", label: "Bahasa Indonesia" },
            ]}
            value={getStringSetting(settings.translation_target, "en")}
            onChange={(e) =>
              void invokeAction(() =>
                update("translation_target", e.target.value),
              )
            }
            className="w-44 disabled:cursor-not-allowed disabled:opacity-50"
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}
