//! Update packages: a setup that carries only the files that changed.
//!
//! A new version of a product usually changes a handful of files, so a project
//! can build a second setup from the release it replaces. That setup carries
//! only the files whose bytes differ and a plan naming the ones it expects to
//! find on disk, so it is a fraction of the full setup -- and it installs over
//! exactly the release it was built from, which the runtime checks before it
//! writes anything.
//!
//! The builder does not read archives itself. Both the release an update is
//! built from and the release it installs are expanded with the same runtime
//! stub that will unpack the update on the machine, so the comparison is made
//! through the code path that has to agree with it.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::install::MANIFEST_NAME;
use super::{BundleIndex, PayloadFormat};

/// The bundle entry an update package carries its plan under.
pub(crate) const UPDATE_PLAN: &str = "update/plan.json";

/// One release's files as an update compares two of them: each path with the
/// byte count and the SHA-256 of what the release really holds there.
type Tree = BTreeMap<String, (u64, String)>;

/// One file an update expects to find on disk exactly as it was built.
pub(crate) struct KeptFile {
    /// The path inside the installation, with `/` as its separator.
    pub(crate) path: String,
    pub(crate) size: u64,
    /// The SHA-256 of the file, in hexadecimal.
    pub(crate) sha256: String,
}

/// What an update package installs over.
pub(crate) struct UpdatePlan {
    /// The archive this update was built from, named for whoever reads an error.
    pub(crate) from: String,
    /// The version this update installs.
    pub(crate) version: String,
    /// The files the destination has to hold already. They are not in the
    /// payload, and the update refuses to run without them.
    pub(crate) keep: Vec<KeptFile>,
}

/// What building an update package came to, for the caller to report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UpdateSummary {
    /// Files the update expects to find, and so does not carry.
    pub kept: usize,
    /// Files the update carries.
    pub changed: usize,
    /// The size of the archive it carries them in.
    pub archive_size: u64,
}

impl UpdatePlan {
    fn to_json(&self) -> Value {
        json!({
            "from": self.from,
            "version": self.version,
            "keep": self
                .keep
                .iter()
                .map(|file| {
                    json!({
                        "path": file.path,
                        "size": file.size,
                        "sha256": file.sha256,
                    })
                })
                .collect::<Vec<_>>(),
        })
    }

    /// The plan as the bundle stores it.
    pub(crate) fn to_json_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.to_json()).context("failed to encode the update plan")
    }

    /// The plan a setup carries, or `None` for an ordinary setup.
    pub(crate) fn read(bundle: &BundleIndex) -> Result<Option<Self>> {
        if !bundle.contains(UPDATE_PLAN) {
            return Ok(None);
        }
        let data = bundle.read_file(UPDATE_PLAN)?;
        let value: Value = serde_json::from_slice(&data)
            .with_context(|| format!("{UPDATE_PLAN} is not a JSON document"))?;
        let mut keep = Vec::new();
        for entry in value["keep"].as_array().into_iter().flatten() {
            keep.push(KeptFile {
                path: entry["path"]
                    .as_str()
                    .context("an update plan entry names no path")?
                    .to_string(),
                size: entry["size"]
                    .as_u64()
                    .context("an update plan entry states no size")?,
                sha256: entry["sha256"]
                    .as_str()
                    .context("an update plan entry carries no digest")?
                    .to_string(),
            });
        }
        Ok(Some(Self {
            from: value["from"]
                .as_str()
                .unwrap_or("an earlier release")
                .to_string(),
            version: value["version"].as_str().unwrap_or_default().to_string(),
            keep,
        }))
    }

    /// Refuses a destination this update does not fit, before anything is written.
    ///
    /// A file the update expects and does not find has one honest answer: the
    /// machine holds a different release, and the full setup is what installs
    /// over it. Half an answer -- deploying what does fit -- would leave a
    /// product that is neither version.
    pub(crate) fn verify(&self, destination: &Path, product: &str) -> Result<()> {
        let mut problems: Vec<String> = Vec::new();
        for file in &self.keep {
            let path = destination.join(relative_path(&file.path));
            let metadata = match std::fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => {
                    problems.push(format!("{} is missing", file.path));
                    continue;
                }
            };
            if !metadata.is_file() {
                problems.push(format!("{} is not a file", file.path));
                continue;
            }
            if metadata.len() != file.size {
                problems.push(format!(
                    "{} holds {} bytes where this update expects {}",
                    file.path,
                    metadata.len(),
                    file.size
                ));
                continue;
            }
            if !super::net::sha256_file(&path)?.eq_ignore_ascii_case(&file.sha256) {
                problems.push(format!(
                    "{} is not the file {} carries",
                    file.path, self.from
                ));
            }
        }
        if problems.is_empty() {
            return Ok(());
        }
        const SHOWN: usize = 4;
        let listed = problems
            .iter()
            .take(SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        let rest = problems.len().saturating_sub(SHOWN);
        let more = if rest > 0 {
            format!(" (and {rest} more like it)")
        } else {
            String::new()
        };
        bail!(
            "this update package installs only over the release it was built from ({from}), \
             and this machine does not hold it: {listed}{more}. \
             Run the full {product} setup on this machine instead.",
            from = self.from,
        )
    }

    /// The files the update leaves where they are, which the installation still owns.
    pub(crate) fn kept_paths(&self) -> Vec<PathBuf> {
        self.keep
            .iter()
            .map(|file| relative_path(&file.path))
            .collect()
    }

    /// Whether a destination holds an installation this update could follow.
    pub(crate) fn requires_installed_product(
        &self,
        destination: &Path,
        product: &str,
    ) -> Result<()> {
        if destination.join(MANIFEST_NAME).is_file() {
            return Ok(());
        }
        bail!(
            "this setup is an update package built from {from}: it installs only over an existing \
             {product} {version} installation. Run the full setup on a machine that has none.",
            from = self.from,
            version = self.version
        )
    }
}

