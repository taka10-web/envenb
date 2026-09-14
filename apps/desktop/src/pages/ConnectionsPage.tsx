import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Plus, Trash2 } from "lucide-react";
import { Button, GoldfishInline, GoldfishLoader, Input } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { CONNECTION_KINDS, type Connection, type ConnectionKind } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { Field } from "../components/Field";
import { KindDot, type KindTone } from "../components/KindDot";
import { WithEnvironment } from "../components/NeedsContext";
import { SectionLabel } from "../components/SectionLabel";
import { Select } from "../components/Select";
import { Sheet } from "../components/Sheet";
import { RowActions, Table, Td, Th, Tr } from "../components/Table";
import { useI18n, type MessageKey } from "../lib/i18n";
import { useAppContext } from "../lib/context";
import { confirmAsync } from "../lib/confirm";

const KIND_LABEL_KEY: Record<ConnectionKind, MessageKey> = {
  generic_http: "connections.kind.generic_http",
  openai: "connections.kind.openai",
  supabase: "connections.kind.supabase",
  cloudflare: "connections.kind.cloudflare",
  vercel: "connections.kind.vercel",
  github: "connections.kind.github",
  aws: "connections.kind.aws",
};
const KIND_TONE: Record<ConnectionKind, KindTone> = {
  generic_http: "slate",
  openai: "emerald",
  supabase: "teal",
  cloudflare: "orange",
  vercel: "violet",
  github: "sky",
  aws: "amber",
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

export function ConnectionsPage() {
  const { t } = useI18n();
  const { isLoading, projectId, environmentId } = useAppContext();
  const ready = !isLoading && !!projectId && !!environmentId;
  return (
    <div>
      {!ready && <PageHeader title={t("connections.title")} context />}
      <WithEnvironment>{({ projectId, environmentId }) => <EnvironmentConnections key={environmentId} projectId={projectId} environmentId={environmentId} />}</WithEnvironment>
    </div>
  );
}

function EnvironmentConnections({ projectId, environmentId }: { projectId: string; environmentId: string }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const conns = useQuery({ queryKey: queryKeys.connections(projectId), queryFn: () => api.listConnections(projectId) });
  const [open, setOpen] = useState(false);

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteConnection(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: queryKeys.connections(projectId) }),
  });

  const list = conns.data?.filter((c) => c.environment_id === environmentId) ?? [];
  const byKind = CONNECTION_KINDS.map((k) => [k, list.filter((c) => c.kind === k)] as const).filter(([, cs]) => cs.length > 0);

  const addButton = (
    <Button size="sm" onClick={() => setOpen(true)}>
      <Plus className="h-3.5 w-3.5" /> {t("connections.add")}
    </Button>
  );

  return (
    <div>
      <PageHeader title={t("connections.title")} context actions={addButton} />
      {conns.error && <ErrorNote error={conns.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {conns.isLoading && <GoldfishLoader label={t("common.loading")} className="py-16" />}
      {conns.data && list.length === 0 && <EmptyState text={t("connections.empty")}>{addButton}</EmptyState>}

      <div className="flex flex-col gap-6">
        {byKind.map(([kind, cs]) => (
          <section key={kind}>
            <SectionLabel>
              <KindDot tone={KIND_TONE[kind]} label={t(KIND_LABEL_KEY[kind])} className="uppercase" />
            </SectionLabel>
            <Table>
              <thead className="sr-only">
                <tr>
                  <Th>{t("common.name")}</Th>
                  <Th>{t("connections.baseUrl")}</Th>
                  <Th>{t("connections.credential")}</Th>
                  <Th>{t("connections.authStyle")}</Th>
                  <Th />
                </tr>
              </thead>
              <tbody>
                {cs.map((c) => (
                  <Tr key={c.id}>
                    <Td className="w-[22%] font-medium">{c.name}</Td>
                    <Td className="max-w-0 truncate font-mono text-xs text-muted-foreground" title={c.base_url}>
                      {c.base_url}
                      {awsScope(c) && <span className="ml-2 text-foreground/70">{awsScope(c)}</span>}
                    </Td>
                    <Td className="w-[22%] whitespace-nowrap font-mono text-xs">
                      {c.auth_secret ? (
                        <span className="inline-flex items-center gap-1">
                          <KeyRound className="h-3 w-3 text-muted-foreground" /> {c.auth_secret}
                        </span>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </Td>
                    <Td className="w-28 font-mono text-[11px] text-muted-foreground">{c.auth_style}</Td>
                    <Td className="w-10">
                      <RowActions>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={t("envs.deleteAria", { name: c.name })}
                          onClick={() => {
                            void confirmAsync(t("connections.confirmDelete", { name: c.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
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
          </section>
        ))}
      </div>

      <Sheet open={open} title={t("connections.add")} onClose={() => setOpen(false)}>
        <AddConnectionForm projectId={projectId} environmentId={environmentId} onDone={() => setOpen(false)} />
      </Sheet>
    </div>
  );
}

function AddConnectionForm({ projectId, environmentId, onDone }: { projectId: string; environmentId: string; onDone: () => void }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const [kind, setKind] = useState<ConnectionKind>("generic_http");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [authSecret, setAuthSecret] = useState("");
  const [authStyle, setAuthStyle] = useState("");
  const [awsRegion, setAwsRegion] = useState("");
  const [awsService, setAwsService] = useState("");
  const [awsAccessKeyId, setAwsAccessKeyId] = useState("");
  const isAws = kind === "aws";

  const vars = useQuery({ queryKey: queryKeys.variables(environmentId), queryFn: () => api.listVariables(environmentId) });
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
        metadata: isAws ? { region: awsRegion.trim(), service: awsService.trim(), access_key_id_secret: awsAccessKeyId } : null,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.connections(projectId) });
      onDone();
    },
  });

  const baseUrlRequired = !BASE_URL_OPTIONAL[kind];
  const awsComplete = !isAws || (!!awsRegion.trim() && !!awsService.trim() && !!awsAccessKeyId && !!authSecret);
  const canSubmit = !!name.trim() && (!baseUrlRequired || !!baseUrl.trim()) && awsComplete && !create.isPending;

  const secretOptions = (
    <>
      <option value="">{t("connections.noCredential")}</option>
      {secrets.map((v) => (
        <option key={v.id} value={v.name}>
          {v.name}
        </option>
      ))}
    </>
  );

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        if (canSubmit) create.mutate();
      }}
    >
      <Field label={t("common.kind")} htmlFor="conn-kind">
        <Select id="conn-kind" value={kind} onChange={(e) => setKind(e.target.value as ConnectionKind)}>
          {CONNECTION_KINDS.map((k) => (
            <option key={k} value={k}>
              {t(KIND_LABEL_KEY[k])}
            </option>
          ))}
        </Select>
      </Field>
      <Field label={t("common.name")} htmlFor="conn-name">
        <Input id="conn-name" placeholder={kind === "generic_http" ? "backend-api" : kind} value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
      <Field label={t("connections.baseUrl")} hint={baseUrlRequired ? undefined : t("connections.optional")} htmlFor="conn-url">
        <Input id="conn-url" placeholder={DEFAULT_BASE_URL[kind]} value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} className="font-mono" />
      </Field>
      {isAws && (
        <>
          <div className="grid grid-cols-2 gap-3">
            <Field label={t("connections.aws.region")} htmlFor="conn-aws-region">
              <Input id="conn-aws-region" placeholder="ap-northeast-1" value={awsRegion} onChange={(e) => setAwsRegion(e.target.value)} className="font-mono" />
            </Field>
            <Field label={t("connections.aws.service")} htmlFor="conn-aws-service">
              <Input id="conn-aws-service" placeholder="sqs" value={awsService} onChange={(e) => setAwsService(e.target.value)} className="font-mono" />
            </Field>
          </div>
          <Field label={t("connections.aws.accessKeyId")} htmlFor="conn-aws-access-key">
            <Select id="conn-aws-access-key" value={awsAccessKeyId} onChange={(e) => setAwsAccessKeyId(e.target.value)} className="font-mono" disabled={vars.isLoading}>
              {secretOptions}
            </Select>
          </Field>
        </>
      )}
      <Field label={isAws ? t("connections.aws.secretAccessKey") : t("connections.credential")} htmlFor="conn-secret">
        <Select id="conn-secret" value={authSecret} onChange={(e) => setAuthSecret(e.target.value)} className="font-mono" disabled={vars.isLoading}>
          {secretOptions}
        </Select>
      </Field>
      <Field label={t("connections.authStyle")} hint={t("connections.authStyleAuto", { style: DEFAULT_AUTH_STYLE[kind] })} htmlFor="conn-auth">
        <Input id="conn-auth" placeholder="bearer | header:X-Api-Key | query:key | none" value={authStyle} onChange={(e) => setAuthStyle(e.target.value)} className="font-mono" />
      </Field>

      <p className="flex items-start gap-1.5 font-mono text-[11px] text-muted-foreground">
        <KeyRound className="mt-0.5 h-3 w-3 shrink-0" />
        {secrets.length === 0 && vars.data ? t("connections.noSecretsHint") : t("connections.credentialHint")}
      </p>
      {create.error && <ErrorNote error={create.error} />}
      {vars.error && <ErrorNote error={vars.error} />}
      <div className="flex justify-end">
        <Button type="submit" disabled={!canSubmit}>
          {create.isPending ? <GoldfishInline /> : <Plus className="h-4 w-4" />} {t("common.save")}
        </Button>
      </div>
    </form>
  );
}
