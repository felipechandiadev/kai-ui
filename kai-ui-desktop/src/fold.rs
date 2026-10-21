//! Text compared the way people search it: without case or accents.

/// `text` lowercase, its vowels without accents (`ñ` stays).
pub(crate) fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn case_and_accents_do_not_count() {
        assert_eq!(fold("Valparaíso"), "valparaiso");
        assert_eq!(fold("ÁRICA Ü"), "arica u");
        assert_eq!(fold("Muñoz"), "muñoz");
    }
}
