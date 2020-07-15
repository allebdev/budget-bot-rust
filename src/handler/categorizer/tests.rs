use super::*;

fn fake_categorizer() -> Categorizer {
    let mut c = Categorizer::new();
    c.add_category(fake_category_sweets());
    c.add_category(fake_category_fruits());
    c.add_category(fake_category_others());
    c
}

fn fake_category_sweets() -> Category {
    Category {
        name: String::from("Sweets"),
        priority: 10,
        lexemes: "cand,sweet,chocolate".into(),
    }
}

fn fake_category_fruits() -> Category {
    Category {
        name: String::from("Fruits"),
        priority: 20,
        lexemes: "apple,banana,orange".into(),
    }
}

fn fake_category_others() -> Category {
    Category {
        name: String::from("Others"),
        priority: 99999,
        lexemes: "other,misc".into(),
    }
}

#[test]
fn classify_in_accord_with_priority() {
    let c = fake_categorizer();
    let category = c.classify_msg("10 for banana chocolates");
    assert_eq!(category.unwrap().name, "Sweets"); // because of priority
}

#[test]
fn classify_with_ignore_case() {
    let c = fake_categorizer();
    let category = c.classify_msg("10 for BANANA Chocolates");
    assert_eq!(category.unwrap().name, "Sweets");
}

#[test]
fn classify_default_category() {
    let mut c = Categorizer::new();
    c.add_category(fake_category_sweets());
    c.add_category(fake_category_fruits());
    c.add_category(fake_category_others());
    let category = c.classify_msg("10 for tea");
    assert_eq!(category.unwrap().name, "Others")
}

#[test]
fn category_match_word() {
    let category = fake_category_sweets();
    assert!(
        category.match_word("candy"),
        "candy should be treated as sweets"
    );
    assert!(
        !category.match_word("apple"),
        "apple shouldn't be treated as sweets"
    );
}

#[test]
fn category_match_word_ignore_case() {
    let category = fake_category_sweets();
    assert!(
        category.match_word("CanDy"),
        "candy should be treated as sweets"
    );
}
