import { useQuery } from "@tanstack/react-query";
import { Badge, cn, MaikoLoader } from "@envenb/ui";
import { api, queryKeys } from "../lib/api";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { Table, Td, Th, Tr } from "../components/Table";
import { useI18n } from "../lib/i18n";

export const DECISION_CLASS: Record<string, string> = {
  ALLOWED: "border-transparent bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  DENIED: "border-transparent bg-destructive/15 text-destructive",
  ASKED: "border-transparent bg-amber-500/15 text-amber-700 dark:text-amber-300",
  ERROR: "border-transparent bg-muted text-muted-foreground",
};

function formatTime(iso: string, locale: string) {
  const d = new Date(iso);
  return `${d.toLocaleDateString(locale, { month: "2-digit", day: "2-digit" })} ${d.toLocaleTimeString(locale, { hour12: false })}`;
}

export function ActivityPage() {
  const { t, locale } = useI18n();
  const audit = useQuery({ queryKey: queryKeys.audit, queryFn: () => api.listAudit(200), refetchInterval: 5000 });
  const rows = audit.data;

  return (
    <div className="flex min-h-full flex-col">
      <PageHeader title={t("activity.title")} />
      {audit.error && <ErrorNote error={audit.error} />}
      {audit.isLoading && <MaikoLoader label={t("common.loading")} className="py-16" />}
      {rows?.length === 0 && <EmptyState text={t("activity.empty")} />}
      {rows && rows.length > 0 && (
        <Table>
          <thead>
            <tr>
              <Th>{t("common.time")}</Th>
              <Th>{t("common.client")}</Th>
              <Th>
                {t("common.project")} / {t("common.environment")}
              </Th>
              <Th>{t("common.connection")}</Th>
              <Th>{t("common.action")}</Th>
              <Th>{t("common.summary")}</Th>
              <Th>{t("common.decision")}</Th>
            </tr>
          </thead>
          <tbody>
            {rows.map((e) => (
              <Tr key={e.id}>
                <Td className="whitespace-nowrap font-mono text-[11px] text-muted-foreground">{formatTime(e.created_at, locale)}</Td>
                <Td className="whitespace-nowrap text-xs">{e.client_name}</Td>
                <Td className="whitespace-nowrap font-mono text-[11px]">
                  {e.project_name ?? <span className="text-muted-foreground">—</span>}
                  {e.environment_name && <span className="text-muted-foreground"> / {e.environment_name}</span>}
                </Td>
                <Td className="whitespace-nowrap font-mono text-[11px]">{e.connection_name ?? <span className="text-muted-foreground">—</span>}</Td>
                <Td className="font-mono text-[11px]">{e.action}</Td>
                <Td className="max-w-md truncate font-mono text-[11px]" title={e.summary}>
                  {e.summary}
                </Td>
                <Td>
                  <Badge className={cn(DECISION_CLASS[e.decision] ?? DECISION_CLASS.ERROR)}>{e.decision}</Badge>
                </Td>
              </Tr>
            ))}
          </tbody>
        </Table>
      )}
    </div>
  );
}
