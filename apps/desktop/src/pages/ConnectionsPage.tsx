import { useEffect, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { ArrowRight, KeyRound, Plus, Trash2 } from "lucide-react";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, GoldfishInline, GoldfishLoader, Input, Label } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { CONNECTION_KINDS, type Connection, type ConnectionKind, type Environment, type Project } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { Select } from "../components/Select";
import { useI18n, type MessageKey } from "../lib/i18n";
import { confirmAsync } from "../lib/confirm";

const KINDS = CONNECTION_KINDS;
const KIND_LABEL_KEY: Record<ConnectionKind, MessageKey> = {
  generic_http: "connections.kind.generic_http",
  openai: "connections.kind.openai",
  supabase: "connections.kind.supabase",
  cloudflare: "connections.kind.cloudflare",
  vercel: "connections.kind.vercel",
  github: "connections.kind.github",
  aws: "connections.kind.aws",
};
/** Shown as the placeholder; kinds with a well-known API host may leave the URL blank. */
const DEFAULT_BASE_URL: Record<ConnectionKind, string> = {
  generic_http: "https://api.example.com",
  openai: "https://api.openai.com/v1",
  supabase: "https://my-app.supabase.co",
  cloudflare: "https://api.cloudflare.com/client/v4",
  vercel: "https://api.vercel.com",
  github: "https://api.github.com",
  aws: "https://sqs.ap-northeast-1.amazonaws.com",
};
const BASE_URL_OPTIONAL: Record<ConnectionKind, boolean> = {
  generic_http: false,
  openai: true,
  supabase: false,
  cloudflare: true,
  vercel: true,
  github: true,
  aws: false,
};
const DEFAULT_AUTH_STYLE: Record<ConnectionKind, string> = {
  generic_http: "bearer",
  openai: "bearer",
  supabase: "supabase",
  cloudflare: "bearer",
  vercel: "bearer",
  github: "bearer",
  aws: "sigv4",
};

/** `metadata` is `unknown` on the wire; read string fields defensively. */
function metadataString(metadata: unknown, key: string): string | null {
  if (typeof metadata !== "object" || metadata === null || Array.isArray(metadata)) return null;
  const v = (metadata as Record<string, unknown>)[key];
  return typeof v === "string" && v.length > 0 ? v : null;
}
function awsScope(c: Connection): string | null {
  if (c.kind !== "aws") return null;
  const region = metadataString(c.metadata, "region");
  const service = metadataString(c.metadata, "service");
  return region || service ? `${region ?? "?"}/${service ?? "?"}` : null;
}

export function ConnectionsPage({ project }: { project?: Project }) {
  return project ? <ProjectConnections project={project} /> : <AllConnections />;
}

// ---------------------------------------------------------------------------
// Global route: every project, its connections, and a link to the project tab.

