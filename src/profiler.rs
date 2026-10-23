//! Profiler: classifies every file and detects the project's languages,
//! entry points, manifest markers, and AI rules files.

/// What a file is, from vibe-pilot's point of view.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Source,
    Test,
    Config,
    Doc,
    Rules, // CLAUDE.md, .cursorrules, AGENTS.md ...
    Lockfile,
    Generated,
    Asset,
    Binary,
    Secret, // .env, *.pem — content is NEVER read
    Data,
    Other,
}

impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Kind::Source => "source",
            Kind::Test => "test",
            Kind::Config => "config",
            Kind::Doc => "doc",
            Kind::Rules => "rules",
            Kind::Lockfile => "lockfile",
            Kind::Generated => "generated",
            Kind::Asset => "asset",
            Kind::Binary => "binary",
            Kind::Secret => "secret",
            Kind::Data => "data",
            Kind::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub primary: String,
    pub langs: Vec<(String, usize)>,
    pub entries: Vec<String>,
    pub rules: Vec<String>,
    pub markers: Vec<String>,
}

pub fn lang_of(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" | "pyi" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "ex" | "exs" => "elixir",
        "scala" => "scala",
        "sh" | "bash" | "zsh" => "shell",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "vue" => "vue",
        "svelte" => "svelte",
        "dart" => "dart",
        "lua" => "lua",
        "r" => "r",
        "jl" => "julia",
        "zig" => "zig",
        "nim" => "nim",
        "hs" => "haskell",
        "elm" => "elm",
        "clj" | "cljs" | "cljc" => "clojure",
        "erl" => "erlang",
        "fs" => "fsharp",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "tf" => "terraform",
        "sol" => "solidity",
        "move" => "move",
        "md" | "mdx" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        _ => "",
    }
}

pub fn fence_of(ext: &str) -> &'static str {
    match ext {
        "" => "text",
        "md" | "mdx" => "markdown",
        "sh" | "bash" | "zsh" => "bash",
        "ts" => "ts",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "js",
        "jsx" => "jsx",
        "py" | "pyi" => "python",
        "rb" => "ruby",
        "go" => "go",
        "rs" => "rust",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "ex" | "exs" => "elixir",
        "scala" => "scala",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "vue" => "vue",
        "svelte" => "svelte",
        "dart" => "dart",
        "lua" => "lua",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "json" => "json",
        "tf" => "hcl",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        _ => "text",
    }
}

const LOCKFILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "cargo.lock",
    "poetry.lock",
    "pipfile.lock",
    "uv.lock",
    "pdm.lock",
    "composer.lock",
    "gemfile.lock",
    "go.sum",
    "flake.lock",
    "bun.lock",
    "bun.lockb",
    "deno.lock",
];

const RULES_FILES: &[&str] = &[
    "claude.md",
    "agents.md",
    "gemini.md",
    "cursorrules",
    "windsurfrules",
    "clinerules",
    "copilot-instructions.md",
];

const MANIFESTS: &[&str] = &[
    "package.json",
    "cargo.toml",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "requirements.txt",
    "pubspec.yaml",
    "gemfile",
    "composer.json",
    "mix.exs",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
];

const CONFIG_EXACT: &[&str] = &[
    "tsconfig.json",
    "jsconfig.json",
    "makefile",
    "justfile",
    "dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    ".dockerignore",
    "biome.json",
    "turbo.json",
    "nx.json",
    "lerna.json",
    "pnpm-workspace.yaml",
    ".editorconfig",
    ".gitignore",
    ".gitattributes",
    ".npmrc",
    ".nvmrc",
    ".python-version",
    ".env.example",
    ".env.sample",
    "rust-toolchain",
    "rust-toolchain.toml",
    "clippy.toml",
    "deny.toml",
    "renovate.json",
    "vercel.json",
    "netlify.toml",
    "wrangler.toml",
    "procfile",
    "taskfile.yml",
];

const BINARY_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico", "icns", "tiff", "pdf", "zip", "gz",
    "bz2", "xz", "7z", "rar", "tar", "wasm", "exe", "dll", "dylib", "so", "a", "o", "lib", "class",
    "jar", "pyc", "pyo", "bin", "dat", "woff", "woff2", "ttf", "otf", "eot", "mp3", "wav", "ogg",
    "flac", "mp4", "webm", "mov", "avi", "mkv", "sqlite", "db", "kdbx", "p12", "pfx", "lockb",
];

const ASSET_EXT: &[&str] = &["svg"];

