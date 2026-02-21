//! `rava init` — initialize a new Rava project.

use anyhow::{Context, Result};
use clap::Args;
use rava_pkg::{BuildConfig, ProjectConfig, ProjectMeta};

#[derive(Args)]
pub struct InitArgs {
    /// Project name (defaults to current directory name)
    pub name: Option<String>,

    /// Project template: app | lib | cli
    #[arg(long, default_value = "app")]
    pub template: String,
}

pub async fn init(args: InitArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Refuse to overwrite an existing project
    if cwd.join("rava.hcl").exists() {
        anyhow::bail!("rava.hcl already exists in this directory");
    }

    let name = args.name.unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-app")
            .to_string()
    });

    let main_class = to_pascal_case(&name);

    // Build config
    let config = ProjectConfig {
        project: ProjectMeta {
            name:    name.clone(),
            version: "0.1.0".into(),
            java:    "21".into(),
            license: "MIT".into(),
        },
        build: BuildConfig {
            target:   "native".into(),
            main:     format!("{main_class}"),
            optimize: "speed".into(),
        },
        ..Default::default()
    };

    // Write rava.hcl
    config.to_file(&cwd.join("rava.hcl"))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Scaffold src/
    let src_dir = cwd.join("src");
    std::fs::create_dir_all(&src_dir)
        .with_context(|| format!("cannot create {}", src_dir.display()))?;

    let main_java = src_dir.join(format!("{main_class}.java"));
    if !main_java.exists() {
        let template = match args.template.as_str() {
            "lib" => lib_template(&main_class),
            "cli" => cli_template(&main_class),
            _     => app_template(&main_class),
        };
        std::fs::write(&main_java, template)
            .with_context(|| format!("cannot write {}", main_java.display()))?;
    }

    println!("  created  rava.hcl");
    println!("  created  src/{main_class}.java");
    println!();
    println!("Run your project:");
    println!("  rava run src/{main_class}.java");

    Ok(())
}

// ── Templates ─────────────────────────────────────────────────────────────────

fn app_template(class: &str) -> String {
    format!(
r#"public class {class} {{
    public static void main(String[] args) {{
        System.out.println("Hello from {class}!");
    }}
}}
"#
    )
}

fn lib_template(class: &str) -> String {
    format!(
r#"public class {class} {{
    public String greet(String name) {{
        return "Hello, " + name + "!";
    }}
}}
"#
    )
}

fn cli_template(class: &str) -> String {
    format!(
r#"public class {class} {{
    public static void main(String[] args) {{
        if (args.length == 0) {{
            System.out.println("Usage: {class} <name>");
            return;
        }}
        System.out.println("Hello, " + args[0] + "!");
    }}
}}
"#
    )
}

/// Convert `my-app` or `my_app` to `MyApp`.
fn to_pascal_case(s: &str) -> String {
    s.split(|c| c == '-' || c == '_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                None    => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_conversion() {
        assert_eq!(to_pascal_case("my-app"),   "MyApp");
        assert_eq!(to_pascal_case("my_app"),   "MyApp");
        assert_eq!(to_pascal_case("myapp"),    "Myapp");
        assert_eq!(to_pascal_case("hello"),    "Hello");
    }
}
