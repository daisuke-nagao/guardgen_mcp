use std::fs;
use std::path::PathBuf;

const TARGETS: [(&str, &str); 3] = [
    ("ubuntu-24.04", "x86_64-unknown-linux-gnu"),
    ("windows-2025", "x86_64-pc-windows-msvc"),
    ("macos-26", "aarch64-apple-darwin"),
];

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

fn mapping_block(input: &str, key: &str, indent: usize) -> String {
    let wanted = format!("{}{}", " ".repeat(indent), key);
    let lines: Vec<&str> = input.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim_end_matches('\r') == wanted)
        .unwrap_or_else(|| panic!("missing mapping {key:?} at indent {indent}"));
    let mut block = String::new();
    for (index, line) in lines.iter().enumerate().skip(start) {
        let line = line.trim_end_matches('\r');
        if index > start && !line.trim().is_empty() && indentation(line) <= indent {
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    block
}

fn compact(input: &str) -> String {
    input.split_whitespace().collect()
}

fn assert_contains(input: &str, expected: &str, context: &str) {
    assert!(
        compact(input).contains(&compact(expected)),
        "{context} is missing {expected:?}"
    );
}

fn assert_action_shas(workflow_name: &str, workflow: &str) {
    for line in workflow.lines() {
        let Some(value) = line.trim().strip_prefix("uses:") else {
            continue;
        };
        let reference = value
            .trim()
            .split_once('@')
            .map(|(_, reference)| reference.split_whitespace().next().unwrap_or_default())
            .unwrap_or_default();
        assert_eq!(
            reference.len(),
            40,
            "{workflow_name} action is not pinned to a 40-character SHA: {line}"
        );
        assert!(
            reference
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
            "{workflow_name} action has a non-hex SHA: {line}"
        );
    }
}

fn assert_checkout_credentials_disabled(workflow_name: &str, workflow: &str) {
    let lines: Vec<&str> = workflow.lines().collect();
    let mut checkout_count = 0;
    for (index, line) in lines.iter().enumerate() {
        if !line.trim().starts_with("uses: actions/checkout@") {
            continue;
        }
        checkout_count += 1;
        let uses_indent = indentation(line);
        let mut found = false;
        for following in lines.iter().skip(index + 1) {
            if !following.trim().is_empty() && indentation(following) < uses_indent {
                break;
            }
            if following.trim() == "persist-credentials: false" {
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

fn assert_matrix(job: &str, workflow_name: &str) {
    assert_contains(job, "fail-fast: false", workflow_name);
    let matrix = mapping_block(job, "matrix:", 6);
    assert_eq!(
        matrix
            .lines()
            .filter(|line| line.trim_start().starts_with("- os:"))
            .count(),
        TARGETS.len(),
        "{workflow_name} must contain exactly three matrix entries"
    );
    for (runner, target) in TARGETS {
        assert_contains(
            &matrix,
            &format!("- os: {runner}\n  target: {target}"),
            workflow_name,
        );
    }
}

fn assert_no_direct_shell_ref(workflow_name: &str, workflow: &str) {
    let lines: Vec<&str> = workflow.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("run:") {
            continue;
        }
        assert!(
            !trimmed.contains("github.ref"),
            "{workflow_name} expands a ref directly in a run scalar: {line}"
        );
        let run_indent = indentation(line);
        for following in lines.iter().skip(index + 1) {
            if !following.trim().is_empty() && indentation(following) <= run_indent {
                break;
            }
            assert!(
                !following.contains("github.ref"),
                "{workflow_name} expands a ref directly in a shell block: {following}"
            );
        }
    }
}

#[test]
fn ci_has_scheduled_three_target_test_matrix_and_quality_job() {
    let ci = workflow("ci.yml");
    let triggers = mapping_block(&ci, "on:", 0);
    for required in [
        "pull_request:",
        "push:",
        "main",
        "workflow_dispatch:",
        "schedule:",
    ] {
        assert_contains(&triggers, required, "ci trigger");
    }
    assert!(
        triggers
            .lines()
            .any(|line| line.contains("cron:") && line.contains("0 0 * * 1")),
        "ci schedule must run weekly at Monday 09:00 JST"
    );

    let quality = mapping_block(&ci, "checks:", 2);
    assert_contains(&quality, "pre-commit run", "ci quality job");
    assert!(
        !compact(&quality).contains("cargo test"),
        "quality must be separate from tests"
    );

    let tests = mapping_block(&ci, "test:", 2);
    assert_matrix(&tests, "ci test matrix");
    assert_contains(
        &tests,
        "cargo test --locked --target ${{ matrix.target }}",
        "ci test matrix",
    );
    for required in [
        "uses: taiki-e/install-action@fcf5432d9f50d67e37ee6e29bdb7a224ff67b4a7",
        "tool: cargo-about@0.9.2",
        "fallback: none",
        "cargo about generate --locked --fail --target ${{ matrix.target }}",
        "--output-file \"${{ runner.temp }}/THIRD-PARTY-LICENSES.html\"",
        "about.hbs",
    ] {
        assert_contains(&tests, required, "ci cargo-about generation");
    }
}

#[test]
fn cargo_about_configuration_is_strict_and_release_scoped() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = fs::read_to_string(root.join("about.toml"))
        .unwrap_or_else(|error| panic!("read about.toml: {error}"));
    let expected = r#"
accepted = ["MIT", "Apache-2.0", "Unicode-3.0"]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-apple-darwin",
]
ignore-build-dependencies = false
ignore-dev-dependencies = true
ignore-transitive-dependencies = false
workarounds = ["chrono"]
"#;
    assert_eq!(
        compact(&config),
        compact(expected),
        "cargo-about policy must remain exact"
    );

    let template = fs::read_to_string(root.join("about.hbs"))
        .unwrap_or_else(|error| panic!("read about.hbs: {error}"));
    for required in [
        "<title>Third Party Licenses</title>",
        "This page lists the licenses of the projects used in guardgen_mcp.",
        "{{#each overview}}",
        "{{#each licenses}}",
        "{{#each used_by}}",
        "{{crate.name}} {{crate.version}}",
        "<pre class=\"license-text\">{{text}}</pre>",
    ] {
        assert_contains(&template, required, "cargo-about template");
    }
}

#[test]
fn cd_is_tag_only_builds_unique_archives_and_publishes_after_all_legs() {
    let cd = workflow("cd.yml");
    let triggers = mapping_block(&cd, "on:", 0);
    for required in ["push:", "tags:", "v*.*.*"] {
        assert_contains(&triggers, required, "cd trigger");
    }
    for forbidden in ["pull_request", "workflow_dispatch", "schedule", "branches:"] {
        assert!(
            !compact(&triggers).contains(&compact(forbidden)),
            "cd trigger must not contain {forbidden:?}"
        );
    }

    let build = mapping_block(&cd, "build:", 2);
    assert_matrix(&build, "cd build matrix");
    let build_text = compact(&build);
    let test_position = build_text
        .find(&compact(
            "cargo test --locked --target ${{ matrix.target }}",
        ))
        .expect("cd build matrix test command");
    let release_position = build_text
        .find(&compact(
            "cargo build --release --locked --target ${{ matrix.target }}",
        ))
        .expect("cd build matrix release command");
    let licenses_position = build_text
        .find(&compact(
            "cargo about generate --locked --fail --target ${{ matrix.target }} --output-file \"target/${{ matrix.target }}/release/THIRD-PARTY-LICENSES.html\" about.hbs",
        ))
        .expect("cd cargo-about generation command");
    let unix_package_position = build_text
        .find(&compact("tar -czf"))
        .expect("cd Unix package");
    let windows_package_position = build_text
        .find(&compact("Compress-Archive"))
        .expect("cd Windows package");
    assert!(
        test_position < release_position,
        "CD must test before building"
    );
    assert!(
        release_position < licenses_position,
        "CD must build before generating license notices"
    );
    assert!(
        licenses_position < unix_package_position && licenses_position < windows_package_position,
        "CD must generate license notices before packaging"
    );
    for required in [
        "uses: taiki-e/install-action@fcf5432d9f50d67e37ee6e29bdb7a224ff67b4a7",
        "tool: cargo-about@0.9.2",
        "fallback: none",
    ] {
        assert_contains(&build, required, "cd cargo-about installation");
    }
    assert_contains(
        &build,
        "name: guardgen_mcp-${{ matrix.target }}",
        "cd artifact",
    );
    assert_contains(&build, "if-no-files-found: error", "cd artifact");
    assert_contains(&build, "tar -czf", "cd Unix archive");
    assert_contains(&build, "test -s \"$file\"", "cd Unix non-empty-file check");
    assert_contains(
        &build,
        "tar -czf \"$archive\" -C \"$release_dir\" \"$(basename \"$binary\")\" THIRD-PARTY-LICENSES.html LICENSE-MIT LICENSE-APACHE",
        "cd Unix archive contents",
    );
    assert_contains(&build, "Compress-Archive", "cd Windows archive");
    assert_contains(&build, "Test-Path", "cd Windows missing-file check");
    assert_contains(
        &build,
        "(Get-Item -LiteralPath $file).Length -eq 0",
        "cd Windows non-empty-file check",
    );
    assert_contains(
        &build,
        "$archive_files = @( \"$release_dir/guardgen_mcp.exe\" \"$release_dir/THIRD-PARTY-LICENSES.html\" \"$release_dir/LICENSE-MIT\" \"$release_dir/LICENSE-APACHE\" )",
        "cd Windows archive contents",
    );
    assert_contains(
        &build,
        "Compress-Archive -LiteralPath $archive_files -DestinationPath $archive",
        "cd Windows archive command",
    );

    let publish = mapping_block(&cd, "publish:", 2);
    assert_contains(&publish, "needs: build", "cd publish dependency");
    assert_contains(&publish, "contents: write", "cd publish permission");
    assert_contains(
        &publish,
        "download-artifact",
        "cd publish artifact download",
    );
    assert_contains(
        &publish,
        "merge-multiple: true",
        "cd publish artifact merge",
    );
    for required in ["gh release create", "--verify-tag", "--generate-notes"] {
        assert_contains(&publish, required, "cd release command");
    }
    assert_contains(
        &publish,
        "GH_TOKEN: ${{ github.token }}",
        "cd release authentication",
    );
    assert_contains(
        &publish,
        "GH_REPO: ${{ github.repository }}",
        "cd release repository",
    );
    assert_contains(
        &publish,
        "test \"$(find dist -type f | wc -l)\" -eq 3",
        "cd release asset count",
    );
    assert_no_direct_shell_ref("cd", &cd);
}

#[test]
fn workflows_pin_actions_and_limit_release_permissions() {
    let ci = workflow("ci.yml");
    let cd = workflow("cd.yml");
    assert_action_shas("ci", &ci);
    assert_action_shas("cd", &cd);
    assert_checkout_credentials_disabled("ci", &ci);
    assert_checkout_credentials_disabled("cd", &cd);

    for (name, workflow) in [("ci", ci.as_str()), ("cd", cd.as_str())] {
        assert_contains(workflow, "permissions: contents: read", name);
        assert!(
            !workflow.contains("pull_request_target"),
            "{name} must not use pull_request_target"
        );
    }
    assert_contains(
        &mapping_block(&cd, "publish:", 2),
        "permissions: contents: write",
        "cd publish permission",
    );
    assert!(!compact(&mapping_block(&cd, "build:", 2)).contains("contents:write"));
    assert!(
        cd.contains("actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"),
        "CD must use the approved upload-artifact pin"
    );
    assert!(
        cd.contains("actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c"),
        "CD must use the approved download-artifact pin"
    );
    let cargo_about_action = "taiki-e/install-action@fcf5432d9f50d67e37ee6e29bdb7a224ff67b4a7";
    for (name, workflow) in [("ci", ci.as_str()), ("cd", cd.as_str())] {
        assert_eq!(
            workflow.matches(cargo_about_action).count(),
            1,
            "{name} must use the approved cargo-about installer exactly once"
        );
    }
}
