use rand::seq::SliceRandom;
// use rand::Rng;

const CHROME_VERSIONS: &[&str] = &[
    "120.0.0.0", "121.0.0.0", "122.0.0.0", "123.0.0.0",
    "124.0.0.0", "125.0.0.0", "126.0.0.0", "127.0.0.0",
    "128.0.0.0", "129.0.0.0", "130.0.0.0", "131.0.0.0",
    "132.0.0.0", "133.0.0.0", "134.0.0.0", "135.0.0.0",
];

const WEBKIT_VERSION: &str = "537.36";

const WINDOWS_PLATFORMS: &[&str] = &[
    "Windows NT 10.0; Win64; x64",
    "Windows NT 10.0; WOW64",
    "Windows NT 11.0; Win64; x64",
];

const LINUX_PLATFORMS: &[&str] = &[
    "X11; Linux x86_64",
    "X11; Ubuntu; Linux x86_64",
    "X11; Fedora; Linux x86_64",
];

const MAC_PLATFORMS: &[&str] = &[
    "Macintosh; Intel Mac OS X 10_15_7",
    "Macintosh; Intel Mac OS X 13_0",
    "Macintosh; Intel Mac OS X 14_0",
];

/// Генерация Chrome User-Agent.
pub fn generate_user_agent() -> String {
    let mut rng = rand::thread_rng();

    let all_platforms: Vec<&str> = WINDOWS_PLATFORMS
        .iter()
        .chain(LINUX_PLATFORMS.iter())
        .chain(MAC_PLATFORMS.iter())
        .copied()
        .collect();

    let platform = all_platforms.choose(&mut rng).unwrap();
    let chrome   = CHROME_VERSIONS.choose(&mut rng).unwrap();

    format!(
        "Mozilla/5.0 ({}) AppleWebKit/{} (KHTML, like Gecko) Chrome/{} Safari/{}",
        platform, WEBKIT_VERSION, chrome, WEBKIT_VERSION
    )
}