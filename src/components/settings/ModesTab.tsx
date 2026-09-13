import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { useSettingsStore } from "../../stores/settingsStore";
import { invokeAction } from "../../lib/invokeAction";
import { useT } from "../../lib/i18n";
import { getBooleanSetting, getStringSetting } from "../../lib/settingsGuards";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";

export function ModesTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const translationOn = getBooleanSetting(settings.translation_enabled, false);

  return (
    <div className="max-w-xl">
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
            options={[
              { value: "dictation", label: t("settings.modes.dictation") },
              { value: "message", label: t("settings.modes.message") },
              { value: "email", label: t("settings.modes.email") },
            ]}
            value={getStringSetting(settings.active_mode, "dictation")}
            onChange={(e) => void invokeAction(() => update("active_mode", e.target.value))}
            className="w-44"
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.modes.trans_group")}>
        <SettingsRow
          label={t("settings.modes.trans_after")}
          description={t("settings.modes.trans_desc")}
        >
          <Switch
            checked={translationOn}
            onChange={(v) => void invokeAction(() => update("translation_enabled", v))}
          />
        </SettingsRow>
        {translationOn && (
          <SettingsRow label={t("settings.modes.target")}>
            <Select
              options={[
                { value: "en", label: "English" },
                { value: "id", label: "Bahasa Indonesia" },
              ]}
              value={getStringSetting(settings.translation_target, "en")}
              onChange={(e) => void invokeAction(() => update("translation_target", e.target.value))}
              className="w-44"
            />
          </SettingsRow>
        )}
      </SettingsGroup>
    </div>
  );
}