/// The path an update plan's entry names, in this platform's separators.
fn relative_path(path: &str) -> PathBuf {
    path.split('/').fold(PathBuf::new(), |mut path, part| {
        path.push(part);
        path
    })
}

/// Every file below `root`, by the path it would take inside an installation.
pub(crate) fn describe_tree(root: &Path) -> Result<Tree> {
    let mut files = Tree::new();
    collect(root, root, &mut files)?;
    Ok(files)
}

fn collect(root: &Path, directory: &Path, files: &mut Tree) -> Result<()> {
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("cannot read {}", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect(root, &path, files)?;
            continue;
        }
        let name = path
            .strip_prefix(root)
            .with_context(|| format!("{} is outside the tree", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(name, (metadata.len(), super::net::sha256_file(&path)?));
    }
    Ok(())
}

/// Splits a release into the files an update carries and the ones it keeps.
fn split(old: &Tree, new: &Tree) -> (Vec<KeptFile>, Vec<String>) {
    let mut keep = Vec::new();
    let mut changed = Vec::new();
    for (path, (size, sha256)) in new {
        match old.get(path) {
            // Size first, then the digest: the two agree on a file that did not
            // change, and a file whose size moved is carried whatever its bytes are.
            Some((old_size, old_sha256)) if old_size == size && old_sha256 == sha256 => {
                keep.push(KeptFile {
                    path: path.clone(),
                    size: *size,
                    sha256: sha256.clone(),
                })
            }
            _ => changed.push(path.clone()),
        }
    }
    (keep, changed)
}

/// Builds the archive and the plan of an update package.
///
/// `target` is where the update archive is written; the caller owns that file.
pub(crate) fn build_update(
    from_archive: &Path,
    new_archive: &Path,
    version: &str,
    stub_directory: Option<&Path>,
    scratch: &Path,
    target: &Path,
) -> Result<(UpdatePlan, UpdateSummary)> {
    let old_tree = scratch.join("from");
    let new_tree = scratch.join("to");
    expand(
        from_archive,
        super::payload_format(from_archive)?,
        &old_tree,
        stub_directory,
    )
    .with_context(|| format!("cannot read the release at {}", from_archive.display()))?;
    expand(
        new_archive,
        super::payload_format(new_archive)?,
        &new_tree,
        stub_directory,
    )?;
    let (keep, changed) = split(&describe_tree(&old_tree)?, &describe_tree(&new_tree)?);
    if changed.is_empty() {
        bail!(
            "{} holds nothing that {} does not, so an update package between them would install \
             no file",
            new_archive.display(),
            from_archive.display()
        );
    }
    write_archive(&new_tree, &changed, target)?;
    let archive_size = std::fs::metadata(target)?.len();
    let summary = UpdateSummary {
        kept: keep.len(),
        changed: changed.len(),
        archive_size,
    };
    let plan = UpdatePlan {
        from: from_archive
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| from_archive.display().to_string()),
        version: version.to_string(),
        keep,
    };
    Ok((plan, summary))
}

