import { useEffect, useRef, useState, type ChangeEvent, type CSSProperties } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ArrowRight, Copy, FolderOpen, KeyRound, Lock, Pencil, Plus, Trash2, X } from "lucide-react";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, GoldfishInline, GoldfishLoader, Input, Label } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { CREDENTIAL_KINDS, type Credential, type CredentialKind, type Environment, type FieldSpec, type Project } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { Select } from "../components/Select";
import { useI18n, type MessageKey } from "../lib/i18n";

const KIND_LABEL_KEY: Record<CredentialKind, MessageKey> = {
  account: "creds.kind.account",
  ssh: "creds.kind.ssh",
  database: "creds.kind.database",
  file: "creds.kind.file",
};

// Field names come from the Rust specs; label them when we know them and fall
// back to the raw name so an unknown field is still usable.
const FIELD_LABEL_KEY: Record<string, MessageKey> = {
  url: "creds.field.url",
  username: "creds.field.username",
  password: "creds.field.password",
  totp_secret: "creds.field.totp_secret",
  host: "creds.field.host",
  port: "creds.field.port",
  user: "creds.field.user",
  private_key: "creds.field.private_key",
  passphrase: "creds.field.passphrase",
  engine: "creds.field.engine",
  database: "creds.field.database",
  filename: "creds.field.filename",
  content: "creds.field.content",
};

const MASK = "••••••••";
// Hides typed characters in a multiline secret textarea (WebKit-only, harmless elsewhere).
const SECRET_TEXTAREA_STYLE = { WebkitTextSecurity: "disc" } as unknown as CSSProperties;

type SpecMap = Partial<Record<CredentialKind, FieldSpec[]>>;

function useFieldSpecs() {
  return useQuery({
    queryKey: queryKeys.credentialSpecs,
    queryFn: api.credentialFieldSpecs,
    staleTime: Infinity,
    select: (pairs): SpecMap => Object.fromEntries(pairs) as SpecMap,
  });
}

function fieldValue(c: Credential, field: string): string | null {
  return c.fields.find((f) => f.field === field)?.value ?? null;
}
function fieldPresent(c: Credential, field: string): boolean {
  return c.fields.find((f) => f.field === field)?.present ?? false;
}

/** One-line, non-secret summary shown next to the name (host:port, user, url, …). */
function summary(c: Credential): string[] {
  const parts: string[] = [];
  const host = fieldValue(c, "host");
  const port = fieldValue(c, "port");
  if (host) parts.push(port ? `${host}:${port}` : host);
  for (const f of ["engine", "database", "user", "url", "filename"]) {
    const v = fieldValue(c, f);
    if (v) parts.push(f === "user" ? `user ${v}` : v);
  }
  return parts;
}

export function CredentialsPage({ project }: { project?: Project }) {
  return project ? <ProjectCredentials project={project} /> : <AllCredentials />;
}

// ---------------------------------------------------------------------------
// Global route: every project, its credentials, and a link to the project tab.

