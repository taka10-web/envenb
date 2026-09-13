import { useI18n } from "../lib/i18n";

export function ErrorNote({ error }: { error: unknown }) {
  const { t } = useI18n();
  const message = error instanceof Error ? error.message : typeof error === "string" ? error : t("common.unexpectedError");
  return (
    <p role="alert" className="mb-4 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
      {message}
    </p>
  );
}
