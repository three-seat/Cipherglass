//! Dependency boundary test (F-001 T-1 to T-5, T-10; ADR-0001).
//!
//! Reads every workspace crate's declared dependencies from `cargo metadata`
//! and checks them against the tables below. The tables are the enforcing copy
//! of `architecture.crates` and `dependencies.approved` in each feature
//! contract; keep them in sync.

use std::collections::BTreeSet;
use std::process::Command;

use serde_json::Value;

/// Workspace crate -> workspace crates it may depend on, by any dependency kind.
const WORKSPACE: &[(&str, &[&str])] = &[
    ("cipherglass-core", &[]),
    ("cipherglass-js", &["cipherglass-core"]),
    ("cipherglass-graphql", &["cipherglass-core"]),
    (
        "cipherglass",
        &["cipherglass-core", "cipherglass-js", "cipherglass-graphql"],
    ),
    ("cipherglass-cli", &["cipherglass"]),
];

/// Approved third-party dependencies: (crate, kind, dependency).
/// A JavaScript engine or runtime must never be added here (REQ-023).
const APPROVED: &[(&str, Kind, &str)] = &[
    ("cipherglass-cli", Kind::Normal, "clap"),
    ("cipherglass", Kind::Dev, "serde_json"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug)]
struct Dependency {
    name: String,
    kind: Kind,
}

