import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Monitor, Moon, Sun } from "lucide-react";
import { GoldfishLoader } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { SectionLabel } from "../components/SectionLabel";
import { Segmented } from "../components/Segmented";
import { useI18n, type LanguageSetting } from "../lib/i18n";
import { useTheme, type ThemeSetting } from "../lib/theme";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-10 items-center justify-between gap-6 border-b border-border/60 py-1.5 last:border-0">
      <span className="text-sm">{label}</span>
      <div className="min-w-0 text-right">{children}</div>
    </div>
  );
}

export function SettingsPage() {
  const { t, languageSetting, setLocale } = useI18n();
  const { theme, setTheme } = useTheme();
  const status = useQuery({ queryKey: queryKeys.status, queryFn: api.status });
  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: api.getSettings });

  const languageOptions: { value: LanguageSetting; label: string }[] = [
    { value: "ja", label: "日本語" },
    { value: "en", label: "English" },
    { value: "system", label: t("settings.language.system") },
  ];
  const themeOptions: { value: ThemeSetting; label: ReactNode }[] = [
    { value: "light", label: (<><Sun className="h-3.5 w-3.5" /> {t("settings.theme.light")}</>) },
    { value: "dark", label: (<><Moon className="h-3.5 w-3.5" /> {t("settings.theme.dark")}</>) },
    { value: "system", label: (<><Monitor className="h-3.5 w-3.5" /> {t("settings.theme.system")}</>) },
  ];

  const vault: { label: string; value: string | number | undefined }[] = [
    { label: t("settings.vault.dataDir"), value: status.data?.data_dir },
    { label: t("settings.vault.databasePath"), value: status.data?.database_path },
    { label: t("settings.vault.masterKey"), value: status.data?.master_key_location },
    { label: t("settings.vault.keyBackend"), value: settings.data?.key_backend },
    { label: t("settings.vault.projects"), value: status.data?.project_count },
    { label: t("settings.vault.secrets"), value: status.data?.secret_count },
  ];

  return (
    <div className="max-w-2xl">
      <PageHeader title={t("settings.title")} />

      <section className="mb-8">
        <SectionLabel>{t("settings.appearance.title")}</SectionLabel>
        <Row label={t("settings.language.title")}>
          <Segmented value={languageSetting} onChange={setLocale} options={languageOptions} ariaLabel={t("lang.label")} />
        </Row>
        <Row label={t("settings.theme.title")}>
          <Segmented value={theme} onChange={setTheme} options={themeOptions} ariaLabel={t("settings.theme.title")} />
        </Row>
      </section>

      <section>
        <SectionLabel>{t("settings.vault.title")}</SectionLabel>
        {status.error && <ErrorNote error={status.error} />}
        {settings.error && <ErrorNote error={settings.error} />}
        {status.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
        {status.data &&
          vault.map((r) => (
            <Row key={r.label} label={r.label}>
              <span className="break-all font-mono text-xs text-muted-foreground">{r.value ?? "—"}</span>
            </Row>
          ))}
        <p className="mt-4 font-mono text-[11px] text-muted-foreground">
          {t("settings.vault.keychainNote")} <code className="rounded bg-muted px-1 py-0.5 text-foreground">envfish vault key-backend keychain</code>
        </p>
      </section>
    </div>
  );
}
