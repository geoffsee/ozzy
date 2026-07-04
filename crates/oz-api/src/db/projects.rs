use oz_core::{MemberRole, Project};
use base64::Engine;
use serde::Deserialize;
use worker::D1Database;

use crate::crypto::{generate_and_wrap_dek, WrappedDek};
use crate::error::{internal, AppResult};

#[derive(Deserialize)]
struct ProjectRow {
    id: String,
    slug: String,
    name: String,
    owner_profile_id: String,
}

impl From<ProjectRow> for Project {
    fn from(r: ProjectRow) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            name: r.name,
            owner_profile_id: r.owner_profile_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProjectCryptoRow {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub owner_profile_id: String,
    pub wrapped_dek: Vec<u8>,
    pub dek_wrap_nonce: Vec<u8>,
}

pub async fn create_project(
    db: &D1Database,
    owner_profile_id: &str,
    slug: &str,
    name: &str,
    master_key: &[u8],
) -> AppResult<Project> {
    let id = uuid::Uuid::new_v4().to_string();
    let (_dek, WrappedDek { wrapped, nonce }) = generate_and_wrap_dek(master_key)?;

    db.prepare(
        "INSERT INTO projects (id, slug, name, owner_profile_id, wrapped_dek, dek_wrap_nonce)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&[
        id.clone().into(),
        slug.into(),
        name.into(),
        owner_profile_id.into(),
        wrapped.into(),
        nonce.into(),
    ])?
    .run()
    .await
    .map_err(|e| internal(e))?;

    db.prepare(
        "INSERT INTO project_members (project_id, profile_id, role) VALUES (?1, ?2, 'admin')",
    )
    .bind(&[id.clone().into(), owner_profile_id.into()])?
    .run()
    .await
    .map_err(|e| internal(e))?;

    Ok(Project {
        id,
        slug: slug.to_string(),
        name: name.to_string(),
        owner_profile_id: owner_profile_id.to_string(),
    })
}

pub async fn list_projects_for_profile(
    db: &D1Database,
    profile_id: &str,
) -> AppResult<Vec<Project>> {
    let rows = db
        .prepare(
            "SELECT DISTINCT p.id, p.slug, p.name, p.owner_profile_id
             FROM projects p
             LEFT JOIN project_members pm ON pm.project_id = p.id
             WHERE p.owner_profile_id = ?1 OR pm.profile_id = ?1
             ORDER BY p.created_at DESC",
        )
        .bind(&[profile_id.into()])?
        .all()
        .await
        .map_err(|e| internal(e))?;

    rows.results::<ProjectRow>()
        .map_err(|e| internal(e))?
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>()
        .pipe(Ok)
}

pub async fn get_project_by_slug_for_owner(
    db: &D1Database,
    owner_profile_id: &str,
    slug: &str,
) -> AppResult<Option<Project>> {
    db.prepare(
        "SELECT id, slug, name, owner_profile_id FROM projects
         WHERE owner_profile_id = ?1 AND slug = ?2",
    )
    .bind(&[owner_profile_id.into(), slug.into()])?
    .first::<ProjectRow>(None)
    .await
    .map_err(|e| internal(e))
    .map(|r| r.map(Into::into))
}

pub async fn get_project_by_id(db: &D1Database, id: &str) -> AppResult<Option<Project>> {
    db.prepare(
        "SELECT id, slug, name, owner_profile_id FROM projects WHERE id = ?1",
    )
    .bind(&[id.into()])?
    .first::<ProjectRow>(None)
    .await
    .map_err(|e| internal(e))
    .map(|r| r.map(Into::into))
}

