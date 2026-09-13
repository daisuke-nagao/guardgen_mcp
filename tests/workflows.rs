use std::fs;
use std::path::PathBuf;

fn workflow(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(".github")
        .join("workflows")
        .join(name);
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {name}: {error}"))
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn action_reference(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix("uses:")
        .or_else(|| line.trim().strip_prefix("- uses:"))
        .map(|value| value.split_whitespace().next().unwrap_or_default())
}

fn assert_action_shas(workflow_name: &str, workflow: &str) {
    let mut action_count = 0;
    for line in workflow.lines() {
        let Some(reference) = action_reference(line) else {
            continue;
        };
        action_count += 1;
        let sha = reference
            .split_once('@')
            .map(|(_, sha)| sha)
            .unwrap_or_default();
        assert!(
            sha.len() == 40 && sha.chars().all(|character| character.is_ascii_hexdigit()),
            "{workflow_name} action is not pinned to a 40-character hexadecimal SHA: {line}"
        );
    }
    assert!(action_count > 0, "{workflow_name} has no actions");
}

fn assert_checkout_credentials_disabled(workflow_name: &str, workflow: &str) {
    let lines: Vec<&str> = workflow.lines().collect();
    let mut checkout_count = 0;
    for (index, line) in lines.iter().enumerate() {
        let Some(reference) = action_reference(line) else {
            continue;
        };
        if !reference.starts_with("actions/checkout@") {
            continue;
        }
        checkout_count += 1;
        let step_indent = if line.trim().starts_with("- uses:") {
            indentation(line)
        } else {
            lines[..index]
                .iter()
                .rev()
                .find(|preceding| {
                    preceding.trim().starts_with("- ") && indentation(preceding) < indentation(line)
                })
                .map(|preceding| indentation(preceding))
                .expect("checkout step")
        };
        let mut with_indent = None;
        let mut found = false;
        for following in lines.iter().skip(index + 1) {
            if following.trim().starts_with("- ") && indentation(following) <= step_indent {
                break;
            }
            if following.trim() == "with:" {
                with_indent = Some(indentation(following));
                continue;
            }
            if with_indent.is_some_and(|indent| {
                !following.trim().is_empty() && indentation(following) <= indent
            }) {
                with_indent = None;
            }
            if with_indent.is_some_and(|indent| indentation(following) > indent)
                && following.trim() == "persist-credentials: false"
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "{workflow_name} checkout must disable persisted credentials"
        );
    }
    assert!(checkout_count > 0, "{workflow_name} has no checkout step");
}

fn publish_job(cd: &str) -> String {
    cd.split_once("\n  publish:\n")
        .map(|(_, publish)| publish)
        .expect("CD publish job")
        .lines()
        .take_while(|line| indentation(line) != 2)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn workflows_enforce_minimum_security_policy() {
    let ci = workflow("ci.yml");
    let cd = workflow("cd.yml");

    for (name, workflow) in [("ci", ci.as_str()), ("cd", cd.as_str())] {
        assert_action_shas(name, workflow);
        assert_checkout_credentials_disabled(name, workflow);
        assert!(
            workflow.contains("permissions:\n  contents: read"),
            "{name} must default to read-only repository contents"
        );
    }
    assert!(
        !ci.contains("contents: write"),
        "CI must not receive write permission"
    );

    let publish = publish_job(&cd);
    assert!(
        publish.contains("    permissions:\n      contents: write"),
        "CD publish job must explicitly receive the release permission"
    );
    assert_eq!(
        cd.matches("contents: write").count(),
        1,
        "only the CD publish job may receive write permission"
    );
}

#[test]
fn publish_scope_excludes_following_jobs() {
    let cd = "\n  publish:\n    permissions:\n      contents: read\n  unrelated:\n    permissions:\n      contents: write\n";
    let publish = publish_job(cd);

    assert!(
        !publish.contains("  unrelated:\n"),
        "the publish scope must stop before the next job"
    );
}
