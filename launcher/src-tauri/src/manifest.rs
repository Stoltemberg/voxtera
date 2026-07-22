use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Read},
    path::{Component, Path},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::Channel;

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const ARCHIVE_NAME: &str = "Voxtera-windows-x64.zip";
pub const GAME_EXECUTABLE: &str = "Voxtera.exe";
const REQUIRED_PRESERVED_PATHS: [&str; 3] = ["userdata/", "screenshots/", "settings/"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub schema_version: u32,
    pub version: Version,
    pub channel: Channel,
    pub archive: ArchiveMetadata,
    pub executable: String,
    pub files: Vec<ManagedFile>,
    pub preserved_paths: Vec<String>,
    pub minimum_launcher_version: Version,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveMetadata {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ManifestError {
    message: String,
}

impl ManifestError {
    pub fn code(&self) -> &'static str { "manifest_contract" }

    fn contract(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Manifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(ManifestError::contract(
                "Versão de schema do manifesto incompatível.",
            ));
        }
        if self.channel != Channel::Preview {
            return Err(ManifestError::contract(
                "O manifesto deve usar o canal Preview.",
            ));
        }
        if self.archive.name != ARCHIVE_NAME || !is_sha256(&self.archive.sha256) {
            return Err(ManifestError::contract("Metadados de archive inválidos."));
        }
        if self.executable != GAME_EXECUTABLE || !is_normalized_relative(&self.executable, false) {
            return Err(ManifestError::contract("Executável do manifesto inválido."));
        }

        let current_launcher = Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("the launcher package version must be semantic");
        if self.minimum_launcher_version > current_launcher {
            return Err(ManifestError::contract(
                "O manifesto exige uma versão mais nova do launcher.",
            ));
        }

        let mut preserved = HashSet::new();
        for path in &self.preserved_paths {
            if !is_normalized_relative(path, true) || !preserved.insert(path.to_ascii_lowercase()) {
                return Err(ManifestError::contract(
                    "Regra de caminho preservado inválida.",
                ));
            }
        }
        if REQUIRED_PRESERVED_PATHS
            .iter()
            .any(|required| !preserved.contains(*required))
        {
            return Err(ManifestError::contract(
                "O manifesto não preserva os dados obrigatórios.",
            ));
        }

        let mut managed = HashSet::new();
        let mut previous: Option<&str> = None;
        for file in &self.files {
            let lower_path = file.path.to_ascii_lowercase();
            if !is_normalized_relative(&file.path, false)
                || !is_sha256(&file.sha256)
                || !managed.insert(lower_path.clone())
                || preserved
                    .iter()
                    .any(|rule| lower_path.starts_with(rule.as_str()))
                || previous.is_some_and(|prior| prior > file.path.as_str())
            {
                return Err(ManifestError::contract(
                    "Arquivo gerenciado inválido no manifesto.",
                ));
            }
            previous = Some(&file.path);
        }
        if !managed.contains(&self.executable.to_ascii_lowercase()) {
            return Err(ManifestError::contract(
                "O executável não consta nos arquivos gerenciados.",
            ));
        }
        Ok(())
    }
}

pub fn build_manifest(
    input: &Path,
    archive: &Path,
    version: Version,
    minimum_launcher_version: Version,
) -> Result<Manifest, ManifestError> {
    let root_metadata = std::fs::symlink_metadata(input)
        .map_err(|_| ManifestError::contract("Diretório de entrada inacessível."))?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        return Err(ManifestError::contract(
            "O diretório de entrada deve ser uma pasta real.",
        ));
    }
    let root = input
        .canonicalize()
        .map_err(|_| ManifestError::contract("Diretório de entrada inacessível."))?;

    if archive.file_name().and_then(|name| name.to_str()) != Some(ARCHIVE_NAME) {
        return Err(ManifestError::contract(
            "Nome do archive fora do contrato de release.",
        ));
    }
    let archive_metadata =
        std::fs::metadata(archive).map_err(|_| ManifestError::contract("Archive inacessível."))?;
    if !archive_metadata.is_file() {
        return Err(ManifestError::contract(
            "O archive deve ser um arquivo regular.",
        ));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(&root).follow_links(false) {
        let entry =
            entry.map_err(|_| ManifestError::contract("Falha ao percorrer a distribuição."))?;
        if entry.path() == root {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(ManifestError::contract(
                "Links não são permitidos na distribuição.",
            ));
        }
        let canonical = entry
            .path()
            .canonicalize()
            .map_err(|_| ManifestError::contract("Entrada inacessível na distribuição."))?;
        let relative = canonical
            .strip_prefix(&root)
            .map_err(|_| ManifestError::contract("Entrada fora do diretório de distribuição."))?;
        let normalized = normalize_relative(relative)?;
        if is_excluded(&normalized) || entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(ManifestError::contract(
                "A distribuição contém uma entrada insegura.",
            ));
        }
        files.push(ManagedFile {
            path: normalized,
            size: entry
                .metadata()
                .map_err(|_| ManifestError::contract("Arquivo gerenciado inacessível."))?
                .len(),
            sha256: hash_file(entry.path())?,
        });
    }
    files.sort_unstable_by(|left, right| left.path.cmp(&right.path));

    let manifest = Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        version,
        channel: Channel::Preview,
        archive: ArchiveMetadata {
            name: ARCHIVE_NAME.to_owned(),
            size: archive_metadata.len(),
            sha256: hash_file(archive)?,
        },
        executable: GAME_EXECUTABLE.to_owned(),
        files,
        preserved_paths: REQUIRED_PRESERVED_PATHS
            .iter()
            .map(|path| (*path).to_owned())
            .collect(),
        minimum_launcher_version,
    };
    manifest.validate()?;
    Ok(manifest)
}

pub fn manifest_json(manifest: &Manifest) -> Result<String, ManifestError> {
    manifest.validate()?;
    let mut json = serde_json::to_string_pretty(manifest)
        .map_err(|_| ManifestError::contract("Não foi possível serializar o manifesto."))?;
    json.push('\n');
    Ok(json)
}

fn hash_file(path: &Path) -> Result<String, ManifestError> {
    let file =
        File::open(path).map_err(|_| ManifestError::contract("Arquivo inacessível para hash."))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| ManifestError::contract("Falha ao calcular hash do arquivo."))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn normalize_relative(path: &Path) -> Result<String, ManifestError> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                parts.push(part.to_str().ok_or_else(|| {
                    ManifestError::contract("Caminho não Unicode na distribuição.")
                })?)
            },
            _ => return Err(ManifestError::contract("Caminho inseguro na distribuição.")),
        }
    }
    let normalized = parts.join("/");
    if !is_normalized_relative(&normalized, false) {
        return Err(ManifestError::contract("Caminho inseguro na distribuição."));
    }
    Ok(normalized)
}

fn is_sha256(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_normalized_relative(path: &str, directory_rule: bool) -> bool {
    if path.is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path.starts_with('/')
        || path.starts_with("//")
        || (!directory_rule && path.ends_with('/'))
        || (directory_rule && !path.ends_with('/'))
    {
        return false;
    }
    let body = if directory_rule {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    };
    !body.is_empty()
        && body
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn is_excluded(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let root = lower.split('/').next().unwrap_or_default();
    matches!(
        root,
        "cache" | "logs" | "launcher" | "userdata" | "screenshots" | "settings"
    ) || lower.ends_with(".tmp")
        || lower.ends_with(".part")
        || lower.ends_with(".partial")
        || lower.ends_with(".download")
        || lower
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with('~'))
}