function AllCredentials() {
  const { t } = useI18n();
  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const perProject = useQueries({
    queries: (projects.data ?? []).map((p) => ({
      queryKey: queryKeys.credentials(p.id),
      queryFn: () => api.listCredentials(p.id),
    })),
  });
  const loading = projects.isLoading || perProject.some((q) => q.isLoading);
  const total = perProject.reduce((n, q) => n + (q.data?.length ?? 0), 0);

  return (
    <div className="p-8">
      <PageHeader title={t("creds.title")} description={t("creds.description")} />
      {projects.error && <ErrorNote error={projects.error} />}
      {perProject.map((q, i) => q.error && <ErrorNote key={projects.data?.[i]?.id ?? i} error={q.error} />)}
      {loading && <GoldfishLoader label={t("common.loading")} className="py-16" />}

      {!loading && projects.data?.length === 0 && (
        <EmptyState text={t("creds.noProjects")}>
          <Button asChild variant="secondary" size="sm">
            <Link to="/projects">{t("nav.projects")}</Link>
          </Button>
        </EmptyState>
      )}
      {!loading && projects.data && projects.data.length > 0 && total === 0 && <EmptyState text={t("creds.emptyAll")} />}

      <div className="grid gap-4">
        {projects.data?.map((p, i) => {
          const list = perProject[i]?.data ?? [];
          if (list.length === 0) return null;
          return (
            <Card key={p.id}>
              <CardHeader className="flex-row items-center justify-between space-y-0">
                <CardTitle>{p.name}</CardTitle>
                <Button asChild variant="ghost" size="sm">
                  <Link to={`/projects/${p.id}/credentials`}>
                    {t("creds.manage")} <ArrowRight className="h-3.5 w-3.5" />
                  </Link>
                </Button>
              </CardHeader>
              <CardContent>
                <ul className="flex flex-col divide-y text-sm">
                  {list.map((c) => (
                    <li key={c.id} className="flex flex-wrap items-center gap-2 py-2">
                      <Badge variant="secondary">{t(KIND_LABEL_KEY[c.kind])}</Badge>
                      <span className="font-medium">{c.name}</span>
                      <span className="font-mono text-xs text-muted-foreground">{summary(c).join(" · ")}</span>
                    </li>
                  ))}
                </ul>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Project tab: environment pills → credentials grouped by kind + add/update form.

function ProjectCredentials({ project }: { project: Project }) {
  const { t } = useI18n();
  const { environmentId } = useParams();
  const navigate = useNavigate();
  const envs = useQuery({ queryKey: queryKeys.environments(project.id), queryFn: () => api.listEnvironments(project.id) });
  const selected = environmentId ?? envs.data?.[0]?.id;

  if (envs.isLoading) return <GoldfishLoader label={t("common.loading")} className="py-16" />;
  if (envs.error) return <ErrorNote error={envs.error} />;
  if (!envs.data?.length) return <p className="py-8 text-sm text-muted-foreground">{t("vars.createEnvFirst")}</p>;
  const env = envs.data.find((e) => e.id === selected) ?? envs.data[0];

  return (
    <div>
      <div className="mb-5 flex flex-wrap gap-1">
        {envs.data.map((e) => (
          <Button key={e.id} size="sm" variant={e.id === env.id ? "default" : "outline"} onClick={() => navigate(`../credentials/${e.id}`)}>
            {e.name}
          </Button>
        ))}
      </div>
      <EnvironmentCredentials key={env.id} project={project} environment={env} />
    </div>
  );
}

function EnvironmentCredentials({ project, environment }: { project: Project; environment: Environment }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const specs = useFieldSpecs();
  const creds = useQuery({ queryKey: queryKeys.credentials(project.id), queryFn: () => api.listCredentials(project.id) });
  const [editing, setEditing] = useState<Credential | null>(null);

  const invalidate = () => void qc.invalidateQueries({ queryKey: queryKeys.credentials(project.id) });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteCredential(id),
    onSuccess: (_data, id) => {
      setEditing((e) => (e && e.id === id ? null : e));
      invalidate();
    },
  });

  if (specs.isLoading || creds.isLoading) return <GoldfishLoader label={t("common.loading")} className="py-16" />;

  const list = creds.data?.filter((c) => c.environment_id === environment.id) ?? [];
  const byKind = CREDENTIAL_KINDS.map((k) => [k, list.filter((c) => c.kind === k)] as const).filter(([, cs]) => cs.length > 0);

  return (
    <div>
      {specs.error && <ErrorNote error={specs.error} />}
      {specs.data &&
        (editing ? (
          <CredentialForm
            key={editing.id}
            specs={specs.data}
            editing={editing}
            onDone={() => {
              setEditing(null);
              invalidate();
            }}
            onCancel={() => setEditing(null)}
          />
        ) : (
          <CredentialForm key="new" specs={specs.data} environmentId={environment.id} onDone={invalidate} />
        ))}
      {creds.error && <ErrorNote error={creds.error} />}
      {remove.error && <ErrorNote error={remove.error} />}

      {list.length === 0 && <EmptyState text={t("creds.empty")} />}

      <div className="grid gap-4">
        {byKind.map(([kind, cs]) => (
          <Card key={kind}>
            <CardHeader>
              <CardTitle className="text-sm">{t(KIND_LABEL_KEY[kind])}</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col divide-y">
              {cs.map((c) => (
                <CredentialRow
                  key={c.id}
                  credential={c}
                  onEdit={() => setEditing(c)}
                  onDelete={() => {
                    if (window.confirm(t("creds.confirmDelete", { name: c.name }))) remove.mutate(c.id);
                  }}
                />
              ))}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------

function fieldLabel(t: ReturnType<typeof useI18n>["t"], field: string): string {
  const key = FIELD_LABEL_KEY[field];
  return key ? t(key) : field;
}

function CredentialRow({ credential: c, onEdit, onDelete }: { credential: Credential; onEdit: () => void; onDelete: () => void }) {
  const { t } = useI18n();
  const secretFields = c.fields.filter((f) => f.secret && f.present);
  const hasTotp = c.kind === "account" && fieldPresent(c, "totp_secret");
  const parts = summary(c);

  return (
    <div className="flex flex-wrap items-start gap-x-4 gap-y-2 py-3">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="secondary">{t(KIND_LABEL_KEY[c.kind])}</Badge>
          <span className="font-medium">{c.name}</span>
          {parts.length > 0 && <span className="font-mono text-xs text-muted-foreground">{parts.join(" · ")}</span>}
        </div>
        {c.note && <p className="mt-1 text-xs text-muted-foreground">{c.note}</p>}
        {(secretFields.length > 0 || hasTotp) && (
          <div className="mt-2 flex flex-wrap items-center gap-2">
            {secretFields.map((f) => (
              <CopyButton key={f.field} credentialId={c.id} field={f.field} label={fieldLabel(t, f.field)} ariaLabel={t("creds.copyAria", { field: fieldLabel(t, f.field), name: c.name })} />
            ))}
            {hasTotp && <CopyButton credentialId={c.id} field="totp" label={t("creds.copyCode")} ariaLabel={t("creds.copyCodeAria", { name: c.name })} code />}
          </div>
        )}
      </div>
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="sm" aria-label={t("creds.updateAria", { name: c.name })} onClick={onEdit}>
          <Pencil className="h-3.5 w-3.5" /> {t("creds.update")}
        </Button>
        <Button variant="ghost" size="icon" aria-label={t("envs.deleteAria", { name: c.name })} onClick={onDelete}>
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

/** Copy happens in Rust; the webview only learns how long the clipboard keeps it. */
function CopyButton({ credentialId, field, label, ariaLabel, code = false }: { credentialId: string; field: string; label: string; ariaLabel: string; code?: boolean }) {
  const { t } = useI18n();
  const [ttl, setTtl] = useState<number | null>(null);
  useEffect(() => {
    if (ttl === null) return;
    const id = window.setTimeout(() => setTtl(null), 3000);
    return () => window.clearTimeout(id);
  }, [ttl]);
  const copy = useMutation({
    mutationFn: () => api.copyCredentialField(credentialId, field),
    onSuccess: (seconds) => setTtl(seconds),
  });

  return (
    <span className="inline-flex items-center gap-1.5 rounded-md border bg-muted/40 px-2 py-1 text-xs">
      {!code && (
        <>
          <span className="text-muted-foreground">{label}</span>
          <span className="font-mono text-muted-foreground">{MASK}</span>
        </>
      )}
      <Button type="button" variant="ghost" size="sm" className="h-6 px-1.5 text-xs" aria-label={ariaLabel} onClick={() => copy.mutate()} disabled={copy.isPending}>
        {copy.isPending ? <GoldfishInline /> : <Copy className="h-3 w-3" />} {code ? label : t("creds.copy")}
      </Button>
      {ttl !== null && <span className="text-muted-foreground">{t("creds.copied", { seconds: ttl })}</span>}
      {copy.error && <span className="text-destructive">{copy.error instanceof Error ? copy.error.message : t("common.unexpectedError")}</span>}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Add / update form, rendered from the FieldSpecs of the chosen kind.

type FormProps =
  | { specs: SpecMap; environmentId: string; editing?: undefined; onDone: () => void; onCancel?: undefined }
  | { specs: SpecMap; environmentId?: undefined; editing: Credential; onDone: () => void; onCancel: () => void };

function CredentialForm({ specs, environmentId, editing, onDone, onCancel }: FormProps) {
  const { t } = useI18n();
  const [kind, setKind] = useState<CredentialKind>(editing?.kind ?? "account");
  const [name, setName] = useState(editing?.name ?? "");
  const [note, setNote] = useState(editing?.note ?? "");
  // Pre-fill non-secret values when editing; secret inputs always start empty.
  const [values, setValues] = useState<Record<string, string>>(() =>
    editing ? Object.fromEntries(editing.fields.filter((f) => !f.secret && f.value !== null).map((f) => [f.field, f.value as string])) : {},
  );
  const fields = specs[kind] ?? [];

  const changeKind = (k: CredentialKind) => {
    setKind(k);
    setValues({});
  };

  const save = useMutation({
    mutationFn: () => {
      if (editing) {
        // Only what the user filled in travels; an empty secret keeps the stored value.
        const changed = fields.filter((f) => (values[f.name] ?? "") !== "" && (f.secret || values[f.name] !== (fieldValue(editing, f.name) ?? ""))).map((f) => ({ field: f.name, value: values[f.name] }));
        return api.updateCredentialFields(editing.id, changed, note.trim() || null);
      }
      return api.createCredential({
        environment_id: environmentId,
        kind,
        name: name.trim(),
        note: note.trim() || null,
        fields: fields.filter((f) => (values[f.name] ?? "") !== "").map((f) => ({ field: f.name, value: values[f.name] })),
      });
    },
    onSuccess: () => {
      setName("");
      setNote("");
      setValues({});
      onDone();
    },
  });

  const requiredFilled = fields.every((f) => !f.required || editing || (values[f.name] ?? "").trim() !== "");
  const canSubmit = (editing ? true : !!environmentId && !!name.trim()) && requiredFilled && !save.isPending;

  return (
    <>
      <form
        className="mb-6 flex flex-col gap-3 rounded-lg border bg-card p-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (canSubmit) save.mutate();
        }}
      >
        {editing && (
          <div className="flex items-center justify-between gap-2">
            <p className="text-sm font-medium">{t("creds.editing", { name: editing.name })}</p>
            <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
              <X className="h-3.5 w-3.5" /> {t("creds.cancel")}
            </Button>
          </div>
        )}
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="cred-name">{t("common.name")}</Label>
            <Input id="cred-name" placeholder="my-account" value={name} onChange={(e) => setName(e.target.value)} className="w-44" disabled={!!editing} />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="cred-kind">{t("common.kind")}</Label>
            <Select id="cred-kind" value={kind} onChange={(e) => changeKind(e.target.value as CredentialKind)} className="w-40" disabled={!!editing}>
              {CREDENTIAL_KINDS.map((k) => (
                <option key={k} value={k}>
                  {t(KIND_LABEL_KEY[k])}
                </option>
              ))}
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="cred-note">
              {t("creds.note")} <span className="ml-1 font-normal text-muted-foreground">({t("creds.optional")})</span>
            </Label>
            <Input id="cred-note" value={note} onChange={(e) => setNote(e.target.value)} className="w-72" />
          </div>
        </div>
        <div className="flex flex-wrap items-end gap-3">
          {fields.map((spec) => (
            <FieldInput key={`${kind}-${spec.name}`} spec={spec} value={values[spec.name] ?? ""} onChange={(v) => setValues((vs) => ({ ...vs, [spec.name]: v }))} editing={!!editing} />
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <Button type="submit" disabled={!canSubmit}>
            {save.isPending ? <GoldfishInline /> : editing ? <Pencil className="h-4 w-4" /> : <Plus className="h-4 w-4" />} {editing ? t("creds.update") : t("creds.add")}
          </Button>
          <p className="text-xs text-muted-foreground">
            <Lock className="mr-1 inline h-3 w-3" />
            {editing ? t("creds.editHint") : t("creds.secretHint")}
          </p>
        </div>
      </form>
      {save.error && <ErrorNote error={save.error} />}
    </>
  );
}

function FieldInput({ spec, value, onChange, editing }: { spec: FieldSpec; value: string; onChange: (v: string) => void; editing: boolean }) {
  const { t } = useI18n();
  const id = `cred-field-${spec.name}`;
  const fileInput = useRef<HTMLInputElement>(null);
  const [fileName, setFileName] = useState<string | null>(null);

  // Pure web API: the file text goes into the textarea and from there to Rust once.
  const onFileChosen = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    onChange(await file.text());
    setFileName(file.name);
  };

  const label = (
    <Label htmlFor={id}>
      {fieldLabel(t, spec.name)}
      {spec.required && !editing && <span className="ml-0.5 text-destructive" title={t("creds.required")}>*</span>}
      {spec.secret && <KeyRound className="ml-1 inline h-3 w-3 text-muted-foreground" aria-hidden />}
    </Label>
  );

  if (spec.multiline) {
    return (
      <div className="flex basis-full flex-col gap-1.5">
        {label}
        <textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          rows={5}
          spellCheck={false}
          autoComplete="off"
          placeholder={spec.secret && editing ? MASK : undefined}
          className="w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          style={spec.secret ? SECRET_TEXTAREA_STYLE : undefined}
        />
        <div className="flex items-center gap-2">
          <input ref={fileInput} type="file" hidden data-testid={`${id}-file`} onChange={(e) => void onFileChosen(e)} />
          <Button type="button" size="sm" variant="outline" onClick={() => fileInput.current?.click()}>
            <FolderOpen className="h-3.5 w-3.5" /> {t("creds.loadFromFile")}
          </Button>
          {fileName && <span className="font-mono text-xs text-muted-foreground">{t("creds.fileLoaded", { name: fileName })}</span>}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1.5">
      {label}
      <Input
        id={id}
        type={spec.secret ? "password" : "text"}
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={spec.secret && editing ? MASK : undefined}
        className={spec.secret || spec.name === "url" || spec.name === "host" || spec.name === "port" ? "w-56 font-mono" : "w-56"}
      />
    </div>
  );
}
