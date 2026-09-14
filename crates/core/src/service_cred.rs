//! Credential service: structured secrets for humans (test accounts, SSH,
//! databases, files/certificates).
//!
//! Listing returns metadata and non-secret fields only. The two internal-tier
//! consumers of plaintext are [`EnvEnb::with_credential_field`] (closure-based,
//! used by `envenb ssh`, `envenb run`, and the clipboard copy in the desktop
//! app — all human-initiated) and nothing else. There is no AI-facing reader.

use chrono::Utc;

use crate::error::{CoreError, Result};
use crate::model::{Credential, CredentialKind};
use crate::repo_cred as repo;
use crate::secret::SecretValue;
use crate::service::EnvEnb;

/// One field to store. Every value arrives as a `SecretValue`; whether it is
/// sealed or stored plain is decided by the kind's [`FieldSpec`](crate::FieldSpec).
pub struct CredentialFieldInput {
    pub field: String,
    pub value: SecretValue,
}

impl std::fmt::Debug for CredentialFieldInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialFieldInput")
            .field("field", &self.field)
            .finish_non_exhaustive()
    }
}

pub struct NewCredential {
    pub environment_id: String,
    pub kind: CredentialKind,
    pub name: String,
    pub note: Option<String>,
    pub fields: Vec<CredentialFieldInput>,
}

impl EnvEnb {
    pub async fn list_credentials(&self, environment_id: &str) -> Result<Vec<Credential>> {
        self.get_environment(environment_id).await?;
        let mut out = repo::list_headers(self.pool(), environment_id).await?;
        for c in &mut out {
            c.fields = repo::list_fields(self.pool(), &c.id).await?;
        }
        Ok(out)
    }

    pub async fn list_project_credentials(&self, project_id: &str) -> Result<Vec<Credential>> {
        self.get_project(project_id).await?;
        let mut out = repo::list_headers_for_project(self.pool(), project_id).await?;
        for c in &mut out {
            c.fields = repo::list_fields(self.pool(), &c.id).await?;
        }
        Ok(out)
    }

    pub async fn get_credential(&self, id: &str) -> Result<Credential> {
        let mut c = repo::find_header_by_id(self.pool(), id)
            .await?
            .ok_or_else(|| CoreError::CredentialNotFound(id.to_string()))?;
        c.fields = repo::list_fields(self.pool(), &c.id).await?;
        Ok(c)
    }

    pub async fn resolve_credential(&self, environment_id: &str, id_or_name: &str) -> Result<Credential> {
        if let Some(c) = repo::find_header_by_id(self.pool(), id_or_name).await?
            && c.environment_id == environment_id
        {
            return self.get_credential(&c.id).await;
        }
        let c = repo::find_header_by_name(self.pool(), environment_id, id_or_name)
            .await?
            .ok_or_else(|| CoreError::CredentialNotFound(id_or_name.to_string()))?;
        self.get_credential(&c.id).await
    }

    pub async fn create_credential(&self, input: NewCredential) -> Result<Credential> {
        let env = self.get_environment(&input.environment_id).await?;
        let name = input.name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(CoreError::InvalidName(
                "credential name must be 1-100 characters".into(),
            ));
        }
        if repo::find_header_by_name(self.pool(), &env.id, name)
            .await?
            .is_some()
        {
            return Err(CoreError::AlreadyExists("credential"));
        }
        for spec in input.kind.fields().iter().filter(|s| s.required) {
            if !input
                .fields
                .iter()
                .any(|f| f.field == spec.name && !f.value.expose().is_empty())
            {
                return Err(CoreError::InvalidName(format!(
                    "{} credentials require the field {}",
                    input.kind, spec.name
                )));
            }
        }
        let now = Utc::now();
        let header = Credential {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: env.project_id.clone(),
            environment_id: env.id.clone(),
            kind: input.kind,
            name: name.to_string(),
            note: input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
            fields: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        repo::insert_header(self.pool(), &header).await?;
        for field in input.fields {
            self.store_field(&header, field).await?;
        }
        tracing::info!(credential = %header.name, kind = %header.kind, "credential stored");
        self.get_credential(&header.id).await
    }

    /// Set or replace fields on an existing credential.
    pub async fn update_credential_fields(
        &self,
        id: &str,
        fields: Vec<CredentialFieldInput>,
        note: Option<String>,
    ) -> Result<Credential> {
        let header = self.get_credential(id).await?;
        for field in fields {
            self.store_field(&header, field).await?;
        }
        repo::touch_header(self.pool(), id, note.as_deref()).await?;
        self.get_credential(id).await
    }

    pub async fn delete_credential_field(&self, id: &str, field: &str) -> Result<Credential> {
        let header = self.get_credential(id).await?;
        if header.kind.field(field).is_some_and(|s| s.required) {
            return Err(CoreError::InvalidName(format!(
                "{field} is required for {} credentials",
                header.kind
            )));
        }
        repo::delete_field(self.pool(), id, field).await?;
        self.get_credential(id).await
    }

    pub async fn delete_credential(&self, id: &str) -> Result<()> {
        if !repo::delete_header(self.pool(), id).await? {
            return Err(CoreError::CredentialNotFound(id.to_string()));
        }
        Ok(())
    }

