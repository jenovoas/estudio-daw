use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

const MAX_BUNDLE_FILES: usize = 16_384;
const MAX_BUNDLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TrustedCode {
    canonical_path: PathBuf,
    sha256: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct TrustRegistry {
    schema_version: u32,
    trusted: Vec<TrustedCode>,
    wine_prefixes: Vec<LocalWinePrefix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LocalWinePrefix {
    canonical_executable: PathBuf,
    prefix: PathBuf,
}

pub async fn authorize_external_code(
    app: &tauri::AppHandle,
    path: &Path,
    code_kind: &str,
) -> Result<PathBuf, String> {
    let identity = identify(path)?;
    let registry_path = registry_path(app)?;
    let mut registry = load_registry(&registry_path)?;
    if registry_trusts_identity(&registry, &identity) {
        return Ok(identity.canonical_path);
    }

    let changed = registry
        .trusted
        .iter()
        .any(|trusted| trusted.canonical_path == identity.canonical_path);
    let explanation = if changed {
        "El contenido de esta ubicación cambió desde su aprobación anterior."
    } else {
        "Este proyecto solicita ejecutar código externo que todavía no aprobaste."
    };
    let message = format!(
        "{explanation}\n\nTipo: {code_kind}\nRuta real: {}\nSHA-256: {}\n\n¿Confiar en este código en este equipo y permitir su ejecución?",
        identity.canonical_path.display(),
        identity.sha256
    );
    let approved = request_approval(app, message).await?;
    if !approved {
        return Err(format!(
            "se canceló porque no se aprobó el {code_kind} {}",
            identity.canonical_path.display()
        ));
    }

    registry
        .trusted
        .retain(|trusted| trusted.canonical_path != identity.canonical_path);
    registry.trusted.push(identity.clone());
    registry.schema_version = 1;
    save_registry(&registry_path, &registry)?;

    // Verify that the path did not resolve to a different object while the
    // native confirmation dialog was open.
    let after_confirmation = identify(&identity.canonical_path)?;
    if after_confirmation != identity {
        return Err("el código cambió durante la aprobación; vuelve a revisarlo".into());
    }
    Ok(identity.canonical_path)
}

async fn request_approval(app: &tauri::AppHandle, message: String) -> Result<bool, String> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.dialog()
        .message(message)
        .title("Aprobar código externo")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .show(move |approved| {
            let _ = sender.send(approved);
        });
    tauri::async_runtime::spawn_blocking(move || receiver.recv())
        .await
        .map_err(|error| format!("falló la espera del diálogo de confianza: {error}"))?
        .map_err(|error| format!("se cerró el diálogo de confianza: {error}"))
}

pub fn revoke_external_code(app: &tauri::AppHandle, path: &Path) -> Result<(), String> {
    let registry_path = registry_path(app)?;
    let mut registry = load_registry(&registry_path)?;
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    revoke_registry_path(&mut registry, &canonical);
    save_registry(&registry_path, &registry)
}

pub fn local_wine_prefix(
    app: &tauri::AppHandle,
    executable: &Path,
) -> Result<Option<PathBuf>, String> {
    let canonical_executable = fs::canonicalize(executable)
        .map_err(|error| format!("no se pudo resolver {}: {error}", executable.display()))?;
    let registry = load_registry(&registry_path(app)?)?;
    Ok(registry_wine_prefix(&registry, &canonical_executable))
}

pub fn set_local_wine_prefix(
    app: &tauri::AppHandle,
    executable: &Path,
    prefix: Option<&Path>,
) -> Result<(), String> {
    let canonical_executable = fs::canonicalize(executable)
        .map_err(|error| format!("no se pudo resolver {}: {error}", executable.display()))?;
    let prefix = prefix
        .map(|prefix| {
            let canonical = fs::canonicalize(prefix).map_err(|error| {
                format!(
                    "no se pudo resolver el prefijo Wine {}: {error}",
                    prefix.display()
                )
            })?;
            if !canonical.is_dir() {
                return Err(format!("{} no es una carpeta", canonical.display()));
            }
            Ok(canonical)
        })
        .transpose()?;

    let registry_path = registry_path(app)?;
    let mut registry = load_registry(&registry_path)?;
    set_registry_wine_prefix(&mut registry, canonical_executable, prefix);
    registry.schema_version = 1;
    save_registry(&registry_path, &registry)
}

fn registry_wine_prefix(registry: &TrustRegistry, executable: &Path) -> Option<PathBuf> {
    registry
        .wine_prefixes
        .iter()
        .find(|entry| entry.canonical_executable == executable)
        .map(|entry| entry.prefix.clone())
}

fn set_registry_wine_prefix(
    registry: &mut TrustRegistry,
    executable: PathBuf,
    prefix: Option<PathBuf>,
) {
    registry
        .wine_prefixes
        .retain(|entry| entry.canonical_executable != executable);
    if let Some(prefix) = prefix {
        registry.wine_prefixes.push(LocalWinePrefix {
            canonical_executable: executable,
            prefix,
        });
    }
}

fn revoke_registry_path(registry: &mut TrustRegistry, path: &Path) -> bool {
    let before = registry.trusted.len();
    registry
        .trusted
        .retain(|trusted| trusted.canonical_path != path);
    registry.trusted.len() != before
}

fn registry_trusts_identity(registry: &TrustRegistry, identity: &TrustedCode) -> bool {
    registry.trusted.iter().any(|trusted| trusted == identity)
}

fn registry_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("trusted-external-code.json"))
        .map_err(|error| format!("no se pudo resolver la configuración local: {error}"))
}

