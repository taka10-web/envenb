import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  AiClientSchema,
  ApprovalSchema,
  AuditEntrySchema,
  ConnectionSchema,
  CredentialKindSchema,
  CredentialSchema,
  FieldSpecSchema,
  DotenvPreviewSchema,
  EnvironmentSchema,
  ImportReportSchema,
  PermissionSchema,
  ProjectSchema,
  SettingsSchema,
  StatusSchema,
  VariableSchema,
  DecisionSchema,
  type Action,
  type ConnectionKind,
  type CredentialKind,
  type Decision,
  type Environment,
  type Project,
  type Variable,
  type VariableKind,
} from "./types";

// Every Tauri command is wrapped here and validated with Zod, so pages never
// call `invoke` directly and the set of IPC entry points stays auditable.
// Note what is absent: there is no `getSecret` / `revealSecret`.

async function call<T>(cmd: string, schema: z.ZodType<T>, args?: Record<string, unknown>): Promise<T> {
  const raw = await invoke(cmd, args);
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    // Name the command so a contract mismatch is diagnosable from the UI.
    const shape = Array.isArray(raw) ? "array" : raw === null ? "null" : typeof raw;
    const detail = parsed.error.issues.map((i) => `${i.path.join(".") || "(root)"}: ${i.message}`).join("; ");
    const err = new Error(`${cmd}: unexpected response (${shape}) — ${detail}`);
    if (import.meta.env.DEV) console.error(err, raw);
    throw err;
  }
  return parsed.data;
}

export const api = {
  status: () => call("status", StatusSchema),

  // settings
  getSettings: () => call("get_settings", SettingsSchema),
  setLanguage: (language: "ja" | "en" | "system") => call("set_language", SettingsSchema, { language }),
  setTheme: (theme: "system" | "light" | "dark") => call("set_theme", SettingsSchema, { theme }),

  // projects
  listProjects: () => call("list_projects", z.array(ProjectSchema)),
  createProject: (name: string, localPath: string | null): Promise<Project> =>
    call("create_project", ProjectSchema, { name, localPath }),
  deleteProject: (projectId: string) => call("delete_project", z.null(), { projectId }),

  // environments
  listEnvironments: (projectId: string) => call("list_environments", z.array(EnvironmentSchema), { projectId }),
  createEnvironment: (projectId: string, name: string): Promise<Environment> =>
    call("create_environment", EnvironmentSchema, { projectId, name }),
  deleteEnvironment: (environmentId: string) => call("delete_environment", z.null(), { environmentId }),

  // variables
  listVariables: (environmentId: string) => call("list_variables", z.array(VariableSchema), { environmentId }),
  setPublicVariable: (environmentId: string, name: string, value: string): Promise<Variable> =>
    call("set_public_variable", VariableSchema, { environmentId, name, value }),
  /** The value travels to Rust once and is sealed there. It is never returned. */
  setSecretVariable: (environmentId: string, name: string, value: string): Promise<Variable> =>
    call("set_secret_variable", VariableSchema, { environmentId, name, value }),
  deleteVariable: (environmentId: string, name: string) => call("delete_variable", z.null(), { environmentId, name }),
  renderEnvExample: (environmentId: string) => call("render_env_example", z.string(), { environmentId }),

  // .env import
  previewDotenv: (text: string) => call("preview_dotenv", DotenvPreviewSchema, { text }),
  importVariables: (environmentId: string, entries: { name: string; value: string; kind: VariableKind }[]) =>
    call("import_variables", ImportReportSchema, { environmentId, entries }),

  // connections
  listConnections: (projectId: string) => call("list_connections", z.array(ConnectionSchema), { projectId }),
  createConnection: (input: {
    environment_id: string;
    kind: ConnectionKind;
    name: string;
    base_url: string | null;
    auth_secret: string | null;
    auth_style: string | null;
    /** Non-secret extras. For aws: { region, service, access_key_id_secret }. */
    metadata?: Record<string, string> | null;
  }) => call("create_connection", ConnectionSchema, { input }),
  deleteConnection: (connectionId: string) => call("delete_connection", z.null(), { connectionId }),

  // AI access
  listAiClients: () => call("list_ai_clients", z.array(AiClientSchema)),
  registerAiClient: (name: string, kind: string) => call("register_ai_client", AiClientSchema, { name, kind }),
  deleteAiClient: (clientId: string) => call("delete_ai_client", z.null(), { clientId }),
  listPermissions: () => call("list_permissions", z.array(PermissionSchema)),
  setPermission: (input: {
    client_id: string | null;
    project_id: string | null;
    environment_id: string | null;
    connection_id: string | null;
    action: Action;
    decision: Decision;
  }) => call("set_permission", PermissionSchema, { input }),
  deletePermission: (permissionId: string) => call("delete_permission", z.null(), { permissionId }),
  effectiveDecision: (scope: {
    clientId: string | null;
    projectId: string | null;
    environmentId: string | null;
    connectionId: string | null;
    action: Action;
  }) => call("effective_decision", DecisionSchema, scope),
  listApprovals: (pendingOnly: boolean) => call("list_approvals", z.array(ApprovalSchema), { pendingOnly }),
  resolveApproval: (approvalId: string, approve: boolean) =>
    call("resolve_approval", ApprovalSchema, { approvalId, approve }),

  // activity
  listAudit: (limit = 200) => call("list_audit", z.array(AuditEntrySchema), { limit }),

  // credentials (accounts / ssh / database / file). Secret fields never come back.
  listCredentials: (projectId: string) => call("list_credentials", z.array(CredentialSchema), { projectId }),
  credentialFieldSpecs: () =>
    call("credential_field_specs", z.array(z.tuple([CredentialKindSchema, z.array(FieldSpecSchema)]))),
  createCredential: (input: {
    environment_id: string;
    kind: CredentialKind;
    name: string;
    note: string | null;
    fields: { field: string; value: string }[];
  }) => call("create_credential", CredentialSchema, { input }),
  updateCredentialFields: (credentialId: string, fields: { field: string; value: string }[], note: string | null) =>
    call("update_credential_fields", CredentialSchema, { credentialId, fields, note }),
  deleteCredential: (credentialId: string) => call("delete_credential", z.null(), { credentialId }),
  /** Human-only. Copies to the OS clipboard in Rust; resolves with the auto-clear TTL in seconds. Pass field "totp" for the current one-time code. */
  copyCredentialField: (credentialId: string, field: string) =>
    call("copy_credential_field", z.number(), { credentialId, field }),
};

export const queryKeys = {
  status: ["status"] as const,
  settings: ["settings"] as const,
  projects: ["projects"] as const,
  environments: (projectId: string) => ["environments", projectId] as const,
  variables: (environmentId: string) => ["variables", environmentId] as const,
  connections: (projectId: string) => ["connections", projectId] as const,
  aiClients: ["ai-clients"] as const,
  permissions: ["permissions"] as const,
  approvals: (pendingOnly: boolean) => ["approvals", pendingOnly] as const,
  audit: ["audit"] as const,
  credentials: (projectId: string) => ["credentials", projectId] as const,
  credentialSpecs: ["credential-specs"] as const,
};
