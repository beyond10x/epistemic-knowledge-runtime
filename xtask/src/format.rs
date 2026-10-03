//! Format exactly the product workspace, preserving raw generated dependency bytes.

use std::path::Path;
use std::process::Command;

fn member_names(metadata: &serde_json::Value) -> Result<Vec<&str>, String> {
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo metadata has no workspace members")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Cargo metadata has no packages")?;
    if members.is_empty() {
        return Err("Cargo workspace is empty".into());
    }
    members
        .iter()
        .map(|id| {
            packages
                .iter()
                .find(|package| package["id"] == *id)
                .and_then(|package| package["name"].as_str())
                .ok_or_else(|| format!("workspace member {id} has no package"))
        })
        .collect()
}

pub(crate) fn run(root: &Path, check: bool) -> Result<(), String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let metadata = Command::new(&cargo)
        .current_dir(root)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--offline",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !metadata.status.success() {
        return Err(String::from_utf8_lossy(&metadata.stderr).into_owned());
    }
    let metadata = serde_json::from_slice(&metadata.stdout).map_err(|e| e.to_string())?;
    let mut command = Command::new(cargo);
    command.current_dir(root).arg("fmt");
    for name in member_names(&metadata)? {
        command.args(["--package", name]);
    }
    if check {
        command.args(["--", "--check"]);
    }
    let status = command.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("workspace formatting exited {status}"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_members_but_no_nested_dependency_are_formatted() {
        let metadata = serde_json::json!({
            "workspace_members":["product", "new-member"],
            "packages":[{"id":"generated", "name":"generated-types"}, {"id":"product","name":"product"}, {"id":"new-member","name":"new-member"}]
        });
        assert_eq!(
            super::member_names(&metadata).unwrap(),
            ["product", "new-member"]
        );
    }
}
