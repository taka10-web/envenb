import { z } from "zod";

// Mirrors envfish-core models. Secret values never cross the IPC boundary:
// `value` is null whenever `kind === "SECRET"`, and `auth_secret` on a
// connection is the *name* of a secret variable, never its value.

export const ProjectSchema = z.object({
  id: z.string(),
  name: z.string(),
  local_path: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});
export type Project = z.infer<typeof ProjectSchema>;

export const EnvironmentSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  name: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
});
export type Environment = z.infer<typeof EnvironmentSchema>;

export const VariableKindSchema = z.enum(["PUBLIC", "SECRET"]);
export type VariableKind = z.infer<typeof VariableKindSchema>;

export const VariableSchema = z.object({
  id: z.string(),
  environment_id: z.string(),
  name: z.string(),
  kind: VariableKindSchema,
  value: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});
export type Variable = z.infer<typeof VariableSchema>;

export const StatusSchema = z.object({
  data_dir: z.string(),
  database_path: z.string(),
  master_key_location: z.string(),
  project_count: z.number(),
  secret_count: z.number(),
});
export type Status = z.infer<typeof StatusSchema>;

export const SettingsSchema = z.object({
  language: z.enum(["ja", "en", "system"]),
  key_backend: z.enum(["file", "keychain"]),
  theme: z.enum(["system", "light", "dark"]),
});
export type Settings = z.infer<typeof SettingsSchema>;

export const ConnectionKindSchema = z.enum(["generic_http", "openai", "supabase"]);
export type ConnectionKind = z.infer<typeof ConnectionKindSchema>;

export const ConnectionSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  environment_id: z.string(),
  kind: ConnectionKindSchema,
  name: z.string(),
  base_url: z.string(),
  auth_secret: z.string().nullable(),
  auth_style: z.string(),
  metadata: z.unknown(),
  created_at: z.string(),
  updated_at: z.string(),
});
export type Connection = z.infer<typeof ConnectionSchema>;

export const AiClientSchema = z.object({
  id: z.string(),
  name: z.string(),
  kind: z.string(),
  created_at: z.string(),
  last_seen_at: z.string().nullable(),
});
export type AiClient = z.infer<typeof AiClientSchema>;

export const ActionSchema = z.enum(["READ", "WRITE", "DELETE"]);
export type Action = z.infer<typeof ActionSchema>;
export const ACTIONS: Action[] = ["READ", "WRITE", "DELETE"];

export const DecisionSchema = z.enum(["ALLOW", "ASK", "DENY"]);
export type Decision = z.infer<typeof DecisionSchema>;
export const DECISIONS: Decision[] = ["ALLOW", "ASK", "DENY"];

export const PermissionSchema = z.object({
  id: z.string(),
  client_id: z.string().nullable(),
  project_id: z.string().nullable(),
  environment_id: z.string().nullable(),
  connection_id: z.string().nullable(),
  action: ActionSchema,
  decision: DecisionSchema,
  created_at: z.string(),
  updated_at: z.string(),
});
export type Permission = z.infer<typeof PermissionSchema>;

export const ApprovalStatusSchema = z.enum(["PENDING", "APPROVED", "DENIED", "EXPIRED"]);
export const ApprovalSchema = z.object({
  id: z.string(),
  client_id: z.string(),
  client_name: z.string(),
  project_id: z.string().nullable(),
  environment_id: z.string().nullable(),
  connection_id: z.string().nullable(),
  action: ActionSchema,
  summary: z.string(),
  status: ApprovalStatusSchema,
  created_at: z.string(),
  resolved_at: z.string().nullable(),
  expires_at: z.string(),
});
export type Approval = z.infer<typeof ApprovalSchema>;

export const AuditEntrySchema = z.object({
  id: z.string(),
  client_id: z.string().nullable(),
  client_name: z.string(),
  project_id: z.string().nullable(),
  project_name: z.string().nullable(),
  environment_id: z.string().nullable(),
  environment_name: z.string().nullable(),
  connection_id: z.string().nullable(),
  connection_name: z.string().nullable(),
  action: ActionSchema,
  summary: z.string(),
  decision: z.string(), // ALLOWED | DENIED | ASKED | ERROR
  created_at: z.string(),
});
export type AuditEntry = z.infer<typeof AuditEntrySchema>;

export const DotenvPreviewSchema = z.object({
  entries: z.array(
    z.object({
      name: z.string(),
      value: z.string(),
      suggestion: z.enum(["PUBLIC", "SECRET", "REVIEW"]),
      line: z.number(),
    }),
  ),
  invalid_lines: z.array(z.number()),
});
export type DotenvPreview = z.infer<typeof DotenvPreviewSchema>;

export const ImportReportSchema = z.object({
  public_added: z.number(),
  secret_added: z.number(),
  skipped: z.array(z.string()),
});
export type ImportReport = z.infer<typeof ImportReportSchema>;