/// Name-only check for files whose content must never be read or emitted.
pub fn is_secret_name(rel: &str) -> bool {
    let l = rel.to_ascii_lowercase();
    let name = l.rsplit('/').next().unwrap_or(&l);
    if name == ".env" || (name.starts_with(".env.") && !name.starts_with(".env.example")) {
        return true;
    }
    if name.starts_with("id_rsa")
        || name.starts_with("id_dsa")
        || name.starts_with("id_ed25519")
        || name.starts_with("id_ecdsa")
    {
        return true;
    }
    if name.contains("credential") || name.contains("secret.key") {
        return true;
    }
    for e in [".pem", ".key", ".p12", ".pfx", ".kdbx"] {
        if name.ends_with(e) {
            return true;
        }
    }
    false
}

/// Full classification. `head` = first ~2KB of content (None for secrets /
/// unreadable files). Lowercase `rel` recommended.
pub fn classify(rel: &str, ext: &str, head: Option<&str>) -> Kind {
    let l = rel.to_ascii_lowercase();
    let name = l.rsplit('/').next().unwrap_or(&l);

    if is_secret_name(&l) {
        return Kind::Secret;
    }
    if LOCKFILES.contains(&name) {
        return Kind::Lockfile;
    }
    if RULES_FILES.contains(&name) || l.starts_with(".cursor/rules/") {
        return Kind::Rules;
    }
    if MANIFESTS.contains(&name) || CONFIG_EXACT.contains(&name) {
        return Kind::Config;
    }
    if name.ends_with(".config.ts")
        || name.ends_with(".config.js")
        || name.ends_with(".config.mjs")
        || name.ends_with(".config.cjs")
        || name.ends_with(".config.mts")
        || name.starts_with(".eslintrc")
        || name.starts_with(".prettierrc")
        || name.starts_with("docker-compose.")
    {
        return Kind::Config;
    }
    // generated code — never useful as context, always counted in the raw bill
    let generated_name = name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || name.ends_with(".map")
        || name.ends_with(".g.dart")
        || name.ends_with(".freezed.dart")
        || name.ends_with(".pb.go")
        || name.ends_with(".pb.cc")
        || name.ends_with(".pb.h")
        || name.ends_with("_pb2.py")
        || name.ends_with("_pb2_grpc.py")
        || name.ends_with(".designer.cs")
        || name.ends_with(".snap")
        || name.ends_with(".generated.ts")
        || name.ends_with(".generated.js")
        || name.ends_with(".generated.rs")
        || name.ends_with(".gen.ts")
        || name.ends_with(".gen.js");
    if generated_name {
        return Kind::Generated;
    }
    if let Some(h) = head {
        let head2: String = h.to_ascii_lowercase().chars().take(400).collect();
        if head2.contains("do not edit")
            || head2.contains("@generated")
            || head2.contains("auto-generated")
            || head2.contains("generated by")
            || head2.contains("<auto-generated")
            || head2.contains("codegen")
        {
            return Kind::Generated;
        }
    }
    // tests
    let is_test = l.contains("/tests/")
        || l.contains("/test/")
        || l.contains("/__tests__/")
        || l.contains("/spec/")
        || l.contains("/e2e/")
        || l.contains("__snapshots__")
        || name.ends_with(".test.ts")
        || name.ends_with(".test.tsx")
        || name.ends_with(".test.js")
        || name.ends_with(".test.jsx")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.tsx")
        || name.ends_with(".spec.js")
        || name.ends_with(".spec.jsx")
        || name.ends_with("_test.rs")
        || name.ends_with("_test.go")
        || name.ends_with("_test.py")
        || name.starts_with("test_")
        || name.starts_with("conftest")
        || name.ends_with("tests.java")
        || name.ends_with("test.java");
    if is_test {
        return Kind::Test;
    }
    if BINARY_EXT.contains(&ext) {
        return Kind::Binary;
    }
    if ASSET_EXT.contains(&ext) {
        return Kind::Asset;
    }
    if ext == "md" || ext == "mdx" || l.starts_with("docs/") || l.starts_with("doc/") {
        return Kind::Doc;
    }
    if !lang_of(ext).is_empty()
        && lang_of(ext) != "markdown"
        && lang_of(ext) != "json"
        && lang_of(ext) != "yaml"
        && lang_of(ext) != "toml"
    {
        return Kind::Source;
    }
    if ext == "json" || ext == "csv" || ext == "tsv" || ext == "xml" || ext == "txt" {
        return Kind::Data;
    }
    Kind::Other
}

