import { useEffect, useRef, useState, type ChangeEvent, type CSSProperties } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, FolderOpen, KeyRound, Lock, Pencil, Plus, Trash2 } from "lucide-react";
import { Button, GoldfishInline, GoldfishLoader, Input } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { CREDENTIAL_KINDS, type Credential, type CredentialKind, type FieldSpec } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { Field } from "../components/Field";
import { KindDot, type KindTone } from "../components/KindDot";
import { WithEnvironment } from "../components/NeedsContext";
import { SectionLabel } from "../components/SectionLabel";
import { Select } from "../components/Select";
import { Sheet } from "../components/Sheet";
import { ListRow, RowActions } from "../components/Table";
import { useI18n, type MessageKey } from "../lib/i18n";
import { useAppContext } from "../lib/context";
import { confirmAsync } from "../lib/confirm";

const KIND_LABEL_KEY: Record<CredentialKind, MessageKey> = {
  account: "creds.kind.account",
  ssh: "creds.kind.ssh",
  database: "creds.kind.database",
  file: "creds.kind.file",
};
const KIND_TONE: Record<CredentialKind, KindTone> = { account: "sky", ssh: "violet", database: "amber", file: "slate" };

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

export function CredentialsPage() {
  const { t } = useI18n();
  const { isLoading, projectId, environmentId } = useAppContext();
  const ready = !isLoading && !!projectId && !!environmentId;
  return (
    <div>
      {/* Once the context is usable the inner view owns the header (it hosts the Add button). */}
      {!ready && <PageHeader title={t("creds.title")} context />}
      <WithEnvironment>{({ projectId, environmentId }) => <EnvironmentCredentials key={environmentId} projectId={projectId} environmentId={environmentId} />}</WithEnvironment>
    </div>
  );
}

type SheetState = { mode: "closed" } | { mode: "new" } | { mode: "edit"; credential: Credential };

