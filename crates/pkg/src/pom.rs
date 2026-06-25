//! Minimal Maven POM parser — extracts a module's transitive dependencies.
//!
//! Best-effort string scanning (no XML-crate dependency). Handles the common cases:
//! literal coordinates, `${...}` property substitution from the same POM, and
//! scope/optional filtering. It does NOT yet resolve `<dependencyManagement>`, parent
//! POMs, or BOM imports — a dependency whose version cannot be resolved is skipped
//! rather than guessed.

use crate::registry::Dependency;
use std::collections::HashMap;

/// Extract the compile/runtime-scope, non-optional dependencies declared in a POM.
pub fn parse_pom_dependencies(pom_xml: &str) -> Vec<Dependency> {
    // Properties are read from the original (they may live in any section).
    let props = parse_properties(pom_xml);

    // Drop sections whose `<dependency>` entries are not direct project dependencies.
    let mut xml = pom_xml.to_string();
    for section in ["dependencyManagement", "build", "profiles", "reporting"] {
        xml = strip_section(&xml, section);
    }

    let mut out = Vec::new();
    for block in xml_blocks(&xml, "dependency") {
        let scope = tag_text(block, "scope").unwrap_or_default();
        if matches!(scope.as_str(), "test" | "provided" | "system") {
            continue;
        }
        if tag_text(block, "optional").as_deref() == Some("true") {
            continue;
        }
        let (g, a, v) = match (
            tag_text(block, "groupId"),
            tag_text(block, "artifactId"),
            tag_text(block, "version"),
        ) {
            (Some(g), Some(a), Some(v)) => (g, a, v),
            // No explicit version (managed elsewhere) → cannot resolve yet; skip.
            _ => continue,
        };
        let version = substitute(&v, &props);
        if version.contains("${") {
            continue; // unresolved property
        }
        out.push(Dependency::new(
            substitute(&g, &props),
            substitute(&a, &props),
            version,
        ));
    }
    out
}

fn parse_properties(xml: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    // `${project.version}` / `${project.groupId}` reference the POM's own coordinates.
    if let Some(v) = tag_text(xml, "version") {
        props.insert("project.version".into(), v);
    }
    if let Some(g) = tag_text(xml, "groupId") {
        props.insert("project.groupId".into(), g);
    }
    for block in xml_blocks(xml, "properties") {
        let mut rest = block;
        while let Some(open_start) = rest.find('<') {
            let after = &rest[open_start + 1..];
            let Some(name_end) = after.find('>') else { break };
            let name = &after[..name_end];
            if name.starts_with('/') || name.starts_with('!') || name.starts_with('?') {
                rest = &after[name_end + 1..];
                continue;
            }
            let close = format!("</{}>", name);
            let value_part = &after[name_end + 1..];
            if let Some(val_end) = value_part.find(&close) {
                props.insert(name.trim().to_string(), value_part[..val_end].trim().to_string());
                rest = &value_part[val_end + close.len()..];
            } else {
                rest = &after[name_end + 1..];
            }
        }
    }
    props
}

fn substitute(s: &str, props: &HashMap<String, String>) -> String {
    let mut out = s.to_string();
    // Bounded passes guard against self-referential properties.
    for _ in 0..5 {
        if !out.contains("${") {
            break;
        }
        let before = out.clone();
        for (k, v) in props {
            out = out.replace(&format!("${{{}}}", k), v);
        }
        if out == before {
            break; // nothing left we can resolve
        }
    }
    out
}

/// Inner text of each `<tag>...</tag>` region, in document order.
fn xml_blocks<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let mut blocks = Vec::new();
    let mut rest = xml;
    while let Some(s) = rest.find(&open) {
        let after = &rest[s + open.len()..];
        match after.find(&close) {
            Some(e) => {
                blocks.push(&after[..e]);
                rest = &after[e + close.len()..];
            }
            None => break,
        }
    }
    blocks
}

fn tag_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let s = block.find(&open)? + open.len();
    let e = block[s..].find(&close)?;
    Some(block[s..s + e].trim().to_string())
}

/// Remove every `<tag>...</tag>` region from `xml`.
fn strip_section(xml: &str, tag: &str) -> String {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(s) = rest.find(&open) {
        out.push_str(&rest[..s]);
        let after = &rest[s + open.len()..];
        match after.find(&close) {
            Some(e) => rest = &after[e + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const POM: &str = r#"
<project>
  <groupId>com.example</groupId>
  <artifactId>app</artifactId>
  <version>1.0.0</version>
  <properties>
    <junit.version>5.10.1</junit.version>
    <guava.version>33.0.0-jre</guava.version>
  </properties>
  <dependencyManagement>
    <dependencies>
      <dependency>
        <groupId>org.managed</groupId><artifactId>managed</artifactId><version>9.9.9</version>
      </dependency>
    </dependencies>
  </dependencyManagement>
  <dependencies>
    <dependency>
      <groupId>com.google.guava</groupId><artifactId>guava</artifactId><version>${guava.version}</version>
    </dependency>
    <dependency>
      <groupId>org.junit.jupiter</groupId><artifactId>junit-jupiter</artifactId><version>${junit.version}</version><scope>test</scope>
    </dependency>
    <dependency>
      <groupId>org.optional</groupId><artifactId>opt</artifactId><version>1.0</version><optional>true</optional>
    </dependency>
    <dependency>
      <groupId>org.slf4j</groupId><artifactId>slf4j-api</artifactId><version>2.0.9</version>
    </dependency>
    <dependency>
      <groupId>org.nover</groupId><artifactId>managed-noversion</artifactId>
    </dependency>
  </dependencies>
</project>
"#;

    #[test]
    fn extracts_compile_deps_with_property_substitution() {
        let got: Vec<String> = parse_pom_dependencies(POM)
            .iter()
            .map(|d| format!("{}:{}:{}", d.group_id, d.artifact_id, d.version))
            .collect();
        assert_eq!(
            got,
            vec![
                "com.google.guava:guava:33.0.0-jre".to_string(),
                "org.slf4j:slf4j-api:2.0.9".to_string(),
            ]
        );
    }

    #[test]
    fn project_version_property() {
        let pom = r#"<project><groupId>g</groupId><version>2.5</version>
          <dependencies><dependency>
            <groupId>g</groupId><artifactId>sibling</artifactId><version>${project.version}</version>
          </dependency></dependencies></project>"#;
        let deps = parse_pom_dependencies(pom);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "2.5");
    }

    #[test]
    fn empty_when_no_dependencies() {
        assert!(parse_pom_dependencies("<project></project>").is_empty());
    }
}