pub async fn get_project_for_profile(
    db: &D1Database,
    profile_id: &str,
    slug: &str,
) -> AppResult<Option<ProjectCryptoRow>> {
    let row = db
        .prepare(
            "SELECT p.id, p.slug, p.name, p.owner_profile_id, p.wrapped_dek, p.dek_wrap_nonce
             FROM projects p
             LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.profile_id = ?1
             WHERE p.slug = ?2 AND (p.owner_profile_id = ?1 OR pm.profile_id IS NOT NULL)
             LIMIT 1",
        )
        .bind(&[profile_id.into(), slug.into()])?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| internal(e))?;

    Ok(row.and_then(|r| {
        Some(ProjectCryptoRow {
            id: r["id"].as_str()?.to_string(),
            slug: r["slug"].as_str()?.to_string(),
            name: r["name"].as_str()?.to_string(),
            owner_profile_id: r["owner_profile_id"].as_str()?.to_string(),
            wrapped_dek: decode_blob(&r["wrapped_dek"])?,
            dek_wrap_nonce: decode_blob(&r["dek_wrap_nonce"])?,
        })
    }))
}

pub async fn get_project_by_slug_any(
    db: &D1Database,
    slug: &str,
) -> AppResult<Option<ProjectCryptoRow>> {
    db.prepare(
        "SELECT id, slug, name, owner_profile_id,
                wrapped_dek, dek_wrap_nonce
         FROM projects WHERE slug = ?1",
    )
    .bind(&[slug.into()])?
    .first::<serde_json::Value>(None)
    .await
    .map_err(|e| internal(e))
    .map(|v| {
        v.and_then(|row| {
            Some(ProjectCryptoRow {
                id: row["id"].as_str()?.to_string(),
                slug: row["slug"].as_str()?.to_string(),
                name: row["name"].as_str()?.to_string(),
                owner_profile_id: row["owner_profile_id"].as_str()?.to_string(),
                wrapped_dek: decode_blob(&row["wrapped_dek"])?,
                dek_wrap_nonce: decode_blob(&row["dek_wrap_nonce"])?,
            })
        })
    })
}

fn decode_blob(v: &serde_json::Value) -> Option<Vec<u8>> {
    if let Some(s) = v.as_str() {
        return base64::engine::general_purpose::STANDARD.decode(s).ok();
    }
    v.as_array().map(|arr| {
        arr.iter()
            .filter_map(|n| n.as_u64().map(|x| x as u8))
            .collect()
    })
}

pub async fn get_member_role(
    db: &D1Database,
    project_id: &str,
    profile_id: &str,
) -> AppResult<Option<MemberRole>> {
    let row = db
        .prepare(
            "SELECT role FROM project_members WHERE project_id = ?1 AND profile_id = ?2",
        )
        .bind(&[project_id.into(), profile_id.into()])?
        .first::<serde_json::Value>(None)
        .await
        .map_err(|e| internal(e))?;

    Ok(row.and_then(|r| r["role"].as_str().and_then(MemberRole::parse)))
}

pub async fn add_member(
    db: &D1Database,
    project_id: &str,
    profile_id: &str,
    role: MemberRole,
) -> AppResult<()> {
    db.prepare(
        "INSERT INTO project_members (project_id, profile_id, role)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(project_id, profile_id) DO UPDATE SET role = excluded.role",
    )
    .bind(&[
        project_id.into(),
        profile_id.into(),
        role.as_str().into(),
    ])?
    .run()
    .await
    .map_err(|e| internal(e))?;
    Ok(())
}

pub async fn remove_member(
    db: &D1Database,
    project_id: &str,
    profile_id: &str,
) -> AppResult<()> {
    db.prepare("DELETE FROM project_members WHERE project_id = ?1 AND profile_id = ?2")
        .bind(&[project_id.into(), profile_id.into()])?
        .run()
        .await
        .map_err(|e| internal(e))?;
    Ok(())
}

pub async fn list_members(
    db: &D1Database,
    project_id: &str,
) -> AppResult<Vec<serde_json::Value>> {
    let rows = db
        .prepare(
            "SELECT pm.profile_id, pm.role, p.login
             FROM project_members pm
             JOIN profiles p ON p.id = pm.profile_id
             WHERE pm.project_id = ?1",
        )
        .bind(&[project_id.into()])?
        .all()
        .await
        .map_err(|e| internal(e))?;
    rows.results::<serde_json::Value>()
        .map_err(|e| internal(e))
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
