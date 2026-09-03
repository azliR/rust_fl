use fl_rust::flutter::log_filter::LogFilter;

#[test]
fn test_empty_log_filter() {
    let filter = LogFilter::new(&[]);
    assert!(!filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(!filter.should_ignore("Flutter run key commands"));
}

#[test]
fn test_filter_matching_substring() {
    let patterns = vec!["Empty SMPTE 2094-40 data".to_string()];
    let filter = LogFilter::new(&patterns);

    assert!(
        filter.should_ignore("2026/09/03 11:06:55 E/gralloc4(15578): Empty SMPTE 2094-40 data")
    );
    assert!(filter.should_ignore("empty smpte 2094-40 data"));
    assert!(!filter.should_ignore("Normal Flutter app log"));
}

#[test]
fn test_filter_matching_regex() {
    let patterns = vec!["gralloc\\d+".to_string()];
    let filter = LogFilter::new(&patterns);

    assert!(filter.should_ignore("E/gralloc4(15578): Some warning"));
    assert!(filter.should_ignore("E/GRALLOC12(123): Another warning"));
    assert!(!filter.should_ignore("E/other_component: Normal warning"));
}

#[test]
fn test_filter_multiple_patterns() {
    let patterns = vec!["gralloc4".to_string(), "W/AudioTrack".to_string()];
    let filter = LogFilter::new(&patterns);

    assert!(filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(filter.should_ignore("W/AudioTrack: underrun occurred"));
    assert!(!filter.should_ignore("App started successfully"));
}

#[test]
fn test_filter_handles_invalid_regex_gracefully() {
    let patterns = vec!["E/gralloc4(".to_string()];
    let filter = LogFilter::new(&patterns);

    assert!(filter.should_ignore("E/gralloc4(15578): Empty SMPTE 2094-40 data"));
    assert!(!filter.should_ignore("Different line"));
}
