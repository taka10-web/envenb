import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bot, Check, Plus, Trash2, X } from "lucide-react";
import { Badge, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, cn, GoldfishInline, GoldfishLoader, Input, Label } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { ACTIONS, DECISIONS, type Action, type Decision, type Permission, type Project } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { Select } from "../components/Select";
import { Segmented } from "../components/Segmented";
import { useI18n } from "../lib/i18n";
import { confirmAsync } from "../lib/confirm";

const CLIENT_KINDS = ["claude_code", "codex", "other"] as const;

const DECISION_ACTIVE: Record<Decision, string> = {
  ALLOW: "bg-emerald-600 text-white",
  ASK: "bg-amber-500 text-white",
  DENY: "bg-destructive text-white",
};
const DECISION_BADGE: Record<Decision, string> = {
  ALLOW: "border-transparent bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  ASK: "border-transparent bg-amber-500/15 text-amber-700 dark:text-amber-300",
  DENY: "border-transparent bg-destructive/15 text-destructive",
};

const effectiveKey = (scope: { clientId: string | null; projectId: string | null; environmentId: string | null; connectionId: string | null; action: Action }) =>
  ["effective-decision", scope] as const;

export function AiAccessPage({ project }: { project?: Project }) {
  const { t } = useI18n();
  const body = (
    <div className="grid gap-6">
      <ApprovalsPanel />
      <ClientsSection />
      <PermissionsSection project={project} />
    </div>
  );
  if (project) return body;
  return (
    <div className="p-8">
      <PageHeader title={t("ai.title")} description={t("ai.description")} />
      {body}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Pending approvals (polled).

function ApprovalsPanel() {
  const { t, locale } = useI18n();
  const qc = useQueryClient();
  const approvals = useQuery({
    queryKey: queryKeys.approvals(true),
    queryFn: () => api.listApprovals(true),
    refetchInterval: 2000,
  });
  const resolve = useMutation({
    mutationFn: ({ id, approve }: { id: string; approve: boolean }) => api.resolveApproval(id, approve),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.approvals(true) });
      void qc.invalidateQueries({ queryKey: queryKeys.audit });
    },
  });
  const fmt = (iso: string) => new Date(iso).toLocaleTimeString(locale);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {t("ai.approvals.title")}
          {approvals.data && approvals.data.length > 0 && <Badge>{approvals.data.length}</Badge>}
        </CardTitle>
        <CardDescription>{t("ai.approvals.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        {approvals.error && <ErrorNote error={approvals.error} />}
        {resolve.error && <ErrorNote error={resolve.error} />}
        {approvals.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
        {approvals.data?.length === 0 && <p className="text-sm text-muted-foreground">{t("ai.approvals.empty")}</p>}
        {approvals.data && approvals.data.length > 0 && (
          <ul className="divide-y">
            {approvals.data.map((a) => (
              <li key={a.id} className="flex flex-wrap items-center gap-3 py-3">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <Bot className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium">{a.client_name}</span>
                    <Badge variant="outline">{a.action}</Badge>
                  </div>
                  <p className="mt-1 break-all font-mono text-xs">{a.summary}</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t("ai.approvals.requested", { time: fmt(a.created_at) })} · {t("ai.approvals.expires", { time: fmt(a.expires_at) })}
                  </p>
                </div>
                <div className="flex gap-2">
                  <Button size="sm" disabled={resolve.isPending} onClick={() => resolve.mutate({ id: a.id, approve: true })}>
                    <Check className="h-3.5 w-3.5" /> {t("common.approve")}
                  </Button>
                  <Button size="sm" variant="destructive" disabled={resolve.isPending} onClick={() => resolve.mutate({ id: a.id, approve: false })}>
                    <X className="h-3.5 w-3.5" /> {t("common.deny")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// AI clients.

function ClientsSection() {
  const { t, locale } = useI18n();
  const qc = useQueryClient();
  const clients = useQuery({ queryKey: queryKeys.aiClients, queryFn: api.listAiClients });
  const [name, setName] = useState("");
  const [kind, setKind] = useState<string>(CLIENT_KINDS[0]);

  const invalidate = () => void qc.invalidateQueries({ queryKey: queryKeys.aiClients });
  const register = useMutation({
    mutationFn: () => api.registerAiClient(name.trim(), kind),
    onSuccess: () => {
      setName("");
      invalidate();
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteAiClient(id),
    onSuccess: () => {
      invalidate();
      void qc.invalidateQueries({ queryKey: queryKeys.permissions });
    },
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("ai.clients.title")}</CardTitle>
        <CardDescription>
          {t("ai.clients.description")} <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">envfish mcp --client &lt;name&gt;</code>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form
          className="mb-4 flex flex-wrap items-end gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (name.trim()) register.mutate();
          }}
        >
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="client-name">{t("common.name")}</Label>
            <Input id="client-name" placeholder="claude-code" value={name} onChange={(e) => setName(e.target.value)} className="w-52" />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="client-kind">{t("common.kind")}</Label>
            <Select id="client-kind" value={kind} onChange={(e) => setKind(e.target.value)} className="w-40">
              {CLIENT_KINDS.map((k) => (
                <option key={k} value={k}>
                  {k}
                </option>
              ))}
            </Select>
          </div>
          <Button type="submit" disabled={!name.trim() || register.isPending}>
            {register.isPending ? <GoldfishInline /> : <Plus className="h-4 w-4" />} {t("ai.clients.register")}
          </Button>
        </form>
        {register.error && <ErrorNote error={register.error} />}
        {remove.error && <ErrorNote error={remove.error} />}
        {clients.error && <ErrorNote error={clients.error} />}
        {clients.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
        {clients.data?.length === 0 && <p className="text-sm text-muted-foreground">{t("ai.clients.empty")}</p>}
        {clients.data && clients.data.length > 0 && (
          <table className="w-full text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr className="border-b">
                <th className="py-2 pr-4 font-medium">{t("common.name")}</th>
                <th className="py-2 pr-4 font-medium">{t("common.kind")}</th>
                <th className="py-2 pr-4 font-medium">{t("ai.clients.lastSeen")}</th>
                <th className="py-2 font-medium" />
              </tr>
            </thead>
            <tbody>
              {clients.data.map((c) => (
                <tr key={c.id} className="border-b last:border-0">
                  <td className="py-2 pr-4 font-medium">{c.name}</td>
                  <td className="py-2 pr-4">
                    <Badge variant="secondary">{c.kind}</Badge>
                  </td>
                  <td className="py-2 pr-4 text-xs text-muted-foreground">{c.last_seen_at ? new Date(c.last_seen_at).toLocaleString(locale) : t("ai.clients.never")}</td>
                  <td className="py-2 text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={t("envs.deleteAria", { name: c.name })}
                      onClick={() => {
                        void confirmAsync(t("ai.clients.confirmDelete", { name: c.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
 if (ok) remove.mutate(c.id);
 });
                      }}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </CardContent>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Permission matrix + explicit rules.

function PermissionsSection({ project }: { project?: Project }) {
  const { t } = useI18n();
  const qc = useQueryClient();

  const clients = useQuery({ queryKey: queryKeys.aiClients, queryFn: api.listAiClients });
  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const permissions = useQuery({ queryKey: queryKeys.permissions, queryFn: api.listPermissions });

  const [clientId, setClientId] = useState("");
  const [projectId, setProjectId] = useState(project?.id ?? "");
  const [environmentId, setEnvironmentId] = useState("");
  const [connectionId, setConnectionId] = useState("");

  // Project tab: the project is fixed. Global: default to the first project.
  useEffect(() => {
    if (project) setProjectId(project.id);
    else if (!projectId && projects.data?.[0]) setProjectId(projects.data[0].id);
  }, [project, projects.data, projectId]);

  // Reset narrower scopes when the wider one changes.
  useEffect(() => {
    setEnvironmentId("");
    setConnectionId("");
  }, [projectId]);
  useEffect(() => {
    setConnectionId("");
  }, [environmentId]);

  const envs = useQuery({ queryKey: queryKeys.environments(projectId), queryFn: () => api.listEnvironments(projectId), enabled: !!projectId });
  const conns = useQuery({ queryKey: queryKeys.connections(projectId), queryFn: () => api.listConnections(projectId), enabled: !!projectId });
  const scopedConns = (conns.data ?? []).filter((c) => !environmentId || c.environment_id === environmentId);

  const scope = {
    clientId: clientId || null,
    projectId: projectId || null,
    environmentId: environmentId || null,
    connectionId: connectionId || null,
  };
  const effective = useQueries({
    queries: ACTIONS.map((action) => ({
      queryKey: effectiveKey({ ...scope, action }),
      queryFn: () => api.effectiveDecision({ ...scope, action }),
      enabled: !!projectId,
    })),
  });

  const setPermission = useMutation({
    mutationFn: ({ action, decision }: { action: Action; decision: Decision }) =>
      api.setPermission({
        client_id: scope.clientId,
        project_id: scope.projectId,
        environment_id: scope.environmentId,
        connection_id: scope.connectionId,
        action,
        decision,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.permissions });
      void qc.invalidateQueries({ queryKey: ["effective-decision"] });
    },
  });
  const deletePermission = useMutation({
    mutationFn: (id: string) => api.deletePermission(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.permissions });
      void qc.invalidateQueries({ queryKey: ["effective-decision"] });
    },
  });

  // Name resolution for the rules list: environments and connections across all projects.
  const allEnvs = useQueries({
    queries: (projects.data ?? []).map((p) => ({ queryKey: queryKeys.environments(p.id), queryFn: () => api.listEnvironments(p.id) })),
  });
  const allConns = useQueries({
    queries: (projects.data ?? []).map((p) => ({ queryKey: queryKeys.connections(p.id), queryFn: () => api.listConnections(p.id) })),
  });
  const names = useMemo(() => {
    const m = new Map<string, string>();
    clients.data?.forEach((c) => m.set(c.id, c.name));
    projects.data?.forEach((p) => m.set(p.id, p.name));
    allEnvs.forEach((q) => q.data?.forEach((e) => m.set(e.id, e.name)));
    allConns.forEach((q) => q.data?.forEach((c) => m.set(c.id, c.name)));
    return m;
  }, [clients.data, projects.data, allEnvs, allConns]);
  const nameOf = (id: string | null, anyKey: "ai.scope.anyClient" | "ai.scope.anyProject" | "ai.scope.anyEnvironment" | "ai.scope.anyConnection") =>
    id === null ? <span className="text-muted-foreground">{t(anyKey)}</span> : names.get(id) ?? <span className="font-mono">{id.slice(0, 8)}</span>;

  const decisionLabel: Record<Decision, string> = { ALLOW: t("ai.decision.allow"), ASK: t("ai.decision.ask"), DENY: t("ai.decision.deny") };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("ai.permissions.title")}</CardTitle>
        <CardDescription>{t("ai.permissions.description")}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5">
        {clients.error && <ErrorNote error={clients.error} />}
        {projects.error && <ErrorNote error={projects.error} />}
        {permissions.error && <ErrorNote error={permissions.error} />}
        {setPermission.error && <ErrorNote error={setPermission.error} />}
        {deletePermission.error && <ErrorNote error={deletePermission.error} />}

        {/* Scope selectors */}
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="perm-client">{t("common.client")}</Label>
            <Select id="perm-client" value={clientId} onChange={(e) => setClientId(e.target.value)} className="w-44">
              <option value="">{t("ai.scope.anyClient")}</option>
              {clients.data?.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="perm-project">{t("common.project")}</Label>
            <Select id="perm-project" value={projectId} onChange={(e) => setProjectId(e.target.value)} className="w-44" disabled={!!project}>
              {projects.data?.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="perm-env">{t("common.environment")}</Label>
            <Select id="perm-env" value={environmentId} onChange={(e) => setEnvironmentId(e.target.value)} className="w-44" disabled={!projectId}>
              <option value="">{t("ai.scope.anyEnvironment")}</option>
              {envs.data?.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.name}
                </option>
              ))}
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="perm-conn">{t("common.connection")}</Label>
            <Select id="perm-conn" value={connectionId} onChange={(e) => setConnectionId(e.target.value)} className="w-44" disabled={!projectId}>
              <option value="">{t("ai.scope.anyConnection")}</option>
              {scopedConns.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          </div>
        </div>

        {/* Matrix */}
        {!projectId ? (
          <p className="text-sm text-muted-foreground">{t("ai.permissions.noProject")}</p>
        ) : (
          <table className="w-full max-w-xl text-sm">
            <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr className="border-b">
                <th className="py-2 pr-4 font-medium">{t("common.action")}</th>
                <th className="py-2 pr-4 font-medium">{t("common.decision")}</th>
              </tr>
            </thead>
            <tbody>
              {ACTIONS.map((action, i) => {
                const q = effective[i];
                return (
                  <tr key={action} className="border-b last:border-0">
                    <td className="py-2 pr-4">
                      <Badge variant="outline">{action}</Badge>
                    </td>
                    <td className="py-2 pr-4">
                      {q?.isLoading ? (
                        <GoldfishInline />
                      ) : (
                        <Segmented
                          value={q?.data ?? null}
                          onChange={(decision) => setPermission.mutate({ action, decision })}
                          ariaLabel={`${action} ${t("common.decision")}`}
                          options={DECISIONS.map((d) => ({ value: d, label: decisionLabel[d], activeClass: DECISION_ACTIVE[d], disabled: setPermission.isPending }))}
                        />
                      )}
                      {q?.error && <ErrorNote error={q.error} />}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}

        {/* Legend */}
        <div className="rounded-md bg-muted/60 px-3 py-2 text-xs text-muted-foreground">
          <p className="mb-1 font-medium text-foreground">{t("ai.legend.title")}</p>
          <p>{t("ai.legend.development")}</p>
          <p>{t("ai.legend.production")}</p>
          <p className="mt-1">{t("ai.legend.note")}</p>
        </div>

        {/* Explicit rules */}
        <div>
          <h4 className="mb-2 text-sm font-medium">{t("ai.rules.title")}</h4>
          {permissions.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
          {permissions.data?.length === 0 && <p className="text-sm text-muted-foreground">{t("ai.rules.empty")}</p>}
          {permissions.data && permissions.data.length > 0 && (
            <table className="w-full text-sm">
              <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
                <tr className="border-b">
                  <th className="py-2 pr-4 font-medium">{t("common.client")}</th>
                  <th className="py-2 pr-4 font-medium">{t("common.project")}</th>
                  <th className="py-2 pr-4 font-medium">{t("common.environment")}</th>
                  <th className="py-2 pr-4 font-medium">{t("common.connection")}</th>
                  <th className="py-2 pr-4 font-medium">{t("common.action")}</th>
                  <th className="py-2 pr-4 font-medium">{t("common.decision")}</th>
                  <th className="py-2 font-medium" />
                </tr>
              </thead>
              <tbody>
                {permissions.data.map((p: Permission) => (
                  <tr key={p.id} className="border-b last:border-0">
                    <td className="py-2 pr-4">{nameOf(p.client_id, "ai.scope.anyClient")}</td>
                    <td className="py-2 pr-4">{nameOf(p.project_id, "ai.scope.anyProject")}</td>
                    <td className="py-2 pr-4">{nameOf(p.environment_id, "ai.scope.anyEnvironment")}</td>
                    <td className="py-2 pr-4">{nameOf(p.connection_id, "ai.scope.anyConnection")}</td>
                    <td className="py-2 pr-4">
                      <Badge variant="outline">{p.action}</Badge>
                    </td>
                    <td className="py-2 pr-4">
                      <Badge className={cn(DECISION_BADGE[p.decision])}>{p.decision}</Badge>
                    </td>
                    <td className="py-2 text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t("common.delete")}
                        onClick={() => {
                          void confirmAsync(t("ai.rules.confirmDelete"), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
 if (ok) deletePermission.mutate(p.id);
 });
                        }}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
