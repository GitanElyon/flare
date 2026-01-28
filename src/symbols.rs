use std::sync::LazyLock;

pub static SYMBOLS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
    let json = include_str!("../symbols.json");
    let data: Vec<(String, String)> = serde_json::from_str(json).expect("Failed to parse symbols.json");
    data.into_iter()
        .map(|(a, b)| (
            Box::leak(a.into_boxed_str()) as &'static str,
            Box::leak(b.into_boxed_str()) as &'static str
        ))
        .collect()
});