    async fn store_field(&self, header: &Credential, input: CredentialFieldInput) -> Result<()> {
        let spec = header
            .kind
            .field(&input.field)
            .ok_or_else(|| CoreError::InvalidName(format!("{} has no field {}", header.kind, input.field)))?;
        if input.value.expose().is_empty() {
            // Empty means "leave unset / clear".
            let _ = repo::delete_field(self.pool(), &header.id, spec.name).await?;
            return Ok(());
        }
        if spec.secret {
            let row_id = match repo::field_row_id(self.pool(), &header.id, spec.name).await? {
                Some(id) => id,
                None => uuid::Uuid::new_v4().to_string(),
            };
            let sealed = self
                .vault()
                .encrypt(input.value.as_secret_string(), row_id.as_bytes())?;
            repo::upsert_secret_field(self.pool(), &row_id, &header.id, spec.name, &sealed).await?;
        } else {
            repo::upsert_plain_field(self.pool(), &header.id, spec.name, input.value.expose()).await?;
        }
        Ok(())
    }

    /// Run `f` with the plaintext of one secret field. Internal tier: used by
    /// human-initiated commands only; the result must not embed the value.
    pub async fn with_credential_field<R>(
        &self,
        credential_id: &str,
        field: &str,
        f: impl FnOnce(&str) -> R,
    ) -> Result<R> {
        let (row_id, sealed) = repo::find_sealed_field(self.pool(), credential_id, field)
            .await?
            .ok_or_else(|| CoreError::VariableNotFound(format!("{credential_id}.{field}")))?;
        let plaintext = self.vault().decrypt(&sealed, row_id.as_bytes())?;
        let value = SecretValue::from(plaintext);
        Ok(f(value.expose()))
    }

    /// Current 6-digit TOTP code for an `account` credential with a `totp_secret`.
    pub async fn credential_totp(&self, credential_id: &str) -> Result<String> {
        self.with_credential_field(credential_id, "totp_secret", crate::totp::code_now)
            .await?
            .map_err(CoreError::InvalidName)
    }

    pub async fn count_credentials(&self) -> Result<i64> {
        repo::count_credentials(self.pool()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use envenb_vault::InMemoryMasterKeyProvider;
    use sqlx::Row;

    async fn app() -> EnvEnb {
        EnvEnb::open_in_memory(&InMemoryMasterKeyProvider::random())
            .await
            .unwrap()
    }

    fn field(name: &str, value: &str) -> CredentialFieldInput {
        CredentialFieldInput {
            field: name.into(),
            value: SecretValue::new(value),
        }
    }

    #[tokio::test]
    async fn account_round_trip_and_listing_hides_secrets() {
        let app = app().await;
        let p = app.create_project("my-app", None).await.unwrap();
        let e = app.create_environment(&p.id, "staging").await.unwrap();
        let c = app
            .create_credential(NewCredential {
                environment_id: e.id.clone(),
                kind: CredentialKind::Account,
                name: "admin-test".into(),
                note: Some("QA account".into()),
                fields: vec![
                    field("url", "https://staging.example.com/login"),
                    field("username", "qa-user-NEEDLE"),
                    field("password", "pw-NEEDLE"),
                ],
            })
            .await
            .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("https://staging.example.com/login"));
        assert!(!json.contains("NEEDLE"));
        let pw = c.fields.iter().find(|f| f.field == "password").unwrap();
        assert!(pw.secret && pw.value.is_none() && pw.present);

        let got = app
            .with_credential_field(&c.id, "password", |v| v.to_string())
            .await
            .unwrap();
        assert_eq!(got, "pw-NEEDLE");

        // Update keeps the same row id and re-seals.
        app.update_credential_fields(&c.id, vec![field("password", "pw2-NEEDLE")], None)
            .await
            .unwrap();
        assert_eq!(
            app.with_credential_field(&c.id, "password", |v| v.to_string())
                .await
                .unwrap(),
            "pw2-NEEDLE"
        );

        // Nothing in SQLite contains plaintext.
        let rows = sqlx::query("SELECT value, ciphertext FROM credential_fields")
            .fetch_all(app.pool())
            .await
            .unwrap();
        for r in rows {
            let v: Option<String> = r.get("value");
            let ct: Option<Vec<u8>> = r.get("ciphertext");
            assert!(!v.unwrap_or_default().contains("NEEDLE"));
            if let Some(ct) = ct {
                assert!(!ct.windows(6).any(|w| w == b"NEEDLE"));
            }
        }

        assert_eq!(
            app.resolve_credential(&e.id, "ADMIN-TEST").await.unwrap().id,
            c.id
        );
        assert!(
            matches!(
                app.create_credential(NewCredential {
                    environment_id: e.id.clone(),
                    kind: CredentialKind::Ssh,
                    name: "bastion".into(),
                    note: None,
                    fields: vec![field("host", "bastion.example.com")],
                })
                .await,
                Err(CoreError::InvalidName(_))
            ),
            "ssh requires user"
        );
        app.delete_credential(&c.id).await.unwrap();
        assert!(app.list_credentials(&e.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn totp_from_stored_seed() {
        let app = app().await;
        let p = app.create_project("my-app", None).await.unwrap();
        let e = app.create_environment(&p.id, "dev").await.unwrap();
        let c = app
            .create_credential(NewCredential {
                environment_id: e.id.clone(),
                kind: CredentialKind::Account,
                name: "mfa".into(),
                note: None,
                fields: vec![
                    field("username", "u"),
                    field("password", "p"),
                    field("totp_secret", "JBSWY3DPEHPK3PXP"),
                ],
            })
            .await
            .unwrap();
        let code = app.credential_totp(&c.id).await.unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }
}
