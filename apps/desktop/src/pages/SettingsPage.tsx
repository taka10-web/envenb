import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Monitor, Moon, Sun } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, Goldfish, GoldfishLoader } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { Segmented } from "../components/Segmented";
import { useI18n, type LanguageSetting } from "../lib/i18n";
import { useTheme, type ThemeSetting } from "../lib/theme";

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
    {
      value: "light",
      label: (
        <>
          <Sun className="h-3.5 w-3.5" /> {t("settings.theme.light")}
        </>
      ),
    },
    {
      value: "dark",
      label: (
        <>
          <Moon className="h-3.5 w-3.5" /> {t("settings.theme.dark")}
        </>
      ),
    },
    {
      value: "system",
      label: (
        <>
          <Monitor className="h-3.5 w-3.5" /> {t("settings.theme.system")}
        </>
      ),
    },
  ];

  const rows: { label: string; value: string | number | undefined }[] = [
    { label: t("settings.vault.dataDir"), value: status.data?.data_dir },
    { label: t("settings.vault.databasePath"), value: status.data?.database_path },
    { label: t("settings.vault.masterKey"), value: status.data?.master_key_location },
    { label: t("settings.vault.keyBackend"), value: settings.data?.key_backend },
    { label: t("settings.vault.projects"), value: status.data?.project_count },
    { label: t("settings.vault.secrets"), value: status.data?.secret_count },
  ];

  return (
    <div className="p-8">
      <PageHeader title={t("settings.title")} description={t("settings.description")} />

      <div className="grid max-w-3xl gap-4">
        <Card>
          <CardHeader>
            <CardTitle>{t("settings.language.title")}</CardTitle>
            <CardDescription>{t("settings.language.description")}</CardDescription>
          </CardHeader>
          <CardContent>
            <Segmented value={languageSetting} onChange={setLocale} options={languageOptions} ariaLabel={t("lang.label")} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("settings.theme.title")}</CardTitle>
            <CardDescription>{t("settings.theme.description")}</CardDescription>
          </CardHeader>
          <CardContent>
            <Segmented value={theme} onChange={setTheme} options={themeOptions} ariaLabel={t("settings.theme.title")} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-start justify-between space-y-0">
            <div className="space-y-1.5">
              <CardTitle>{t("settings.vault.title")}</CardTitle>
              <CardDescription>{t("settings.vault.description")}</CardDescription>
            </div>
            <div className="flex items-end gap-1.5" aria-hidden>
              <Goldfish variant="red" size={3} />
              <Goldfish variant="nishiki" size={3} />
              <Goldfish variant="demekin" size={3} />
            </div>
          </CardHeader>
          <CardContent>
            {status.error && <ErrorNote error={status.error} />}
            {settings.error && <ErrorNote error={settings.error} />}
            {status.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
            {status.data && (
              <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
                {rows.map((r) => (
                  <div key={r.label} className="contents">
                    <dt className="text-muted-foreground">{r.label}</dt>
                    <dd className="break-all font-mono text-xs leading-5">{r.value ?? "—"}</dd>
                  </div>
                ))}
              </dl>
            )}
            <p className="mt-4 text-xs text-muted-foreground">
              {t("settings.vault.keychainNote")} <code className="rounded bg-muted px-1 py-0.5 font-mono">envfish vault key-backend keychain</code>
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
