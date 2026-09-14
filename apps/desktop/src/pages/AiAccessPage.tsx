import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Plus, Trash2, X } from "lucide-react";
import { Badge, Button, cn, GoldfishInline, GoldfishLoader, Input } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { ACTIONS, DECISIONS, type Action, type Decision, type Permission } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { KindDot } from "../components/KindDot";
import { SectionLabel } from "../components/SectionLabel";
import { Select } from "../components/Select";
import { RowActions, Table, Td, Th, Tr } from "../components/Table";
import { useI18n } from "../lib/i18n";
import { useAppContext } from "../lib/context";
import { confirmAsync } from "../lib/confirm";

const CLIENT_KINDS = ["claude_code", "codex", "other"] as const;

const DECISION_ACTIVE: Record<Decision, string> = {
  ALLOW: "bg-emerald-600 text-white border-emerald-600",
  ASK: "bg-amber-500 text-white border-amber-500",
  DENY: "bg-destructive text-white border-destructive",
};
const DECISION_BADGE: Record<Decision, string> = {
  ALLOW: "border-transparent bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  ASK: "border-transparent bg-amber-500/15 text-amber-700 dark:text-amber-300",
  DENY: "border-transparent bg-destructive/15 text-destructive",
};

const effectiveKey = (scope: { clientId: string | null; projectId: string | null; environmentId: string | null; connectionId: string | null; action: Action }) =>
  ["effective-decision", scope] as const;