/// Expands an archive with the runtime stub that matches its format.
fn expand(
    archive: &Path,
    format: PayloadFormat,
    destination: &Path,
    stub_directory: Option<&Path>,
) -> Result<()> {
    let stub = super::find_native_stub(format.stub_name(), stub_directory)?;
    std::fs::create_dir_all(destination)?;
    let output = Command::new(&stub)
        .arg("--extract")
        .arg(archive)
        .arg(destination)
        .output()
        .with_context(|| format!("failed to run {}", stub.display()))?;
    if !output.status.success() {
        bail!(
            "failed to read {}: {}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Writes the files an update carries into an archive of its own.
///
/// The archive is always a ZIP, whichever format the project's own payload
/// uses: the setup that carries it embeds the runtime matching this archive,
/// and a project that keeps its full release in 7z gets a ZIP update package
/// rather than no update package at all.
fn write_archive(root: &Path, files: &[String], target: &Path) -> Result<()> {
    let file = std::fs::File::create(target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let mut archive = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for name in files {
        let source = root.join(relative_path(name));
        archive
            .start_file(name.clone(), options)
            .with_context(|| format!("failed to add {name} to the update package"))?;
        let mut contents = std::fs::File::open(&source)?;
        std::io::copy(&mut contents, &mut archive)?;
    }
    let mut file = archive
        .finish()
        .context("failed to finish the update archive")?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two trees and the split between them.
    fn trees() -> (Tree, Tree) {
        let digest = |text: &str| super::super::net::sha256_hex(text.as_bytes()).unwrap();
        let old = Tree::from([
            ("E2eProbe.exe".to_string(), (11, digest("version one"))),
            ("data/kept.bin".to_string(), (4, digest("kept"))),
            ("gone.txt".to_string(), (4, digest("gone"))),
        ]);
        let new = Tree::from([
            ("E2eProbe.exe".to_string(), (11, digest("version two"))),
            ("data/kept.bin".to_string(), (4, digest("kept"))),
            ("added.txt".to_string(), (5, digest("added"))),
        ]);
        (old, new)
    }

    /// A file the new release leaves alone is kept, not carried.
    #[test]
    fn an_unchanged_file_is_kept_rather_than_carried() {
        let (old, new) = trees();
        let (keep, _) = split(&old, &new);
        assert_eq!(
            keep.iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["data/kept.bin"]
        );
        // The digest of the copy that stays is the one the machine is checked
        // against, so it is the byte count and hash of the file, not a promise.
        assert_eq!(keep[0].size, 4);
        assert_eq!(keep[0].sha256, new["data/kept.bin"].1);
    }

    /// A changed file and a file that did not exist before both travel.
    #[test]
    fn a_changed_or_new_file_travels_in_the_update() {
        let (old, new) = trees();
        let (_, changed) = split(&old, &new);
        assert_eq!(changed, vec!["E2eProbe.exe", "added.txt"]);
    }

    /// A file whose size changed is carried even when its digest is the same.
    #[test]
    fn a_file_that_moved_in_size_travels() {
        let digest = super::super::net::sha256_hex(b"same bytes").unwrap();
        let old = Tree::from([("file.bin".to_string(), (10, digest.clone()))]);
        let new = Tree::from([("file.bin".to_string(), (11, digest))]);
        let (keep, changed) = split(&old, &new);
        assert!(keep.is_empty());
        assert_eq!(changed, vec!["file.bin"]);
    }

    /// A tree is described by the paths an installation would carry, digests
    /// included, however deep it nests.
    #[test]
    fn a_tree_is_described_by_its_installed_paths() -> Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("data/deep"))?;
        std::fs::write(temp.path().join("app.exe"), b"exe")?;
        std::fs::write(temp.path().join("data/deep/notes.txt"), b"notes")?;
        let described = describe_tree(temp.path())?;
        assert_eq!(
            described.keys().cloned().collect::<Vec<_>>(),
            vec!["app.exe", "data/deep/notes.txt"]
        );
        assert_eq!(
            described["app.exe"],
            (3, super::super::net::sha256_hex(b"exe")?)
        );
        Ok(())
    }

    /// The plan survives the trip through the bundle.
    #[test]
    fn a_plan_round_trips_through_its_json() -> Result<()> {
        let plan = UpdatePlan {
            from: "app-v1.zip".to_string(),
            version: "1.0.1".to_string(),
            keep: vec![KeptFile {
                path: "data/kept.bin".to_string(),
                size: 4,
                sha256: "ab".repeat(32),
            }],
        };
        let text = serde_json::to_vec(&plan.to_json())?;
        let value: Value = serde_json::from_slice(&text)?;
        let kept = value["keep"].as_array().expect("a keep list").clone();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["path"], "data/kept.bin");
        assert_eq!(kept[0]["size"], 4);
        assert_eq!(kept[0]["sha256"].as_str().unwrap().len(), 64);
        // A relative path keeps its shape on the way back out.
        assert_eq!(relative_path("data/kept.bin"), Path::new("data/kept.bin"));
        Ok(())
    }
}
