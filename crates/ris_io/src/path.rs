use std::path::Path;

pub fn sanitize(value: &str, sanitize_slashes: bool) -> String {
    const SLASHES: [char; 2] = ['\\', '/'];
    const INVALID_CHARS: [char; 7] = [':', '*', '?', '"', '<', '>', '|'];

    let mut value = String::from(value);
    if sanitize_slashes {
        for slash in SLASHES {
            value = value.replace(slash, "_");
        }
    }

    for invalid_char in INVALID_CHARS {
        value = value.replace(invalid_char, "_");
    }

    value
}

pub fn to_str(path: impl AsRef<Path>) -> String {
    let debug = format!("{:?}", path.as_ref());
    let without_quotes = debug.trim_matches('"');

    #[cfg(target_os = "windows")]
    {
        without_quotes.replace("\\\\", "\\").replace('/', "\\")
    }

    #[cfg(not(target_os = "windows"))]
    {
        without_quotes.to_string()
    }
}
