//! Short-name registry — maps aliases like `"spring-boot-web"` to full Maven coordinates.
//!
//! Built-in aliases match the table in §2.5 of the spec.
//! Users can extend the registry via `~/.rava/aliases.hcl`.

use std::collections::HashMap;

/// Maps short names to full `"groupId:artifactId"` coordinates.
pub struct ShortNameRegistry {
    aliases: HashMap<&'static str, &'static str>,
}

impl ShortNameRegistry {
    /// Build the registry with the built-in aliases from §2.5.
    pub fn builtin() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("spring-boot-web",      "org.springframework.boot:spring-boot-starter-web");
        aliases.insert("spring-boot-data-jpa", "org.springframework.boot:spring-boot-starter-data-jpa");
        aliases.insert("lombok",               "org.projectlombok:lombok");
        aliases.insert("guava",                "com.google.guava:guava");
        aliases.insert("jackson",              "com.fasterxml.jackson.core:jackson-databind");
        aliases.insert("slf4j",                "org.slf4j:slf4j-api");
        aliases.insert("logback",              "ch.qos.logback:logback-classic");
        aliases.insert("junit",                "org.junit.jupiter:junit-jupiter");
        aliases.insert("mockito",              "org.mockito:mockito-core");
        aliases.insert("assertj",              "org.assertj:assertj-core");
        Self { aliases }
    }

    /// Resolve a short name or pass through a full coordinate unchanged.
    pub fn resolve<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases.get(name).copied().unwrap_or(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_builtin_alias() {
        let reg = ShortNameRegistry::builtin();
        assert_eq!(
            reg.resolve("spring-boot-web"),
            "org.springframework.boot:spring-boot-starter-web"
        );
    }

    #[test]
    fn passthrough_for_full_coordinate() {
        let reg = ShortNameRegistry::builtin();
        assert_eq!(
            reg.resolve("com.example:my-lib"),
            "com.example:my-lib"
        );
    }
}