function AllConnections() {
  const { t } = useI18n();
  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const perProject = useQueries({
    queries: (projects.data ?? []).map((p) => ({
      queryKey: queryKeys.connections(p.id),
      queryFn: () => api.listConnections(p.id),
    })),
  });
  const loading = projects.isLoading || perProject.some((q) => q.isLoading);
  const total = perProject.reduce((n, q) => n + (q.data?.length ?? 0), 0);

  return (
    <div className="p-8">
      <PageHeader title={t("connections.title")} description={t("connections.description")} />
      {projects.error && <ErrorNote error={projects.error} />}
      {perProject.map((q, i) => q.error && <ErrorNote key={projects.data?.[i]?.id ?? i} error={q.error} />)}
      {loading && <GoldfishLoader label={t("common.loading")} className="py-16" />}

      {!loading && projects.data?.length === 0 && (
        <EmptyState text={t("connections.noProjects")}>
          <Button asChild variant="secondary" size="sm">
            <Link to="/projects">{t("nav.projects")}</Link>
          </Button>
        </EmptyState>
      )}
      {!loading && projects.data && projects.data.length > 0 && total === 0 && <EmptyState text={t("connections.emptyAll")} />}

      <div className="grid gap-4">
        {projects.data?.map((p, i) => {
          const conns = perProject[i]?.data ?? [];
          if (conns.length === 0) return null;
          return (
            <Card key={p.id}>
              <CardHeader className="flex-row items-center justify-between space-y-0">
                <CardTitle>{p.name}</CardTitle>
                <Button asChild variant="ghost" size="sm">
                  <Link to={`/projects/${p.id}/connections`}>
                    {t("connections.manage")} <ArrowRight className="h-3.5 w-3.5" />
                  </Link>
                </Button>
              </CardHeader>
              <CardContent>
                <ConnectionTable connections={conns} />
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Project tab: connections per environment + "Add connection" form.

function ProjectConnections({ project }: { project: Project }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const envs = useQuery({ queryKey: queryKeys.environments(project.id), queryFn: () => api.listEnvironments(project.id) });
  const conns = useQuery({ queryKey: queryKeys.connections(project.id), queryFn: () => api.listConnections(project.id) });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteConnection(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: queryKeys.connections(project.id) }),
  });

  if (envs.isLoading || conns.isLoading) return <GoldfishLoader label={t("common.loading")} className="py-16" />;
  if (!envs.data?.length) return <p className="py-8 text-sm text-muted-foreground">{t("vars.createEnvFirst")}</p>;

  return (
    <div>
      <AddConnectionForm project={project} environments={envs.data} />
      {envs.error && <ErrorNote error={envs.error} />}
      {conns.error && <ErrorNote error={conns.error} />}
      {remove.error && <ErrorNote error={remove.error} />}

      {conns.data?.length === 0 && <EmptyState text={t("connections.empty")} />}

      <div className="grid gap-4">
        {envs.data.map((env) => {
          const list = conns.data?.filter((c) => c.environment_id === env.id) ?? [];
          if (list.length === 0) return null;
          return (
            <Card key={env.id}>
              <CardHeader>
                <CardTitle className="text-sm">{env.name}</CardTitle>
              </CardHeader>
              <CardContent>
                <ConnectionTable
                  connections={list}
                  onDelete={(c) => {
                    void confirmAsync(t("connections.confirmDelete", { name: c.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
 if (ok) remove.mutate(c.id);
 });
                  }}
                />
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

function AddConnectionForm({ project, environments }: { project: Project; environments: Environment[] }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const [environmentId, setEnvironmentId] = useState(environments[0]?.id ?? "");
  const [kind, setKind] = useState<ConnectionKind>("generic_http");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [authSecret, setAuthSecret] = useState("");
  const [authStyle, setAuthStyle] = useState("");
  const [awsRegion, setAwsRegion] = useState("");
  const [awsService, setAwsService] = useState("");
  const [awsAccessKeyId, setAwsAccessKeyId] = useState("");
  const isAws = kind === "aws";

  // Keep the environment selection valid if the list changes underneath us.
  useEffect(() => {
    if (!environments.some((e) => e.id === environmentId)) setEnvironmentId(environments[0]?.id ?? "");
  }, [environments, environmentId]);

  const vars = useQuery({
    queryKey: queryKeys.variables(environmentId),
    queryFn: () => api.listVariables(environmentId),
    enabled: !!environmentId,
  });
  const secrets = vars.data?.filter((v) => v.kind === "SECRET") ?? [];

  const create = useMutation({
    mutationFn: () =>
      api.createConnection({
        environment_id: environmentId,
        kind,
        name: name.trim(),
        base_url: baseUrl.trim() || null,
        auth_secret: authSecret || null,
        auth_style: authStyle.trim() || null,
        metadata: isAws
          ? { region: awsRegion.trim(), service: awsService.trim(), access_key_id_secret: awsAccessKeyId }
          : null,
      }),
    onSuccess: () => {
      setName("");
      setBaseUrl("");
      setAuthSecret("");
      setAuthStyle("");
      setAwsRegion("");
      setAwsService("");
      setAwsAccessKeyId("");
      void qc.invalidateQueries({ queryKey: queryKeys.connections(project.id) });
    },
  });

  const baseUrlRequired = !BASE_URL_OPTIONAL[kind];
  const awsComplete = !isAws || (!!awsRegion.trim() && !!awsService.trim() && !!awsAccessKeyId && !!authSecret);
  const canSubmit = !!environmentId && !!name.trim() && (!baseUrlRequired || !!baseUrl.trim()) && awsComplete && !create.isPending;

  return (
    <>
      <form
        className="mb-6 flex flex-wrap items-end gap-3 rounded-lg border bg-card p-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (canSubmit) create.mutate();
        }}
      >
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="conn-env">{t("common.environment")}</Label>
          <Select id="conn-env" value={environmentId} onChange={(e) => setEnvironmentId(e.target.value)} className="w-44">
            {environments.map((env) => (
              <option key={env.id} value={env.id}>
                {env.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="conn-kind">{t("common.kind")}</Label>
          <Select id="conn-kind" value={kind} onChange={(e) => setKind(e.target.value as ConnectionKind)} className="w-40">
            {KINDS.map((k) => (
              <option key={k} value={k}>
                {t(KIND_LABEL_KEY[k])}
              </option>
            ))}
          </Select>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="conn-name">{t("common.name")}</Label>
          <Input id="conn-name" placeholder={kind === "generic_http" ? "backend-api" : kind} value={name} onChange={(e) => setName(e.target.value)} className="w-44" />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="conn-url">
            {t("connections.baseUrl")}
            {!baseUrlRequired && <span className="ml-1 font-normal text-muted-foreground">({t("connections.optional")})</span>}
          </Label>
          <Input id="conn-url" placeholder={DEFAULT_BASE_URL[kind]} value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} className="w-72 font-mono" />
        </div>
        {isAws && (
          <>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="conn-aws-region">{t("connections.aws.region")}</Label>
              <Input id="conn-aws-region" placeholder="ap-northeast-1" value={awsRegion} onChange={(e) => setAwsRegion(e.target.value)} className="w-40 font-mono" />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="conn-aws-service">{t("connections.aws.service")}</Label>
              <Input id="conn-aws-service" placeholder="sqs" value={awsService} onChange={(e) => setAwsService(e.target.value)} className="w-32 font-mono" />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="conn-aws-access-key">{t("connections.aws.accessKeyId")}</Label>
              <Select
                id="conn-aws-access-key"
                value={awsAccessKeyId}
                onChange={(e) => setAwsAccessKeyId(e.target.value)}
                className="w-52 font-mono"
                disabled={vars.isLoading}
              >
                <option value="">{t("connections.noCredential")}</option>
                {secrets.map((v) => (
                  <option key={v.id} value={v.name}>
                    {v.name}
                  </option>
                ))}
              </Select>
            </div>
          </>
        )}
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="conn-secret">{isAws ? t("connections.aws.secretAccessKey") : t("connections.credential")}</Label>
          <Select id="conn-secret" value={authSecret} onChange={(e) => setAuthSecret(e.target.value)} className="w-52 font-mono" disabled={vars.isLoading}>
            <option value="">{t("connections.noCredential")}</option>
            {secrets.map((v) => (
              <option key={v.id} value={v.name}>
                {v.name}
              </option>
            ))}
          </Select>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="conn-auth">
            {t("connections.authStyle")} <span className="ml-1 font-normal text-muted-foreground">({t("connections.authStyleAuto", { style: DEFAULT_AUTH_STYLE[kind] })})</span>
          </Label>
          <Input id="conn-auth" placeholder="bearer | header:X-Api-Key | query:key | none" value={authStyle} onChange={(e) => setAuthStyle(e.target.value)} className="w-72 font-mono" />
        </div>
        <Button type="submit" disabled={!canSubmit}>
          {create.isPending ? <GoldfishInline /> : <Plus className="h-4 w-4" />} {t("connections.add")}
        </Button>
        <p className="basis-full text-xs text-muted-foreground">
          <KeyRound className="mr-1 inline h-3 w-3" />
          {secrets.length === 0 && vars.data ? t("connections.noSecretsHint") : t("connections.credentialHint")}
        </p>
      </form>
      {create.error && <ErrorNote error={create.error} />}
      {vars.error && <ErrorNote error={vars.error} />}
    </>
  );
}

// ---------------------------------------------------------------------------

function ConnectionTable({ connections, onDelete }: { connections: Connection[]; onDelete?: (c: Connection) => void }) {
  const { t } = useI18n();
  return (
    <table className="w-full text-sm">
      <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
        <tr className="border-b">
          <th className="py-2 pr-4 font-medium">{t("common.kind")}</th>
          <th className="py-2 pr-4 font-medium">{t("common.name")}</th>
          <th className="py-2 pr-4 font-medium">{t("connections.baseUrl")}</th>
          <th className="py-2 pr-4 font-medium">{t("connections.credential")}</th>
          <th className="py-2 pr-4 font-medium">{t("connections.authStyle")}</th>
          {onDelete && <th className="py-2 font-medium" />}
        </tr>
      </thead>
      <tbody>
        {connections.map((c) => (
          <tr key={c.id} className="border-b last:border-0">
            <td className="py-2 pr-4">
              <Badge variant="secondary">{t(KIND_LABEL_KEY[c.kind])}</Badge>
            </td>
            <td className="py-2 pr-4 font-medium">{c.name}</td>
            <td className="py-2 pr-4 font-mono text-xs text-muted-foreground">
              {c.base_url}
              {awsScope(c) && <span className="ml-2 rounded bg-muted px-1.5 py-0.5 text-[11px] text-foreground/80">{awsScope(c)}</span>}
            </td>
            <td className="py-2 pr-4 font-mono text-xs">
              {c.auth_secret ? (
                <span className="inline-flex items-center gap-1">
                  <KeyRound className="h-3 w-3 text-muted-foreground" /> {c.auth_secret}
                </span>
              ) : (
                <span className="text-muted-foreground">—</span>
              )}
            </td>
            <td className="py-2 pr-4 font-mono text-xs text-muted-foreground">{c.auth_style}</td>
            {onDelete && (
              <td className="py-2 text-right">
                <Button variant="ghost" size="icon" aria-label={t("envs.deleteAria", { name: c.name })} onClick={() => onDelete(c)}>
                  <Trash2 className="h-4 w-4" />
                </Button>
              </td>
            )}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
