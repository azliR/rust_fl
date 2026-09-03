use fl_rust::flutter::log_filter::LogFilter;

#[test]
fn test_empty_log_filter_without_defaults() {
    let filter = LogFilter::new(&[], false);
    assert!(!filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(!filter.should_ignore("Flutter run key commands"));
}

#[test]
fn test_default_noise_filter() {
    let filter = LogFilter::new(&[], true);
    assert!(filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(filter.should_ignore("E/gralloc(123): Empty SMPTE 2094-40 data"));
    assert!(!filter.should_ignore("Flutter run key commands"));
}

#[test]
fn test_filter_matching_substring_and_quotes() {
    let patterns = vec!["\"Empty SMPTE 2094-40 data\"".to_string()];
    let filter = LogFilter::new(&patterns, false);

    assert!(
        filter.should_ignore("2026/09/03 11:06:55 E/gralloc4(15578): Empty SMPTE 2094-40 data")
    );
    assert!(filter.should_ignore("empty smpte 2094-40 data"));
    assert!(!filter.should_ignore("Normal Flutter app log"));
}

#[test]
fn test_filter_matching_regex() {
    let patterns = vec!["gralloc\\d+".to_string()];
    let filter = LogFilter::new(&patterns, false);

    assert!(filter.should_ignore("E/gralloc4(15578): Some warning"));
    assert!(filter.should_ignore("E/GRALLOC12(123): Another warning"));
    assert!(!filter.should_ignore("E/other_component: Normal warning"));
}

#[test]
fn test_filter_strips_ansi_codes_before_matching() {
    let patterns = vec!["gralloc4".to_string()];
    let filter = LogFilter::new(&patterns, false);

    let colored_log = "\x1b[31mE/gralloc4(21192):\x1b[0m Empty SMPTE 2094-40 data";
    assert!(filter.should_ignore(colored_log));
}

#[test]
fn test_filter_multiple_patterns() {
    let patterns = vec!["gralloc4".to_string(), "W/AudioTrack".to_string()];
    let filter = LogFilter::new(&patterns, false);

    assert!(filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(filter.should_ignore("W/AudioTrack: underrun occurred"));
    assert!(!filter.should_ignore("App started successfully"));
}

#[test]
fn test_filter_handles_invalid_regex_gracefully() {
    let patterns = vec!["E/gralloc4(".to_string()];
    let filter = LogFilter::new(&patterns, false);

    assert!(filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(!filter.should_ignore("Different line"));
}