function EnvironmentCredentials({ projectId, environmentId }: { projectId: string; environmentId: string }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const specs = useFieldSpecs();
  const creds = useQuery({ queryKey: queryKeys.credentials(projectId), queryFn: () => api.listCredentials(projectId) });
  const [sheet, setSheet] = useState<SheetState>({ mode: "closed" });
  const close = () => setSheet({ mode: "closed" });

  const invalidate = () => void qc.invalidateQueries({ queryKey: queryKeys.credentials(projectId) });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteCredential(id),
    onSuccess: (_data, id) => {
      setSheet((s) => (s.mode === "edit" && s.credential.id === id ? { mode: "closed" } : s));
      invalidate();
    },
  });

  const list = creds.data?.filter((c) => c.environment_id === environmentId) ?? [];
  const byKind = CREDENTIAL_KINDS.map((k) => [k, list.filter((c) => c.kind === k)] as const).filter(([, cs]) => cs.length > 0);
  const loading = specs.isLoading || creds.isLoading;

  return (
    <div>
      <PageHeader
        title={t("creds.title")}
        context
        actions={
          <Button size="sm" onClick={() => setSheet({ mode: "new" })} disabled={!specs.data}>
            <Plus className="h-3.5 w-3.5" /> {t("creds.add")}
          </Button>
        }
      />
      {specs.error && <ErrorNote error={specs.error} />}
      {creds.error && <ErrorNote error={creds.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {loading && <GoldfishLoader label={t("common.loading")} className="py-16" />}

      {!loading && list.length === 0 && (
        <EmptyState text={t("creds.empty")}>
          <Button size="sm" onClick={() => setSheet({ mode: "new" })} disabled={!specs.data}>
            <Plus className="h-3.5 w-3.5" /> {t("creds.add")}
          </Button>
        </EmptyState>
      )}

      <div className="flex flex-col gap-6">
        {byKind.map(([kind, cs]) => (
          <section key={kind}>
            <SectionLabel>
              <KindDot tone={KIND_TONE[kind]} label={t(KIND_LABEL_KEY[kind])} className="uppercase" />
            </SectionLabel>
            <div className="-mx-2">
              {cs.map((c) => (
                <CredentialRow
                  key={c.id}
                  credential={c}
                  onEdit={() => setSheet({ mode: "edit", credential: c })}
                  onDelete={() => {
                    void confirmAsync(t("creds.confirmDelete", { name: c.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
                      if (ok) remove.mutate(c.id);
                    });
                  }}
                />
              ))}
            </div>
          </section>
        ))}
      </div>

      {specs.data && (
        <Sheet open={sheet.mode !== "closed"} title={sheet.mode === "edit" ? t("creds.editing", { name: sheet.credential.name }) : t("creds.add")} onClose={close}>
          {sheet.mode === "edit" && (
            <CredentialForm
              key={sheet.credential.id}
              specs={specs.data}
              editing={sheet.credential}
              onDone={() => {
                close();
                invalidate();
              }}
            />
          )}
          {sheet.mode === "new" && (
            <CredentialForm
              key="new"
              specs={specs.data}
              environmentId={environmentId}
              onDone={() => {
                close();
                invalidate();
              }}
            />
          )}
        </Sheet>
      )}
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
    <ListRow className="py-1.5">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-baseline gap-x-3 gap-y-0.5">
          <span className="text-sm font-medium">{c.name}</span>
          {parts.length > 0 && <span className="truncate font-mono text-xs text-muted-foreground">{parts.join(" · ")}</span>}
          {c.note && <span className="truncate text-xs text-muted-foreground">{c.note}</span>}
        </div>
      </div>
      {(secretFields.length > 0 || hasTotp) && (
        <div className="flex flex-wrap items-center justify-end gap-1">
          {secretFields.map((f) => (
            <CopyButton key={f.field} credentialId={c.id} field={f.field} label={fieldLabel(t, f.field)} ariaLabel={t("creds.copyAria", { field: fieldLabel(t, f.field), name: c.name })} />
          ))}
          {hasTotp && <CopyButton credentialId={c.id} field="totp" label={t("creds.copyCode")} ariaLabel={t("creds.copyCodeAria", { name: c.name })} code />}
        </div>
      )}
      <RowActions>
        <Button variant="ghost" size="icon-sm" aria-label={t("creds.updateAria", { name: c.name })} onClick={onEdit}>
          <Pencil className="h-3.5 w-3.5" />
        </Button>
        <Button variant="ghost" size="icon-sm" aria-label={t("envs.deleteAria", { name: c.name })} onClick={onDelete}>
          <Trash2 className="h-3.5 w-3.5" />
        </Button>
      </RowActions>
    </ListRow>
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
    <span className="inline-flex items-center gap-1 font-mono text-[11px]">
      <Button type="button" variant="outline" size="sm" className="h-6 gap-1 px-1.5 font-mono text-[11px]" aria-label={ariaLabel} onClick={() => copy.mutate()} disabled={copy.isPending}>
        {copy.isPending ? <GoldfishInline size={1} /> : <Copy className="h-3 w-3" />} {code ? label : `${label} ${MASK}`}
      </Button>
      {ttl !== null && <span className="text-muted-foreground">{t("creds.copied", { seconds: ttl })}</span>}
      {copy.error && <span className="text-destructive">{copy.error instanceof Error ? copy.error.message : t("common.unexpectedError")}</span>}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Add / update form, rendered from the FieldSpecs of the chosen kind.

type FormProps =
  | { specs: SpecMap; environmentId: string; editing?: undefined; onDone: () => void }
  | { specs: SpecMap; environmentId?: undefined; editing: Credential; onDone: () => void };

function CredentialForm({ specs, environmentId, editing, onDone }: FormProps) {
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
        const changed = fields
          .filter((f) => (values[f.name] ?? "") !== "" && (f.secret || values[f.name] !== (fieldValue(editing, f.name) ?? "")))
          .map((f) => ({ field: f.name, value: values[f.name] }));
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
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        if (canSubmit) save.mutate();
      }}
    >
      <Field label={t("common.name")} htmlFor="cred-name">
        <Input id="cred-name" placeholder="my-account" value={name} onChange={(e) => setName(e.target.value)} disabled={!!editing} />
      </Field>
      <Field label={t("common.kind")} htmlFor="cred-kind">
        <Select id="cred-kind" value={kind} onChange={(e) => changeKind(e.target.value as CredentialKind)} disabled={!!editing}>
          {CREDENTIAL_KINDS.map((k) => (
            <option key={k} value={k}>
              {t(KIND_LABEL_KEY[k])}
            </option>
          ))}
        </Select>
      </Field>
      {fields.map((spec) => (
        <FieldInput key={`${kind}-${spec.name}`} spec={spec} value={values[spec.name] ?? ""} onChange={(v) => setValues((vs) => ({ ...vs, [spec.name]: v }))} editing={!!editing} />
      ))}
      <Field label={t("creds.note")} hint={t("creds.optional")} htmlFor="cred-note">
        <Input id="cred-note" value={note} onChange={(e) => setNote(e.target.value)} />
      </Field>

      <p className="flex items-start gap-1.5 font-mono text-[11px] text-muted-foreground">
        <Lock className="mt-0.5 h-3 w-3 shrink-0" />
        {editing ? t("creds.editHint") : t("creds.secretHint")}
      </p>
      {save.error && <ErrorNote error={save.error} />}
      <div className="flex justify-end">
        <Button type="submit" disabled={!canSubmit}>
          {save.isPending ? <GoldfishInline /> : editing ? <Pencil className="h-4 w-4" /> : <Plus className="h-4 w-4" />} {editing ? t("creds.update") : t("common.save")}
        </Button>
      </div>
    </form>
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
    <>
      {fieldLabel(t, spec.name)}
      {spec.required && !editing && <span className="ml-0.5 text-destructive" title={t("creds.required")}>*</span>}
      {spec.secret && <KeyRound className="ml-1 inline h-3 w-3 text-muted-foreground" aria-hidden />}
    </>
  );

  if (spec.multiline) {
    return (
      <Field label={label} htmlFor={id}>
        <textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          rows={5}
          spellCheck={false}
          autoComplete="off"
          placeholder={spec.secret && editing ? MASK : undefined}
          className="w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs placeholder:text-muted-foreground focus-visible:border-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
          style={spec.secret ? SECRET_TEXTAREA_STYLE : undefined}
        />
        <div className="flex items-center gap-2">
          <input ref={fileInput} type="file" hidden data-testid={`${id}-file`} onChange={(e) => void onFileChosen(e)} />
          <Button type="button" size="sm" variant="outline" onClick={() => fileInput.current?.click()}>
            <FolderOpen className="h-3.5 w-3.5" /> {t("creds.loadFromFile")}
          </Button>
          {fileName && <span className="font-mono text-xs text-muted-foreground">{t("creds.fileLoaded", { name: fileName })}</span>}
        </div>
      </Field>
    );
  }

  return (
    <Field label={label} htmlFor={id}>
      <Input
        id={id}
        type={spec.secret ? "password" : "text"}
        autoComplete="off"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={spec.secret && editing ? MASK : undefined}
        className={spec.secret || spec.name === "url" || spec.name === "host" || spec.name === "port" ? "font-mono" : undefined}
      />
    </Field>
  );
}
