/// Financial Graph with pgvector
///
/// Entity embeddings, cosine-similarity search, relation graph,
/// vendor cluster detection, and entity risk scoring.
///
/// SQL setup (run once during migration):
/// ```sql
/// CREATE EXTENSION IF NOT EXISTS vector;
///
/// CREATE TABLE IF NOT EXISTS parties (
///     id           UUID PRIMARY KEY,
///     external_id  TEXT,
///     name         TEXT NOT NULL,
///     org_number   TEXT,
///     vat_number   TEXT,
///     country_code CHAR(2) NOT NULL DEFAULT 'SE',
///     street       TEXT,
///     city         TEXT,
///     postal_code  TEXT,
///     email        TEXT,
///     created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
///     updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// );
///
/// CREATE TABLE IF NOT EXISTS entity_embeddings (
///     id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
///     entity_id  UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
///     embedding  vector(1536) NOT NULL,
///     model      TEXT NOT NULL DEFAULT 'text-embedding-3-small',
///     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// );
/// CREATE INDEX IF NOT EXISTS entity_embeddings_ivfflat_idx
///     ON entity_embeddings USING ivfflat (embedding vector_cosine_ops)
///     WITH (lists = 100);
///
/// CREATE TABLE IF NOT EXISTS entity_relations (
///     id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
///     from_id      UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
///     to_id        UUID NOT NULL REFERENCES parties(id) ON DELETE CASCADE,
///     relation_type TEXT NOT NULL,
///     strength     DOUBLE PRECISION NOT NULL DEFAULT 0.5,
///     evidence     JSONB,
///     created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
///     updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
///     UNIQUE (from_id, to_id, relation_type)
/// );
/// CREATE INDEX IF NOT EXISTS entity_relations_from_idx ON entity_relations(from_id);
/// CREATE INDEX IF NOT EXISTS entity_relations_to_idx   ON entity_relations(to_id);
///
/// CREATE TABLE IF NOT EXISTS vendor_clusters (
///     id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
///     risk_level TEXT NOT NULL,
///     reason     TEXT NOT NULL,
///     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
/// );
///
/// CREATE TABLE IF NOT EXISTS vendor_cluster_members (
///     cluster_id UUID NOT NULL REFERENCES vendor_clusters(id) ON DELETE CASCADE,
///     entity_id  UUID NOT NULL REFERENCES parties(id)         ON DELETE CASCADE,
///     PRIMARY KEY (cluster_id, entity_id)
/// );
/// ```
use anyhow::{anyhow, Context, Result};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::fmt;
use uuid::Uuid;

// Re-use canonical Party from the quickbooks connector.
use crate::connectors::quickbooks::{Address, Party};

// ---------------------------------------------------------------------------
// Public enums & structs (as specified)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationType {
    /// Almost certainly the same legal entity (duplicate / alias)
    SameEntity,
    /// Parent, subsidiary, or known affiliated company
    RelatedParty,
    /// Have transacted together historically
    HistoricallyLinked,
    /// High name/address similarity — potential shell company / fraud
    SuspiciousSimilarity,
}