export function AiAccessPage() {
  const { t } = useI18n();
  return (
    <div>
      <PageHeader title={t("ai.title")} context />
      <div className="flex flex-col gap-10">
        <ApprovalsPanel />
        <ClientsSection />
        <PermissionsSection />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Pending approvals (polled). Rendered only while something is waiting.

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
  const fmt = (iso: string) => new Date(iso).toLocaleTimeString(locale, { hour12: false });

  if (!approvals.error && !resolve.error && !approvals.data?.length) return null;

  return (
    <section>
      <SectionLabel right={approvals.data && <span className="font-mono text-[11px] text-amber-600 dark:text-amber-400">{approvals.data.length}</span>}>{t("ai.approvals.title")}</SectionLabel>
      {approvals.error && <ErrorNote error={approvals.error} />}
      {resolve.error && <ErrorNote error={resolve.error} />}
      {approvals.data && approvals.data.length > 0 && (
        <ul className="rounded-md border border-amber-500/40 bg-amber-500/5">
          {approvals.data.map((a) => (
            <li key={a.id} className="flex items-center gap-4 border-b border-amber-500/20 px-3 py-2 last:border-0">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 text-sm">
                  <span className="font-medium">{a.client_name}</span>
                  <span className="font-mono text-[11px] text-muted-foreground">{a.action}</span>
                </div>
                <p className="truncate font-mono text-xs" title={a.summary}>
                  {a.summary}
                </p>
                <p className="font-mono text-[11px] text-muted-foreground">
                  {t("ai.approvals.requested", { time: fmt(a.created_at) })} · {t("ai.approvals.expires", { time: fmt(a.expires_at) })}
                </p>
              </div>
              <div className="flex shrink-0 gap-1.5">
                <Button size="sm" disabled={resolve.isPending} onClick={() => resolve.mutate({ id: a.id, approve: true })}>
                  <Check className="h-3.5 w-3.5" /> {t("common.approve")}
                </Button>
                <Button size="sm" variant="outline" className="text-destructive hover:text-destructive" disabled={resolve.isPending} onClick={() => resolve.mutate({ id: a.id, approve: false })}>
                  <X className="h-3.5 w-3.5" /> {t("common.deny")}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// AI clients: table with an inline register row.

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
    <section>
      <SectionLabel>{t("ai.clients.title")}</SectionLabel>
      {register.error && <ErrorNote error={register.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {clients.error && <ErrorNote error={clients.error} />}
      <Table>
        <thead>
          <tr>
            <Th className="w-[36%]">{t("common.name")}</Th>
            <Th className="w-40">{t("common.kind")}</Th>
            <Th>{t("ai.clients.lastSeen")}</Th>
            <Th className="w-10" />
          </tr>
        </thead>
        <tbody>
          <tr className="h-10 border-b border-border/60">
            <Td>
              <Input
                aria-label={t("common.name")}
                placeholder="claude-code"
                value={name}
                onChange={(e) => setName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && name.trim()) register.mutate();
                }}
                className="h-7 border-transparent bg-transparent px-1 font-mono text-xs hover:border-border focus-visible:border-primary"
              />
            </Td>
            <Td>
              <Select aria-label={t("common.kind")} value={kind} onChange={(e) => setKind(e.target.value)} className="h-7 border-transparent bg-transparent px-1 font-mono text-xs hover:border-border">
                {CLIENT_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </Select>
            </Td>
            <Td className="font-mono text-[11px] text-muted-foreground">{t("ai.clients.register")}</Td>
            <Td>
              <div className="flex justify-end">
                <Button type="button" size="icon-sm" variant="ghost" aria-label={t("ai.clients.register")} disabled={!name.trim() || register.isPending} onClick={() => register.mutate()}>
                  {register.isPending ? <GoldfishInline size={1} /> : <Plus className="h-4 w-4" />}
                </Button>
              </div>
            </Td>
          </tr>
          {clients.data?.map((c) => (
            <Tr key={c.id}>
              <Td className="font-medium">{c.name}</Td>
              <Td>
                <KindDot tone={c.kind === "claude_code" ? "teal" : c.kind === "codex" ? "violet" : "slate"} label={c.kind} />
              </Td>
              <Td className="font-mono text-[11px] text-muted-foreground">{c.last_seen_at ? new Date(c.last_seen_at).toLocaleString(locale) : t("ai.clients.never")}</Td>
              <Td>
                <RowActions>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={t("envs.deleteAria", { name: c.name })}
                    onClick={() => {
                      void confirmAsync(t("ai.clients.confirmDelete", { name: c.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
                        if (ok) remove.mutate(c.id);
                      });
                    }}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </RowActions>
              </Td>
            </Tr>
          ))}
        </tbody>
      </Table>
      {clients.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
      {clients.data?.length === 0 && <p className="py-4 text-center text-sm text-muted-foreground">{t("ai.clients.empty")}</p>}
      <p className="mt-2 font-mono text-[11px] text-muted-foreground">
        {t("ai.clients.description")} <code className="rounded bg-muted px-1 py-0.5 text-foreground">envfish mcp --client &lt;name&gt;</code>
      </p>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Permission matrix (3 actions × 3 decisions) + explicit rules.

function PermissionsSection() {
  const { t } = useI18n();
  const qc = useQueryClient();
  const ctx = useAppContext();
  const projectId = ctx.projectId ?? "";

  const clients = useQuery({ queryKey: queryKeys.aiClients, queryFn: api.listAiClients });
  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const permissions = useQuery({ queryKey: queryKeys.permissions, queryFn: api.listPermissions });

  const [clientId, setClientId] = useState("");
  const [environmentId, setEnvironmentId] = useState(ctx.environmentId ?? "");
  const [connectionId, setConnectionId] = useState("");

  // Follow the global switcher; narrower scopes reset when the wider one changes.
  useEffect(() => {
    setEnvironmentId(ctx.environmentId ?? "");
    setConnectionId("");
  }, [projectId, ctx.environmentId]);
  useEffect(() => {
    setConnectionId("");
  }, [environmentId]);

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

  const invalidateRules = () => {
    void qc.invalidateQueries({ queryKey: queryKeys.permissions });
    void qc.invalidateQueries({ queryKey: ["effective-decision"] });
  };
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
    onSuccess: invalidateRules,
  });
  const deletePermission = useMutation({ mutationFn: (id: string) => api.deletePermission(id), onSuccess: invalidateRules });

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
  const scopeSelect = "h-7 w-auto min-w-[140px] font-mono text-xs";

  return (
    <section>
      <SectionLabel>{t("ai.permissions.title")}</SectionLabel>
      {clients.error && <ErrorNote error={clients.error} />}
      {projects.error && <ErrorNote error={projects.error} />}
      {permissions.error && <ErrorNote error={permissions.error} />}
      {setPermission.error && <ErrorNote error={setPermission.error} />}
      {deletePermission.error && <ErrorNote error={deletePermission.error} />}

      {!projectId ? (
        <p className="py-4 text-sm text-muted-foreground">{t("guide.needProject")}</p>
      ) : (
        <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          {/* Scope */}
          <div className="flex flex-col gap-2">
            <ScopeRow label={t("common.client")} htmlFor="perm-client">
              <Select id="perm-client" value={clientId} onChange={(e) => setClientId(e.target.value)} className={scopeSelect}>
                <option value="">{t("ai.scope.anyClient")}</option>
                {clients.data?.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </Select>
            </ScopeRow>
            <ScopeRow label={t("common.project")} htmlFor="perm-project">
              <span id="perm-project" className="font-mono text-xs">
                {ctx.project?.name}
              </span>
            </ScopeRow>
            <ScopeRow label={t("common.environment")} htmlFor="perm-env">
              <Select id="perm-env" value={environmentId} onChange={(e) => setEnvironmentId(e.target.value)} className={scopeSelect}>
                <option value="">{t("ai.scope.anyEnvironment")}</option>
                {ctx.environments.map((e) => (
                  <option key={e.id} value={e.id}>
                    {e.name}
                  </option>
                ))}
              </Select>
            </ScopeRow>
            <ScopeRow label={t("common.connection")} htmlFor="perm-conn">
              <Select id="perm-conn" value={connectionId} onChange={(e) => setConnectionId(e.target.value)} className={scopeSelect}>
                <option value="">{t("ai.scope.anyConnection")}</option>
                {scopedConns.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </Select>
            </ScopeRow>
          </div>

          {/* 3×3 matrix */}
          <div>
            <div className="grid grid-cols-[72px_repeat(3,minmax(0,1fr))] gap-1 text-center">
              <span />
              {DECISIONS.map((d) => (
                <span key={d} className="font-mono text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
                  {decisionLabel[d]}
                </span>
              ))}
              {ACTIONS.map((action, i) => {
                const q = effective[i];
                return (
                  <div key={action} className="contents" role="group" aria-label={`${action} ${t("common.decision")}`}>
                    <span className="flex items-center font-mono text-xs">{action}</span>
                    {DECISIONS.map((d) => {
                      const active = q?.data === d;
                      return (
                        <button
                          key={d}
                          type="button"
                          aria-pressed={active}
                          aria-label={`${action} ${decisionLabel[d]}`}
                          disabled={setPermission.isPending || q?.isLoading}
                          onClick={() => setPermission.mutate({ action, decision: d })}
                          className={cn(
                            "flex h-9 items-center justify-center rounded-md border text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60",
                            active ? DECISION_ACTIVE[d] : "border-border/60 text-muted-foreground hover:bg-accent/60 hover:text-foreground",
                          )}
                        >
                          {q?.isLoading ? <GoldfishInline size={1} /> : active ? <Check className="h-3.5 w-3.5" /> : null}
                        </button>
                      );
                    })}
                    {q?.error && (
                      <div className="col-span-4">
                        <ErrorNote error={q.error} />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
            <p className="mt-2 font-mono text-[11px] text-muted-foreground">{t("ai.permissions.effective")}</p>
          </div>
        </div>
      )}

      {/* Legend */}
      <div className="mt-6 font-mono text-[11px] leading-5 text-muted-foreground">
        <p className="text-foreground">{t("ai.legend.title")}</p>
        <p>{t("ai.legend.development")}</p>
        <p>{t("ai.legend.production")}</p>
        <p>{t("ai.legend.note")}</p>
      </div>

      {/* Explicit rules */}
      <div className="mt-8">
        <SectionLabel>{t("ai.rules.title")}</SectionLabel>
        {permissions.isLoading && <GoldfishLoader label={t("common.loading")} className="py-6" />}
        {permissions.data?.length === 0 && <p className="py-2 text-sm text-muted-foreground">{t("ai.rules.empty")}</p>}
        {permissions.data && permissions.data.length > 0 && (
          <Table>
            <thead>
              <tr>
                <Th>{t("common.client")}</Th>
                <Th>{t("common.project")}</Th>
                <Th>{t("common.environment")}</Th>
                <Th>{t("common.connection")}</Th>
                <Th>{t("common.action")}</Th>
                <Th>{t("common.decision")}</Th>
                <Th className="w-10" />
              </tr>
            </thead>
            <tbody>
              {permissions.data.map((p: Permission) => (
                <Tr key={p.id}>
                  <Td className="text-xs">{nameOf(p.client_id, "ai.scope.anyClient")}</Td>
                  <Td className="text-xs">{nameOf(p.project_id, "ai.scope.anyProject")}</Td>
                  <Td className="text-xs">{nameOf(p.environment_id, "ai.scope.anyEnvironment")}</Td>
                  <Td className="text-xs">{nameOf(p.connection_id, "ai.scope.anyConnection")}</Td>
                  <Td className="font-mono text-[11px]">{p.action}</Td>
                  <Td>
                    <Badge className={cn(DECISION_BADGE[p.decision])}>{p.decision}</Badge>
                  </Td>
                  <Td>
                    <RowActions>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={t("common.delete")}
                        onClick={() => {
                          void confirmAsync(t("ai.rules.confirmDelete"), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
                            if (ok) deletePermission.mutate(p.id);
                          });
                        }}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </RowActions>
                  </Td>
                </Tr>
              ))}
            </tbody>
          </Table>
        )}
      </div>
    </section>
  );
}

function ScopeRow({ label, htmlFor, children }: { label: string; htmlFor: string; children: React.ReactNode }) {
  return (
    <div className="flex h-9 items-center justify-between gap-4 border-b border-border/60 last:border-0">
      <label htmlFor={htmlFor} className="text-xs text-muted-foreground">
        {label}
      </label>
      {children}
    </div>
  );
}
