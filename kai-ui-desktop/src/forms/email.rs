//! Correo electrónico: validación práctica (no RFC 5322 completa).

/// Sin espacios alrededor y con el dominio en minúsculas.
pub fn normalize(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.rsplit_once('@') {
        Some((local, domain)) => format!("{local}@{}", domain.to_lowercase()),
        None => trimmed.to_owned(),
    }
}

/// Un `@`; parte local de 1 a 64 caracteres sin espacios ni puntos al
/// borde ni `..`; dominio con al menos un punto, etiquetas de 1 a 63
/// caracteres `[A-Za-z0-9-]` sin guion al borde y la última de 2 o más
/// letras; máximo 254 en total. Ignora los espacios alrededor.
pub fn is_valid(raw: &str) -> bool {
    let email = raw.trim();
    if email.len() > 254 || email.matches('@').count() != 1 {
        return false;
    }
    let (local, domain) = email.split_once('@').expect("un @");
    let local_ok = (1..=64).contains(&local.len())
        && !local.chars().any(|c| c.is_whitespace() || c.is_control())
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..");
    if !local_ok || !domain.contains('.') {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    let labels_ok = labels.iter().all(|label| {
        (1..=63).contains(&label.len())
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    });
    let tld = labels.last().expect("etiqueta");
    labels_ok && tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_addresses() {
        for email in [
            "ana@ejemplo.cl",
            "ana.perez+kai@sub.ejemplo.com",
            "  ANA@Ejemplo.CL ",
            "a_b-c@mi-dominio.org",
            "x@a.io",
        ] {
            assert!(is_valid(email), "{email}");
        }
    }

    #[test]
    fn invalid_addresses() {
        let long_local = format!("{}@ejemplo.cl", "a".repeat(65));
        let long_total = format!("a@{}.cl", "b".repeat(63).repeat(4));
        for email in [
            "",
            "ana",
            "ana@",
            "@ejemplo.cl",
            "ana@@ejemplo.cl",
            "ana@ejemplo",
            "ana@ejemplo.c",
            "ana@ejemplo.c1",
            "ana pérez@ejemplo.cl",
            ".ana@ejemplo.cl",
            "ana.@ejemplo.cl",
            "an..a@ejemplo.cl",
            "ana@-ejemplo.cl",
            "ana@ejemplo-.cl",
            "ana@ejemplo..cl",
            "ana@ej_emplo.cl",
            long_local.as_str(),
            long_total.as_str(),
        ] {
            assert!(!is_valid(email), "{email}");
        }
    }

    #[test]
    fn normalize_lowercases_only_the_domain() {
        assert_eq!(normalize(" Ana@EJEMPLO.cl "), "Ana@ejemplo.cl");
    }
}
