//! Dependency resolution module

use std::collections::{HashMap, HashSet, VecDeque};

use semver::{Version, VersionReq};

use crate::pkg::error::{PkgError, Result};
use crate::pkg::registry::{PackageMetadata, PackagistClient, VersionMetadata};

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub metadata: VersionMetadata,
}

#[derive(Debug)]
pub struct ResolveResult {
    pub packages: Vec<ResolvedPackage>,
}

pub struct DependencyResolver {
    client: PackagistClient,
}

impl DependencyResolver {
    pub fn new(client: PackagistClient) -> Self {
        Self { client }
    }

    pub async fn resolve(
        &self,
        root_require: &HashMap<String, String>,
        include_dev: bool,
    ) -> Result<ResolveResult> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(String, String)> = VecDeque::new();

        for (name, constraint) in root_require {
            if is_platform_package(name) {
                continue;
            }
            queue.push_back((name.clone(), constraint.clone()));
        }

        while let Some((package, constraint)) = queue.pop_front() {
            if visited.contains(&package) {
                continue;
            }
            let metadata = self.client.get_package_metadata(&package).await?;
            let selected = select_version(&metadata, &constraint)?;
            let selected_version = selected.version.clone();

            visited.insert(package.clone());
            resolved.push(ResolvedPackage {
                name: package.clone(),
                version: selected_version.clone(),
                metadata: selected.clone(),
            });

            let mut deps = HashMap::new();
            if let Some(reqs) = selected.require.as_ref() {
                deps.extend(parse_require_map(reqs));
            }
            if include_dev {
                if let Some(reqs) = selected.require_dev.as_ref() {
                    deps.extend(parse_require_map(reqs));
                }
            }

            for (dep_name, dep_constraint) in deps {
                if is_platform_package(&dep_name) {
                    continue;
                }
                if !visited.contains(&dep_name) {
                    queue.push_back((dep_name, dep_constraint));
                }
            }
        }

        Ok(ResolveResult { packages: resolved })
    }
}

fn is_platform_package(name: &str) -> bool {
    name == "php" || name.starts_with("ext-") || name.starts_with("lib-")
}

fn parse_require_map(value: &serde_json::Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (name, constraint) in obj {
            if let Some(constraint_str) = constraint.as_str() {
                map.insert(name.to_string(), constraint_str.to_string());
            }
        }
    }
    map
}

fn select_version(metadata: &PackageMetadata, constraint: &str) -> Result<VersionMetadata> {
    if constraint.trim().is_empty() || constraint == "*" {
        return metadata
            .latest_stable_version()
            .cloned()
            .ok_or_else(|| PkgError::VersionNotFound {
                package: metadata.name.clone(),
                version: "*".to_string(),
            });
    }

    if let Some(exact) = metadata.get_version(constraint) {
        return Ok(exact.clone());
    }

    let parsed_req = parse_version_req(constraint);
    if let Some(req) = parsed_req {
        let mut candidates: Vec<(Version, &VersionMetadata)> = metadata
            .versions
            .iter()
            .filter_map(|(v, meta)| normalize_version(v).map(|ver| (ver, meta)))
            .filter(|(ver, _)| req.matches(ver))
            .collect();
        candidates.sort_by(|(a, _), (b, _)| a.cmp(b));
        if let Some((_, meta)) = candidates.pop() {
            return Ok(meta.clone());
        }
    }

    metadata
        .latest_stable_version()
        .cloned()
        .ok_or_else(|| PkgError::VersionNotFound {
            package: metadata.name.clone(),
            version: constraint.to_string(),
        })
}

fn parse_version_req(constraint: &str) -> Option<VersionReq> {
    let normalized = constraint.trim().trim_start_matches('v');
    VersionReq::parse(normalized).ok()
}

fn normalize_version(version: &str) -> Option<Version> {
    let trimmed = version.trim().trim_start_matches('v');
    Version::parse(trimmed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(name: &str, versions: &[(&str, &str)]) -> PackageMetadata {
        let mut map = HashMap::new();
        for (version, dep) in versions {
            let require = if dep.is_empty() {
                None
            } else {
                let mut obj = serde_json::Map::new();
                obj.insert(dep.to_string(), serde_json::Value::String("^1.0".to_string()));
                Some(serde_json::Value::Object(obj))
            };
            map.insert(
                version.to_string(),
                VersionMetadata {
                    version: version.to_string(),
                    version_normalized: None,
                    source: None,
                    dist: None,
                    require,
                    require_dev: None,
                    autoload: None,
                    time: None,
                    type_: None,
                    license: None,
                    description: None,
                    name: Some(name.to_string()),
                    extra: None,
                },
            );
        }
        PackageMetadata {
            name: name.to_string(),
            versions: map,
        }
    }

    #[test]
    fn test_select_version_exact() {
        let meta = metadata("demo/pkg", &[("1.0.0", "")]);
        let selected = select_version(&meta, "1.0.0").unwrap();
        assert_eq!(selected.version, "1.0.0");
    }

    #[test]
    fn test_select_version_semver_req() {
        let meta = metadata("demo/pkg", &[("1.0.0", ""), ("1.2.0", "")]);
        let selected = select_version(&meta, "^1.0").unwrap();
        assert_eq!(selected.version, "1.2.0");
    }
}
