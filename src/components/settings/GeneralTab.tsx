import { useSettingsStore } from "../../stores/settingsStore";
import { setFloatingWidgetEnabled } from "../../lib/tauri";
import { Switch } from "../ui/Switch";
import { Select } from "../ui/Select";
import { SettingsHeader, SettingsGroup, SettingsRow } from "./SettingsLayout";
import { useT } from "../../lib/i18n";

export function GeneralTab() {
  const t = useT();
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);

  // Toggle the floating widget: update the local store optimistically and let
  // the backend persist the setting and show/hide the overlay window.
  const toggleFloatingWidget = async (v: boolean) => {
    const prev = useSettingsStore.getState().settings.floating_widget;
    useSettingsStore.setState((s) => ({
      settings: { ...s.settings, floating_widget: v },
    }));
    try {
      await setFloatingWidgetEnabled(v);
    } catch {
      if (useSettingsStore.getState().settings.floating_widget === v) {
        useSettingsStore.setState((s) => ({
          settings: { ...s.settings, floating_widget: prev },
        }));
      }
    }
  };

  return (
    <div className="max-w-xl">
      <SettingsHeader
        title={t("settings.general.title")}
        description={t("settings.general.desc")}
      />

      <SettingsGroup title={t("settings.general.appearance")}>
        <SettingsRow label={t("settings.general.lang")}>
          <Select
            options={[
              { value: "id", label: "Bahasa Indonesia" },
              { value: "en", label: "English" },
            ]}
            value={(settings.language as string) ?? "en"}
            onChange={(e) => void update("language", e.target.value)}
            className="w-48"
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.widget")}
          description={t("settings.general.widget.desc")}
        >
          <Switch
            checked={(settings.floating_widget as boolean) ?? true}
            onChange={toggleFloatingWidget}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.system")}>
        <SettingsRow
          label={t("settings.general.startup")}
          description={t("settings.general.startup.desc")}
        >
          <Switch
            checked={(settings.auto_start as boolean) ?? false}
            onChange={(v) => void update("auto_start", v)}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.updates")}
          description={t("settings.general.updates.desc")}
        >
          <Switch
            checked={(settings.auto_update as boolean) ?? true}
            onChange={(v) => void update("auto_update", v)}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.sound")}
          description={t("settings.general.sound.desc")}
        >
          <Switch
            checked={(settings.sound_cues as boolean) ?? false}
            onChange={(v) => void update("sound_cues", v)}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.privacy")}>
        <SettingsRow
          label={t("settings.general.command")}
          description={t("settings.general.command.desc")}
        >
          <Switch
            checked={(settings.command_mode as boolean) ?? false}
            onChange={(v) => void update("command_mode", v)}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.general.stats")}
          description={t("settings.general.stats.desc")}
        >
          <Switch
            checked={(settings.telemetry as boolean) ?? false}
            onChange={(v) => void update("telemetry", v)}
          />
        </SettingsRow>
      </SettingsGroup>
    </div>
  );
}
