//! Supabase database integration for player profiles and characters.
//!
//! Uses the Supabase REST API with the service_role key to bypass Row Level
//! Security. The service_role key must be set in the `SUPABASE_SERVICE_KEY`
//! environment variable.

use authc::Uuid;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// Supabase project URL (hardcoded for Voxtera)
const SUPABASE_URL: &str = "https://gcfavlnisyhdwseuvzpd.supabase.co";

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Row shape for the `profiles` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Row shape for the `characters` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    #[serde(default)]
    pub id: Option<String>,
    pub user_id: String,
    pub name: String,
    #[serde(default)]
    pub body_data: serde_json::Value,
    #[serde(default)]
    pub stats_data: serde_json::Value,
    #[serde(default)]
    pub skill_set_data: serde_json::Value,
    #[serde(default)]
    pub inventory_data: serde_json::Value,
    #[serde(default)]
    pub position_data: serde_json::Value,
    #[serde(default)]
    pub waypoint_data: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Handles all server-side Supabase REST API calls.
pub struct SupabaseDb {
    client: Client,
    service_key: String,
}

impl SupabaseDb {
    /// Create a new Supabase DB client.
    ///
    /// Reads `SUPABASE_SERVICE_KEY` from the environment. Returns `None` if
    /// the variable is not set (Supabase integration disabled).
    pub fn new() -> Option<Self> {
        let service_key = std::env::var("SUPABASE_SERVICE_KEY").ok()?;
        if service_key.is_empty() {
            return None;
        }
        info!("Supabase DB client initialised");
        Some(Self {
            client: Client::new(),
            service_key,
        })
    }

    /// Common headers for every request (service_role bypasses RLS).
    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "apikey",
            reqwest::header::HeaderValue::from_str(&self.service_key).unwrap(),
        );
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.service_key))
                .unwrap(),
        );
        headers
    }

    // -----------------------------------------------------------------------
    // Profiles
    // -----------------------------------------------------------------------

    /// Fetch an existing profile or create one for the given Supabase user id.
    pub async fn get_or_create_profile(
        &self,
        uuid: &Uuid,
        username: &str,
    ) -> Result<Profile, String> {
        // Try to fetch first
        let url = format!(
            "{}/rest/v1/profiles?id=eq.{}&select=*",
            SUPABASE_URL, uuid
        );
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Supabase request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!(%status, %body, "Supabase GET profiles failed");
            return Err(format!("Supabase GET profiles: {status} {body}"));
        }

        let rows: Vec<Profile> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse profiles response: {e}"))?;

        if let Some(profile) = rows.into_iter().next() {
            return Ok(profile);
        }

        // Not found – create
        info!(%uuid, %username, "Creating new Supabase profile");
        let url = format!("{}/rest/v1/profiles", SUPABASE_URL);
        let body = serde_json::json!({
            "id": uuid.to_string(),
            "username": username,
        });
        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Supabase insert profile failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!(%status, %text, "Supabase INSERT profile failed");
            return Err(format!("Supabase INSERT profile: {status} {text}"));
        }

        let rows: Vec<Profile> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse inserted profile: {e}"))?;

        rows.into_iter()
            .next()
            .ok_or_else(|| "Supabase returned empty response".to_string())
    }

    // -----------------------------------------------------------------------
    // Characters
    // -----------------------------------------------------------------------

    /// Fetch all characters belonging to `uuid`.
    pub async fn get_characters(&self, uuid: &Uuid) -> Result<Vec<Character>, String> {
        let url = format!(
            "{}/rest/v1/characters?user_id=eq.{}&select=*",
            SUPABASE_URL, uuid
        );
        let resp = self
            .client
            .get(&url)
            .headers(self.auth_headers())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Supabase GET characters failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!(%status, %body, "Supabase GET characters failed");
            return Err(format!("Supabase GET characters: {status} {body}"));
        }

        resp.json()
            .await
            .map_err(|e| format!("Failed to parse characters response: {e}"))
    }

    /// Upsert (insert or update) a character. If `character_data.id` is set
    /// the row is updated; otherwise a new row is inserted.
    pub async fn save_character(
        &self,
        uuid: &Uuid,
        character_data: &Character,
    ) -> Result<Character, String> {
        let url = format!("{}/rest/v1/characters", SUPABASE_URL);
        let mut payload = serde_json::to_value(character_data)
            .map_err(|e| format!("Failed to serialise character: {e}"))?;

        // Ensure user_id is set correctly
        payload["user_id"] = serde_json::Value::String(uuid.to_string());

        let resp = self
            .client
            .post(&url)
            .headers(self.auth_headers())
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation, resolution=merge-duplicates")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Supabase upsert character failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!(%status, %text, "Supabase upsert character failed");
            return Err(format!("Supabase upsert character: {status} {text}"));
        }

        let rows: Vec<Character> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse saved character: {e}"))?;

        rows.into_iter()
            .next()
            .ok_or_else(|| "Supabase returned empty response".to_string())
    }
}