/// Analyze a scanned repo: languages by file count, entry points, rules files.
/// `files` = already-classified analyses (rel paths, kinds, langs).
pub fn analyze<F>(
    root_name: &str,
    files: &[F],
    rel: impl Fn(&F) -> &str,
    kind: impl Fn(&F) -> Kind,
    lang: impl Fn(&F) -> &str,
) -> ProjectInfo {
    let mut counts: Vec<(String, usize)> = Vec::new();
    let mut rules = Vec::new();
    let mut markers = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    for f in files {
        let r = rel(f);
        paths.push(r.to_string());
        let k = kind(f);
        let name = r.rsplit('/').next().unwrap_or(r).to_ascii_lowercase();
        if matches!(k, Kind::Source | Kind::Test) {
            let l = lang(f);
            if !l.is_empty() {
                match counts.iter_mut().find(|(x, _)| x == l) {
                    Some(c) => c.1 += 1,
                    None => counts.push((l.to_string(), 1)),
                }
            }
        }
        if k == Kind::Rules {
            rules.push(r.to_string());
        }
        if MANIFESTS.contains(&name.as_str()) {
            markers.push(r.to_string());
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let primary = counts.first().map(|(l, _)| l.clone()).unwrap_or_default();

    let mut entries = Vec::new();
    for cand in entry_candidates(&primary) {
        if paths.iter().any(|p| p == &cand) {
            entries.push(cand);
        }
        if entries.len() >= 3 {
            break;
        }
    }
    if entries.is_empty() {
        // generic fallback: main.* / index.* near the root
        for p in &paths {
            let n = p.rsplit('/').next().unwrap_or(p);
            let depth = p.matches('/').count();
            if depth <= 1
                && (n.starts_with("main.") || n.starts_with("index."))
                && lang_of(n.rsplit('.').next().unwrap_or("")).len() > 2
            {
                entries.push(p.clone());
            }
            if entries.len() >= 2 {
                break;
            }
        }
    }

    ProjectInfo {
        name: root_name.to_string(),
        primary,
        langs: counts,
        entries,
        rules,
        markers,
    }
}

fn entry_candidates(primary: &str) -> Vec<String> {
    match primary {
        "rust" => vec![
            "src/main.rs".into(),
            "src/lib.rs".into(),
            "main.rs".into(),
            "src/bin/main.rs".into(),
        ],
        "typescript" => vec![
            "src/index.ts".into(),
            "index.ts".into(),
            "src/main.ts".into(),
            "src/index.tsx".into(),
            "src/main.tsx".into(),
            "src/app/page.tsx".into(),
            "app/page.tsx".into(),
            "src/app/layout.tsx".into(),
            "pages/index.tsx".into(),
            "src/server.ts".into(),
            "server.ts".into(),
        ],
        "javascript" => vec![
            "index.js".into(),
            "src/index.js".into(),
            "src/main.js".into(),
            "src/app.js".into(),
            "app.js".into(),
            "server.js".into(),
            "src/server.js".into(),
        ],
        "python" => vec![
            "main.py".into(),
            "app.py".into(),
            "manage.py".into(),
            "run.py".into(),
            "src/__main__.py".into(),
            "__main__.py".into(),
        ],
        "go" => vec![
            "main.go".into(),
            "cmd/server/main.go".into(),
            "cmd/app/main.go".into(),
        ],
        "java" => vec!["pom.xml".into()],
        "kotlin" => vec!["build.gradle.kts".into()],
        "ruby" => vec!["config.ru".into()],
        "php" => vec!["index.php".into(), "public/index.php".into()],
        "elixir" => vec!["lib/application.ex".into()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_files() {
        assert_eq!(classify("src/main.rs", "rs", None), Kind::Source);
        assert_eq!(classify("package-lock.json", "json", None), Kind::Lockfile);
        assert_eq!(classify("CLAUDE.md", "md", None), Kind::Rules);
        assert_eq!(classify("src/x.test.ts", "ts", None), Kind::Test);
        assert_eq!(classify("app.min.js", "js", None), Kind::Generated);
        assert_eq!(classify("logo.png", "png", None), Kind::Binary);
        assert_eq!(classify(".env", "", None), Kind::Secret);
        assert_eq!(classify("cert.pem", "pem", None), Kind::Secret);
        assert_eq!(classify("README.md", "md", None), Kind::Doc);
        assert_eq!(classify("package.json", "json", None), Kind::Config);
    }

    #[test]
    fn detects_generated_banner() {
        assert_eq!(
            classify(
                "src/api.ts",
                "ts",
                Some("// @generated by protoc\nexport {}")
            ),
            Kind::Generated
        );
        assert_eq!(
            classify("src/hand.ts", "ts", Some("export const x = 1;")),
            Kind::Source
        );
    }
}
