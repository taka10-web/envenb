import { useQuery } from "@tanstack/react-query";
import { Badge, cn, GoldfishLoader } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import type { Project } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { useI18n } from "../lib/i18n";

const DECISION_CLASS: Record<string, string> = {
  ALLOWED: "border-transparent bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  DENIED: "border-transparent bg-destructive/15 text-destructive",
  ASKED: "border-transparent bg-amber-500/15 text-amber-700 dark:text-amber-300",
  ERROR: "border-transparent bg-muted text-muted-foreground",
};

export function ActivityPage({ project }: { project?: Project }) {
  const { t, locale } = useI18n();
  const audit = useQuery({ queryKey: queryKeys.audit, queryFn: () => api.listAudit(200), refetchInterval: 5000 });
  const rows = project ? audit.data?.filter((e) => e.project_id === project.id) : audit.data;

  const body = (
    <>
      {audit.error && <ErrorNote error={audit.error} />}
      {audit.isLoading && <GoldfishLoader label={t("common.loading")} className="py-16" />}
      {rows?.length === 0 && <EmptyState text={t("activity.empty")} variant="red" />}
      {rows && rows.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr className="border-b">
                <th className="py-2 pr-4 font-medium">{t("common.time")}</th>
                <th className="py-2 pr-4 font-medium">{t("common.client")}</th>
                <th className="py-2 pr-4 font-medium">
                  {t("common.project")} / {t("common.environment")}
                </th>
                <th className="py-2 pr-4 font-medium">{t("common.connection")}</th>
                <th className="py-2 pr-4 font-medium">{t("common.action")}</th>
                <th className="py-2 pr-4 font-medium">{t("common.summary")}</th>
                <th className="py-2 font-medium">{t("common.decision")}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((e) => (
                <tr key={e.id} className="border-b align-top last:border-0">
                  <td className="whitespace-nowrap py-2 pr-4 text-xs text-muted-foreground">{new Date(e.created_at).toLocaleString(locale)}</td>
                  <td className="py-2 pr-4">{e.client_name}</td>
                  <td className="py-2 pr-4 text-xs">
                    {e.project_name ?? <span className="text-muted-foreground">—</span>}
                    {e.environment_name && <span className="text-muted-foreground"> / {e.environment_name}</span>}
                  </td>
                  <td className="py-2 pr-4 text-xs">{e.connection_name ?? <span className="text-muted-foreground">—</span>}</td>
                  <td className="py-2 pr-4">
                    <Badge variant="outline">{e.action}</Badge>
                  </td>
                  <td className="max-w-md break-all py-2 pr-4 font-mono text-xs">{e.summary}</td>
                  <td className="py-2">
                    <Badge className={cn(DECISION_CLASS[e.decision] ?? DECISION_CLASS.ERROR)}>{e.decision}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );

  if (project) return <div>{body}</div>;
  return (
    <div className="p-8">
      <PageHeader title={t("activity.title")} description={t("activity.description")} />
      {body}
    </div>
  );
}