impl RelationType {
    fn as_str(&self) -> &'static str {
        match self {
            RelationType::SameEntity => "same_entity",
            RelationType::RelatedParty => "related_party",
            RelationType::HistoricallyLinked => "historically_linked",
            RelationType::SuspiciousSimilarity => "suspicious_similarity",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "same_entity" => Some(RelationType::SameEntity),
            "related_party" => Some(RelationType::RelatedParty),
            "historically_linked" => Some(RelationType::HistoricallyLinked),
            "suspicious_similarity" => Some(RelationType::SuspiciousSimilarity),
            _ => None,
        }
    }
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }

    fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.85 => RiskLevel::Critical,
            s if s >= 0.65 => RiskLevel::High,
            s if s >= 0.40 => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySimilarity {
    pub entity: Party,
    pub similarity: f64,
    pub relation_type: Option<RelationType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelation {
    pub id: Uuid,
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub rel_type: RelationType,
    pub strength: f64,
    pub evidence: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorCluster {
    pub cluster_id: Uuid,
    pub entities: Vec<Party>,
    pub risk_level: RiskLevel,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Embedding provider abstraction
// ---------------------------------------------------------------------------

/// Trait to generate a 1536-dim vector from a text fragment.
/// Production impl calls OpenAI text-embedding-3-small; tests may use a stub.
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vector>;
}

/// OpenAI-backed embedding provider.
pub struct OpenAiEmbedder {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[derive(Deserialize)]
struct OpenAiEmbedResponse {
    data: Vec<OpenAiEmbedDatum>,
}

#[derive(Deserialize)]
struct OpenAiEmbedDatum {
    embedding: Vec<f32>,
}

impl OpenAiEmbedder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAiEmbedder {
    async fn embed(&self, text: &str) -> Result<Vector> {
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
            "dimensions": 1536
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("OpenAI embedding request failed")?;

        if !resp.status().is_success() {
            let code = resp.status();
            let msg = resp.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI embedding error {}: {}", code, msg));
        }

        let parsed: OpenAiEmbedResponse =
            resp.json().await.context("Deserializing embedding response")?;

        let floats = parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow!("Empty embedding response from OpenAI"))?;

        Ok(Vector::from(floats))
    }
}

// ---------------------------------------------------------------------------
// FinancialGraph
// ---------------------------------------------------------------------------

pub struct FinancialGraph {
    pool: PgPool,
    embedder: Box<dyn EmbeddingProvider>,
}

impl FinancialGraph {
    /// Construct with an existing pool and embedding provider.
    pub fn new(pool: PgPool, embedder: impl EmbeddingProvider + 'static) -> Self {
        Self {
            pool,
            embedder: Box::new(embedder),
        }
    }

    // -----------------------------------------------------------------------
    // Embedding
    // -----------------------------------------------------------------------

    /// Build the canonical text representation of a Party for embedding.
    fn entity_text(entity: &Party) -> String {
        let mut parts = vec![entity.name.clone()];
        if let Some(ref org) = entity.org_number {
            parts.push(org.clone());
        }
        if let Some(ref vat) = entity.vat_number {
            parts.push(vat.clone());
        }
        if let Some(ref addr) = entity.address {
            if let Some(ref street) = addr.street {
                parts.push(street.clone());
            }
            if let Some(ref city) = addr.city {
                parts.push(city.clone());
            }
            if let Some(ref postal) = addr.postal_code {
                parts.push(postal.clone());
            }
        }
        parts.push(entity.country_code.clone());
        parts.join(" | ")
    }

    /// Generate and persist an embedding for `entity`.
    /// If an embedding already exists it is replaced (upsert by entity_id).
    pub async fn embed_entity(&self, entity: &Party) -> Result<Vector> {
        let text = Self::entity_text(entity);
        let embedding = self.embedder.embed(&text).await?;

        sqlx::query(
            r#"
            INSERT INTO entity_embeddings (id, entity_id, embedding)
            VALUES ($1, $2, $3)
            ON CONFLICT (entity_id)
            DO UPDATE SET embedding = EXCLUDED.embedding,
                          created_at = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(entity.id)
        .bind(&embedding)
        .execute(&self.pool)
        .await
        .context("Upserting entity embedding")?;

        Ok(embedding)
    }

    // -----------------------------------------------------------------------
    // Similarity search
    // -----------------------------------------------------------------------

    /// Find entities similar to `embedding` using cosine similarity (ivfflat index).
    /// Returns up to `limit` results ordered by descending similarity.
    pub async fn find_similar_entities(
        &self,
        embedding: &Vector,
        limit: usize,
    ) -> Result<Vec<EntitySimilarity>> {
        // 1 - cosine_distance = cosine_similarity  (pgvector uses <=> for cosine distance)
        let rows = sqlx::query(
            r#"
            SELECT
                p.id,
                p.external_id,
                p.name,
                p.org_number,
                p.vat_number,
                p.country_code,
                p.street,
                p.city,
                p.postal_code,
                p.email,
                1 - (ee.embedding <=> $1) AS similarity
            FROM entity_embeddings ee
            JOIN parties p ON p.id = ee.entity_id
            WHERE ee.embedding IS NOT NULL
            ORDER BY ee.embedding <=> $1
            LIMIT $2
            "#,
        )
        .bind(embedding)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("find_similar_entities query")?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let similarity: f64 = row.try_get("similarity").unwrap_or(0.0);
            let entity = Self::row_to_party(&row)?;

            let relation_type = if similarity >= 0.97 {
                Some(RelationType::SameEntity)
            } else if similarity >= 0.90 {
                Some(RelationType::RelatedParty)
            } else if similarity >= 0.75 {
                Some(RelationType::SuspiciousSimilarity)
            } else {
                None
            };

            results.push(EntitySimilarity {
                entity,
                similarity,
                relation_type,
            });
        }

        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Relations
    // -----------------------------------------------------------------------

    /// Create or update a directed relation between two entities.
    pub async fn create_relation(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        rel_type: RelationType,
        strength: f64,
    ) -> Result<()> {
        if strength < 0.0 || strength > 1.0 {
            return Err(anyhow!("strength must be in [0.0, 1.0], got {}", strength));
        }

        sqlx::query(
            r#"
            INSERT INTO entity_relations (id, from_id, to_id, relation_type, strength)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (from_id, to_id, relation_type)
            DO UPDATE SET strength    = EXCLUDED.strength,
                          updated_at  = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(from_id)
        .bind(to_id)
        .bind(rel_type.as_str())
        .bind(strength)
        .execute(&self.pool)
        .await
        .context("create_relation")?;

        Ok(())
    }

    /// Retrieve all relations (in both directions) for an entity.
    pub async fn get_relations(&self, entity_id: Uuid) -> Result<Vec<EntityRelation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_id, to_id, relation_type, strength, evidence
            FROM entity_relations
            WHERE from_id = $1 OR to_id = $1
            ORDER BY strength DESC, created_at DESC
            "#,
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("get_relations query")?;

        rows.iter()
            .map(|row| {
                let rel_str: String = row.try_get("relation_type")?;
                let rel_type = RelationType::from_str(&rel_str)
                    .ok_or_else(|| anyhow!("Unknown relation type: {}", rel_str))?;

                Ok(EntityRelation {
                    id: row.try_get("id")?,
                    from_id: row.try_get("from_id")?,
                    to_id: row.try_get("to_id")?,
                    rel_type,
                    strength: row.try_get("strength")?,
                    evidence: row.try_get("evidence").ok(),
                })
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Cluster detection
    // -----------------------------------------------------------------------

    /// Identify clusters of vendors that are suspiciously similar to each other.
    ///
    /// Algorithm:
    /// 1. Pull all entity embeddings.
    /// 2. For each pair, compute cosine similarity.
    /// 3. Union-Find to group entities with similarity ≥ 0.80.
    /// 4. Any group with ≥ 2 members is a cluster; assign risk from max similarity.
    ///
    /// In production, replace the O(n²) pair scan with an ANN query per entity.
    pub async fn detect_vendor_clusters(&self) -> Result<Vec<VendorCluster>> {
        // ---- Step 1: load all embeddings + metadata ----
        let rows = sqlx::query(
            r#"
            SELECT
                p.id        AS entity_id,
                p.name,
                p.external_id,
                p.org_number,
                p.vat_number,
                p.country_code,
                p.street,
                p.city,
                p.postal_code,
                p.email,
                ee.embedding
            FROM entity_embeddings ee
            JOIN parties p ON p.id = ee.entity_id
            WHERE ee.embedding IS NOT NULL
            ORDER BY p.name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("detect_vendor_clusters: loading embeddings")?;

        if rows.len() < 2 {
            return Ok(vec![]);
        }

        let parties: Vec<Party> = rows
            .iter()
            .map(|r| Self::row_to_party(r))
            .collect::<Result<_>>()?;

        let embeddings: Vec<Vector> = rows
            .iter()
            .map(|r| r.try_get::<Vector, _>("embedding").map_err(Into::into))
            .collect::<Result<_>>()?;

        let n = parties.len();
        const CLUSTER_THRESHOLD: f64 = 0.80;

        // ---- Step 2 & 3: Union-Find ----
        let mut parent: Vec<usize> = (0..n).collect();
        let mut rank: Vec<usize> = vec![0; n];
        let mut max_sim: Vec<f64> = vec![0.0; n]; // track max sim within root's group

        fn find(parent: &mut Vec<usize>, x: usize) -> usize {
            if parent[x] != x {
                parent[x] = find(parent, parent[x]);
            }
            parent[x]
        }

        fn union(parent: &mut Vec<usize>, rank: &mut Vec<usize>, x: usize, y: usize) {
            let rx = find(parent, x);
            let ry = find(parent, y);
            if rx == ry {
                return;
            }
            if rank[rx] < rank[ry] {
                parent[rx] = ry;
            } else if rank[rx] > rank[ry] {
                parent[ry] = rx;
            } else {
                parent[ry] = rx;
                rank[rx] += 1;
            }
        }

        for i in 0..n {
            for j in (i + 1)..n {
                let sim = cosine_similarity(&embeddings[i], &embeddings[j]);
                if sim >= CLUSTER_THRESHOLD {
                    let ri = find(&mut parent, i);
                    let rj = find(&mut parent, j);
                    // Update max similarity for both roots
                    if sim > max_sim[ri] {
                        max_sim[ri] = sim;
                    }
                    if sim > max_sim[rj] {
                        max_sim[rj] = sim;
                    }
                    union(&mut parent, &mut rank, i, j);
                }
            }
        }

        // ---- Step 4: collect clusters ----
        let mut group_members: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for i in 0..n {
            let root = find(&mut parent, i);
            group_members.entry(root).or_default().push(i);
        }

        let mut clusters = Vec::new();
        for (root, members) in group_members {
            if members.len() < 2 {
                continue;
            }

            let max_s = max_sim[root];
            let risk_level = RiskLevel::from_score(max_s);

            let reason = Self::cluster_reason(&members.iter().map(|&i| &parties[i]).collect::<Vec<_>>(), max_s);

            // Persist cluster in DB
            let cluster_id = Uuid::new_v4();
            let mut tx = self.pool.begin().await.context("begin cluster tx")?;

            sqlx::query(
                "INSERT INTO vendor_clusters (id, risk_level, reason) VALUES ($1, $2, $3)",
            )
            .bind(cluster_id)
            .bind(risk_level.as_str())
            .bind(&reason)
            .execute(&mut *tx)
            .await
            .context("insert vendor_cluster")?;

            for &mi in &members {
                sqlx::query(
                    "INSERT INTO vendor_cluster_members (cluster_id, entity_id) VALUES ($1, $2) \
                     ON CONFLICT DO NOTHING",
                )
                .bind(cluster_id)
                .bind(parties[mi].id)
                .execute(&mut *tx)
                .await
                .context("insert cluster member")?;
            }

            tx.commit().await.context("commit cluster tx")?;

            clusters.push(VendorCluster {
                cluster_id,
                entities: members.iter().map(|&i| parties[i].clone()).collect(),
                risk_level,
                reason,
            });
        }

        // Sort: Critical first
        clusters.sort_by(|a, b| b.risk_level.partial_cmp(&a.risk_level).unwrap_or(std::cmp::Ordering::Equal));
        Ok(clusters)
    }

    // -----------------------------------------------------------------------
    // Risk scoring
    // -----------------------------------------------------------------------

    /// Compute a risk score [0.0, 1.0] for an entity based on its position
    /// in the relation graph and embedding neighbourhood.
    ///
    /// Scoring factors:
    /// - `suspicious_similarity` relations  (+0.30 per relation, cap 0.60)
    /// - `same_entity` relations            (+0.20 per relation, cap 0.40)
    /// - membership in a High/Critical cluster (+0.25 / +0.40)
    /// - weighted sum, clamped to [0.0, 1.0]
    pub async fn entity_risk_score(&self, entity_id: Uuid) -> Result<f64> {
        let relations = self.get_relations(entity_id).await?;

        let mut score = 0.0f64;

        // Count risky relations
        let suspicious_count = relations
            .iter()
            .filter(|r| r.rel_type == RelationType::SuspiciousSimilarity)
            .count() as f64;

        let same_entity_count = relations
            .iter()
            .filter(|r| r.rel_type == RelationType::SameEntity)
            .count() as f64;

        score += (suspicious_count * 0.30).min(0.60);
        score += (same_entity_count * 0.20).min(0.40);

        // Add relation-strength weighted component
        let strength_contrib: f64 = relations
            .iter()
            .filter(|r| {
                r.rel_type == RelationType::SuspiciousSimilarity
                    || r.rel_type == RelationType::SameEntity
            })
            .map(|r| r.strength)
            .sum::<f64>()
            .min(0.30);
        score += strength_contrib * 0.10;

        // Check cluster membership
        let cluster_row = sqlx::query(
            r#"
            SELECT vc.risk_level
            FROM vendor_cluster_members vcm
            JOIN vendor_clusters vc ON vc.id = vcm.cluster_id
            WHERE vcm.entity_id = $1
            ORDER BY
                CASE vc.risk_level
                    WHEN 'critical' THEN 4
                    WHEN 'high'     THEN 3
                    WHEN 'medium'   THEN 2
                    ELSE 1
                END DESC
            LIMIT 1
            "#,
        )
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
        .context("entity_risk_score: cluster lookup")?;

        if let Some(row) = cluster_row {
            let level: String = row.try_get("risk_level").unwrap_or_default();
            score += match level.as_str() {
                "critical" => 0.40,
                "high" => 0.25,
                "medium" => 0.10,
                _ => 0.0,
            };
        }

        Ok(score.min(1.0))
    }

    // -----------------------------------------------------------------------
    // Utility helpers
    // -----------------------------------------------------------------------

    fn row_to_party(row: &PgRow) -> Result<Party> {
        let street: Option<String> = row.try_get("street").ok();
        let city: Option<String> = row.try_get("city").ok();
        let postal_code: Option<String> = row.try_get("postal_code").ok();
        let country_code: String = row.try_get("country_code").unwrap_or_else(|_| "SE".to_string());

        let address = if street.is_some() || city.is_some() || postal_code.is_some() {
            Some(Address {
                street,
                city,
                postal_code,
                country_code: country_code.clone(),
            })
        } else {
            None
        };

        Ok(Party {
            id: row.try_get("entity_id").or_else(|_| row.try_get("id"))?,
            external_id: row.try_get("external_id").ok().flatten(),
            name: row.try_get("name")?,
            org_number: row.try_get("org_number").ok().flatten(),
            vat_number: row.try_get("vat_number").ok().flatten(),
            country_code,
            address,
            email: row.try_get("email").ok().flatten(),
        })
    }

    fn cluster_reason(members: &[&Party], max_similarity: f64) -> String {
        let names: Vec<&str> = members.iter().map(|p| p.name.as_str()).collect();
        let pct = (max_similarity * 100.0).round() as u32;

        if max_similarity >= 0.97 {
            format!(
                "Entities [{names}] share ≥{pct}% name/address similarity — likely duplicate registrations",
                names = names.join(", "),
                pct = pct,
            )
        } else if max_similarity >= 0.90 {
            format!(
                "Entities [{names}] are {pct}% similar — potential related-party transactions",
                names = names.join(", "),
                pct = pct,
            )
        } else {
            format!(
                "Entities [{names}] cluster at {pct}% similarity — review for shell-company indicators",
                names = names.join(", "),
                pct = pct,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Cosine similarity between two pgvector Vectors
// ---------------------------------------------------------------------------

fn cosine_similarity(a: &Vector, b: &Vector) -> f64 {
    let av: &[f32] = a.as_slice();
    let bv: &[f32] = b.as_slice();

    if av.len() != bv.len() || av.is_empty() {
        return 0.0;
    }

    let dot: f64 = av
        .iter()
        .zip(bv.iter())
        .map(|(&x, &y)| x as f64 * y as f64)
        .sum();

    let norm_a: f64 = av.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = bv.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct StubEmbedder;

    #[async_trait::async_trait]
    impl EmbeddingProvider for StubEmbedder {
        async fn embed(&self, text: &str) -> Result<Vector> {
            // Deterministic stub: hash text chars into a 1536-dim float vector
            let mut v = vec![0.0f32; 1536];
            for (i, b) in text.bytes().enumerate() {
                v[i % 1536] += b as f32 / 255.0;
            }
            // Normalise
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
            v.iter_mut().for_each(|x| *x /= norm);
            Ok(Vector::from(v))
        }
    }

    #[test]
    fn cosine_similarity_identical() {
        let v: Vec<f32> = (0..1536).map(|i| (i as f32) / 1536.0).collect();
        let a = Vector::from(v.clone());
        let b = Vector::from(v);
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "sim = {}", sim);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let mut av = vec![0.0f32; 1536];
        let mut bv = vec![0.0f32; 1536];
        av[0] = 1.0;
        bv[1] = 1.0;
        let sim = cosine_similarity(&Vector::from(av), &Vector::from(bv));
        assert!((sim - 0.0).abs() < 1e-6, "sim = {}", sim);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let av: Vec<f32> = vec![1.0; 1536];
        let bv: Vec<f32> = vec![-1.0; 1536];
        let sim = cosine_similarity(&Vector::from(av), &Vector::from(bv));
        assert!((sim + 1.0).abs() < 1e-6, "sim = {}", sim);
    }

    #[test]
    fn relation_type_roundtrip() {
        for rt in &[
            RelationType::SameEntity,
            RelationType::RelatedParty,
            RelationType::HistoricallyLinked,
            RelationType::SuspiciousSimilarity,
        ] {
            let s = rt.as_str();
            let parsed = RelationType::from_str(s).expect("round-trip failed");
            assert_eq!(&parsed, rt);
        }
    }

    #[test]
    fn risk_level_from_score_buckets() {
        assert_eq!(RiskLevel::from_score(0.90), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_score(0.70), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(0.50), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(0.20), RiskLevel::Low);
    }

    #[test]
    fn entity_text_includes_all_fields() {
        let party = Party {
            id: Uuid::new_v4(),
            external_id: None,
            name: "ACME AB".to_string(),
            org_number: Some("556000-0001".to_string()),
            vat_number: Some("SE556000000101".to_string()),
            country_code: "SE".to_string(),
            address: Some(Address {
                street: Some("Testgatan 1".to_string()),
                city: Some("Stockholm".to_string()),
                postal_code: Some("11111".to_string()),
                country_code: "SE".to_string(),
            }),
            email: None,
        };
        let text = FinancialGraph::entity_text(&party);
        assert!(text.contains("ACME AB"));
        assert!(text.contains("556000-0001"));
        assert!(text.contains("Stockholm"));
        assert!(text.contains("SE"));
    }

    #[test]
    fn cluster_reason_critical() {
        let p = Party {
            id: Uuid::new_v4(),
            external_id: None,
            name: "Shell Corp AB".to_string(),
            org_number: None,
            vat_number: None,
            country_code: "SE".to_string(),
            address: None,
            email: None,
        };
        let reason = FinancialGraph::cluster_reason(&[&p], 0.98);
        assert!(reason.contains("duplicate"));
    }

    #[tokio::test]
    async fn stub_embedder_normalised() {
        let embedder = StubEmbedder;
        let v = embedder.embed("hello world").await.unwrap();
        let norm: f32 = v.as_slice().iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm = {}", norm);
    }
}