fn identify(path: &Path) -> Result<TrustedCode, String> {
    let canonical_path = fs::canonicalize(path)
        .map_err(|error| format!("no se pudo resolver {}: {error}", path.display()))?;
    let sha256 = hash_path(&canonical_path)?;
    Ok(TrustedCode {
        canonical_path,
        sha256,
    })
}

fn hash_path(root: &Path) -> Result<String, String> {
    let metadata = fs::metadata(root)
        .map_err(|error| format!("no se pudo inspeccionar {}: {error}", root.display()))?;
    if metadata.is_file() {
        return hash_file(root);
    }
    if !metadata.is_dir() {
        return Err(format!(
            "{} no es un archivo ejecutable ni un bundle",
            root.display()
        ));
    }

    let mut pending = vec![(root.to_path_buf(), PathBuf::new())];
    let mut visited = HashSet::new();
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    while let Some((directory, relative_directory)) = pending.pop() {
        let real_directory = fs::canonicalize(&directory)
            .map_err(|error| format!("no se pudo resolver {}: {error}", directory.display()))?;
        if !real_directory.starts_with(root) {
            return Err("el bundle contiene un enlace fuera de su carpeta".into());
        }
        if !visited.insert(real_directory.clone()) {
            continue;
        }
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("no se pudo inspeccionar {}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let relative = relative_directory.join(entry.file_name());
            let real_path = fs::canonicalize(&path)
                .map_err(|error| format!("no se pudo resolver {}: {error}", path.display()))?;
            if !real_path.starts_with(root) {
                return Err(format!(
                    "el bundle contiene un enlace fuera de su carpeta: {}",
                    path.display()
                ));
            }
            let metadata = fs::metadata(&real_path).map_err(|error| error.to_string())?;
            if metadata.is_dir() {
                pending.push((real_path, relative));
            } else if metadata.is_file() {
                total_bytes = total_bytes.saturating_add(metadata.len());
                if total_bytes > MAX_BUNDLE_BYTES || files.len() >= MAX_BUNDLE_FILES {
                    return Err("el bundle supera los límites de inspección segura".into());
                }
                files.push((relative, real_path));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty() {
        return Err("el bundle no contiene archivos".into());
    }

    let mut digest = Sha256::new();
    for (relative, path) in files {
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update([0]);
        let mut file = File::open(&path)
            .map_err(|error| format!("no se pudo leer {}: {error}", path.display()))?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|error| format!("no se pudo leer {}: {error}", path.display()))?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("no se pudo leer {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("no se pudo leer {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn load_registry(path: &Path) -> Result<TrustRegistry, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("el registro local de confianza no es válido: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(TrustRegistry {
            schema_version: 1,
            trusted: Vec::new(),
            wine_prefixes: Vec::new(),
        }),
        Err(error) => Err(format!("no se pudo leer el registro de confianza: {error}")),
    }
}

fn save_registry(path: &Path, registry: &TrustRegistry) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "la ruta de configuración no tiene carpeta".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("no se pudo crear la configuración local: {error}"))?;
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(registry)
        .map_err(|error| format!("no se pudo serializar la confianza local: {error}"))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("no se pudo crear el registro temporal: {error}"))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("no se pudo escribir el registro de confianza: {error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("no se pudo publicar el registro de confianza: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_identity_changes_when_contents_change() {
        let directory = std::env::temp_dir().join(format!(
            "estudio-daw-trust-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("instrument.exe");
        fs::write(&executable, b"first version").unwrap();
        let first = identify(&executable).unwrap();
        let mut registry = TrustRegistry::default();
        registry.trusted.push(first.clone());
        assert!(registry_trusts_identity(&registry, &first));
        fs::write(&executable, b"second version").unwrap();
        let second = identify(&executable).unwrap();
        assert_eq!(first.canonical_path, second.canonical_path);
        assert_ne!(first.sha256, second.sha256);
        assert!(!registry_trusts_identity(&registry, &second));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn bundle_identity_changes_when_member_contents_change() {
        let directory = std::env::temp_dir().join(format!(
            "estudio-daw-bundle-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let bundle = directory.join("Instrument.vst3");
        fs::create_dir_all(&bundle).unwrap();
        let binary = bundle.join("Contents").join("x86_64-linux");
        fs::create_dir_all(&binary).unwrap();
        let module = binary.join("Instrument.so");
        fs::write(&module, b"first version").unwrap();
        let first = identify(&bundle).unwrap();
        let mut registry = TrustRegistry::default();
        registry.trusted.push(first.clone());
        assert!(registry_trusts_identity(&registry, &first));
        fs::write(&module, b"second version").unwrap();
        let second = identify(&bundle).unwrap();
        assert_ne!(first.sha256, second.sha256);
        assert!(!registry_trusts_identity(&registry, &second));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn registry_round_trips_and_revokes_canonical_path() {
        let directory = std::env::temp_dir().join(format!(
            "estudio-daw-registry-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("trusted.json");
        let registry = TrustRegistry {
            schema_version: 1,
            trusted: vec![TrustedCode {
                canonical_path: directory.join("instrument.exe"),
                sha256: "abc123".into(),
            }],
            wine_prefixes: vec![LocalWinePrefix {
                canonical_executable: directory.join("instrument.exe"),
                prefix: directory.join(".wine"),
            }],
        };
        save_registry(&path, &registry).unwrap();
        let mut loaded = load_registry(&path).unwrap();
        assert_eq!(loaded.trusted, registry.trusted);
        assert!(revoke_registry_path(
            &mut loaded,
            &directory.join("instrument.exe")
        ));
        assert!(loaded.trusted.is_empty());
        assert_eq!(loaded.wine_prefixes, registry.wine_prefixes);
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_target_share_the_canonical_identity() {
        use std::os::unix::fs::symlink;

        let directory = std::env::temp_dir().join(format!(
            "estudio-daw-symlink-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("instrument.exe");
        let alias = directory.join("alias.exe");
        fs::write(&target, b"instrument").unwrap();
        symlink(&target, &alias).unwrap();
        assert_eq!(identify(&target).unwrap(), identify(&alias).unwrap());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_binary_is_rejected_before_approval() {
        let path = std::env::temp_dir().join(format!(
            "estudio-daw-no-such-binary-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        assert!(identify(&path).is_err());
    }

    #[test]
    fn local_wine_prefix_is_independent_and_can_be_reset() {
        let mut registry = TrustRegistry::default();
        let executable = PathBuf::from("/opt/instrument.exe");
        let prefix = PathBuf::from("/home/user/.wine-instrument");
        set_registry_wine_prefix(&mut registry, executable.clone(), Some(prefix.clone()));
        assert_eq!(registry_wine_prefix(&registry, &executable), Some(prefix));
        set_registry_wine_prefix(&mut registry, executable.clone(), None);
        assert_eq!(registry_wine_prefix(&registry, &executable), None);
    }

    #[test]
    fn old_registry_without_local_wine_settings_still_loads() {
        let old = r#"{"schema_version":1,"trusted":[]}"#;
        let registry: TrustRegistry = serde_json::from_str(old).unwrap();
        assert!(registry.trusted.is_empty());
        assert!(registry.wine_prefixes.is_empty());
    }
}