#[derive(Debug)]
struct Package {
    name: String,
    /// `cargo metadata` reports `publish = false` as an empty registry list.
    publish_disabled: bool,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Violation {
    UnclassifiedCrate(String),
    ForbiddenWorkspaceEdge {
        from: String,
        to: String,
        kind: Kind,
    },
    UnapprovedDependency {
        krate: String,
        dependency: String,
        kind: Kind,
    },
    Publishable(String),
}

fn parse_metadata(json: &str) -> Result<Vec<Package>, String> {
    let root: Value = serde_json::from_str(json).map_err(|e| format!("invalid JSON: {e}"))?;
    let packages = root["packages"]
        .as_array()
        .ok_or("metadata has no `packages` array")?;
    packages.iter().map(parse_package).collect()
}

fn parse_package(package: &Value) -> Result<Package, String> {
    let name = package["name"]
        .as_str()
        .ok_or("package has no `name`")?
        .to_owned();
    let publish_disabled = package["publish"].as_array().is_some_and(Vec::is_empty);
    let dependencies = package["dependencies"]
        .as_array()
        .ok_or_else(|| format!("{name}: no `dependencies` array"))?
        .iter()
        .map(|dep| {
            let dep_name = dep["name"]
                .as_str()
                .ok_or_else(|| format!("{name}: dependency has no `name`"))?;
            let kind = match &dep["kind"] {
                Value::Null => Kind::Normal,
                Value::String(k) if k == "dev" => Kind::Dev,
                Value::String(k) if k == "build" => Kind::Build,
                other => return Err(format!("{name}: unknown dependency kind {other}")),
            };
            Ok(Dependency {
                name: dep_name.to_owned(),
                kind,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Package {
        name,
        publish_disabled,
        dependencies,
    })
}

fn check(
    packages: &[Package],
    workspace: &[(&str, &[&str])],
    approved: &[(&str, Kind, &str)],
) -> Vec<Violation> {
    let members: BTreeSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let mut violations = Vec::new();

    for package in packages {
        if !package.publish_disabled {
            violations.push(Violation::Publishable(package.name.clone()));
        }
        let Some((_, allowed)) = workspace.iter().find(|(name, _)| *name == package.name) else {
            violations.push(Violation::UnclassifiedCrate(package.name.clone()));
            continue;
        };
        for dep in &package.dependencies {
            if members.contains(dep.name.as_str()) {
                if !allowed.contains(&dep.name.as_str()) {
                    violations.push(Violation::ForbiddenWorkspaceEdge {
                        from: package.name.clone(),
                        to: dep.name.clone(),
                        kind: dep.kind,
                    });
                }
            } else if !approved.contains(&(package.name.as_str(), dep.kind, dep.name.as_str())) {
                violations.push(Violation::UnapprovedDependency {
                    krate: package.name.clone(),
                    dependency: dep.name.clone(),
                    kind: dep.kind,
                });
            }
        }
    }

    violations.sort();
    violations
}

fn workspace_metadata() -> String {
    let cargo = std::env::var("CARGO").expect("CARGO is not set; run this test through cargo");
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo metadata output is not UTF-8")
}

#[test]
fn workspace_has_no_boundary_violations() {
    let packages = parse_metadata(&workspace_metadata()).expect("unparsable cargo metadata");
    assert_eq!(
        packages.len(),
        WORKSPACE.len(),
        "unexpected workspace members"
    );
    assert_eq!(check(&packages, WORKSPACE, APPROVED), vec![]);
}

// Synthetic inputs prove the checker detects each kind of violation.

fn package(name: &str, deps: &[(&str, Kind)]) -> Package {
    Package {
        name: name.to_owned(),
        publish_disabled: true,
        dependencies: deps
            .iter()
            .map(|&(name, kind)| Dependency {
                name: name.to_owned(),
                kind,
            })
            .collect(),
    }
}

fn analyzer_and_adapter(analyzer_deps: &[(&str, Kind)]) -> Vec<Package> {
    vec![
        package("cipherglass-core", &[]),
        package("cipherglass-js", &[("cipherglass-core", Kind::Normal)]),
        package("cipherglass-graphql", analyzer_deps),
    ]
}

#[test]
fn detects_forbidden_workspace_edge() {
    let packages = analyzer_and_adapter(&[("cipherglass-js", Kind::Normal)]);
    assert_eq!(
        check(&packages, WORKSPACE, APPROVED),
        vec![Violation::ForbiddenWorkspaceEdge {
            from: "cipherglass-graphql".into(),
            to: "cipherglass-js".into(),
            kind: Kind::Normal,
        }]
    );
}

#[test]
fn detects_forbidden_edge_declared_as_dev_dependency() {
    let packages = analyzer_and_adapter(&[("cipherglass-js", Kind::Dev)]);
    assert_eq!(
        check(&packages, WORKSPACE, APPROVED),
        vec![Violation::ForbiddenWorkspaceEdge {
            from: "cipherglass-graphql".into(),
            to: "cipherglass-js".into(),
            kind: Kind::Dev,
        }]
    );
}

#[test]
fn detects_unapproved_third_party_dependency() {
    let packages = analyzer_and_adapter(&[("oxc_parser", Kind::Normal)]);
    assert_eq!(
        check(&packages, WORKSPACE, APPROVED),
        vec![Violation::UnapprovedDependency {
            krate: "cipherglass-graphql".into(),
            dependency: "oxc_parser".into(),
            kind: Kind::Normal,
        }]
    );
}

#[test]
fn approval_is_specific_to_dependency_kind() {
    // serde_json is approved for cipherglass as a dev-dependency only.
    let packages = vec![package("cipherglass", &[("serde_json", Kind::Normal)])];
    assert_eq!(
        check(&packages, WORKSPACE, APPROVED),
        vec![Violation::UnapprovedDependency {
            krate: "cipherglass".into(),
            dependency: "serde_json".into(),
            kind: Kind::Normal,
        }]
    );
}

#[test]
fn detects_unclassified_crate() {
    let packages = vec![package("cipherglass-extra", &[])];
    assert_eq!(
        check(&packages, WORKSPACE, APPROVED),
        vec![Violation::UnclassifiedCrate("cipherglass-extra".into())]
    );
}

#[test]
fn detects_publishable_crate() {
    let mut core = package("cipherglass-core", &[]);
    core.publish_disabled = false;
    assert_eq!(
        check(&[core], WORKSPACE, APPROVED),
        vec![Violation::Publishable("cipherglass-core".into())]
    );
}

#[test]
fn malformed_metadata_is_an_error_not_an_empty_result() {
    assert!(parse_metadata("not json").is_err());
    assert!(parse_metadata("{}").is_err());
    assert!(parse_metadata(r#"{"packages": [{"name": "x"}]}"#).is_err());
}
