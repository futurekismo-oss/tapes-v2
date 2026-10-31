#[cfg(test)]
mod tests {
    // Import everything from the outer module
    use super::*;

    #[test]
    fn test_parse_target_list_comma() {
        // Test comma-separated target names
        let input = vec!["hyprland,quickshell".to_string()];
        // Parse the input list
        let result = parse_target_list(&input);
        // Verify output matches parsed components
        assert_eq!(result, vec!["hyprland", "quickshell"]);
    }

    #[test]
    fn test_parse_target_list_with_whitespace() {
        // Test comma-separated target names with surrounding whitespace
        let input = vec![" hyprland , quickshell , waybar ".to_string()];
        // Parse the input list
        let result = parse_target_list(&input);
        // Verify whitespace was trimmed and targets split properly
        assert_eq!(result, vec!["hyprland", "quickshell", "waybar"]);
    }

    #[test]
    fn test_parse_target_list_colon_not_split() {
        // Verify colon is NOT treated as a delimiter when comma-only splitting is enforced
        let input = vec!["hyprland:quickshell".to_string()];
        // Parse the input list
        let result = parse_target_list(&input);
        // The colon string remains a single unsplit target name
        assert_eq!(result, vec!["hyprland:quickshell"]);
    }

    #[test]
    fn test_parse_target_list_empty_strings() {
        // Test input with only commas and empty strings
        let input = vec![", , ,".to_string(), "".to_string()];
        // Parse the input list
        let result = parse_target_list(&input);
        // Verify empty vector is returned
        assert!(result.is_empty());
    }

    #[test]
    fn test_should_include_target_default() {
        // Without include or exclude lists, all targets are included
        let included = should_include_target("hypr", None, None);
        // Verify true is returned
        assert!(included);
    }

    #[test]
    fn test_should_include_target_with_include() {
        // Create an include list containing hypr and waybar
        let includes = vec!["hypr".to_string(), "waybar".to_string()];
        // Check target present in include list
        assert!(should_include_target("hypr", Some(&includes), None));
        // Check target not present in include list
        assert!(!should_include_target("quickshell", Some(&includes), None));
    }

    #[test]
    fn test_should_include_target_with_exclude() {
        // Create an exclude list containing quickshell
        let excludes = vec!["quickshell".to_string()];
        // Check target not in exclude list
        assert!(should_include_target("hypr", None, Some(&excludes)));
        // Check target present in exclude list
        assert!(!should_include_target("quickshell", None, Some(&excludes)));
    }

    #[test]
    fn test_should_include_target_with_both() {
        // Create an include list with hypr and waybar
        let includes = vec!["hypr".to_string(), "waybar".to_string()];
        // Create an exclude list with waybar
        let excludes = vec!["waybar".to_string()];
        // Target in includes and not in excludes should pass
        assert!(should_include_target(
            "hypr",
            Some(&includes),
            Some(&excludes)
        ));
        // Target in includes but also in excludes should be rejected
        assert!(!should_include_target(
            "waybar",
            Some(&includes),
            Some(&excludes)
        ));
        // Target not in includes should be rejected
        assert!(!should_include_target(
            "quickshell",
            Some(&includes),
            Some(&excludes)
        ));
    }

    #[test]
    fn test_filter_tape_targets_simulation() {
        // List of candidate targets discovered in a tape directory
        let tape_targets = vec!["hypr", "quickshell", "waybar", "nvim"];
        // User requested include targets list
        let includes = vec!["hypr".to_string(), "quickshell".to_string()];
        // User requested exclude targets list
        let excludes = vec!["quickshell".to_string()];

        // Filter the targets using should_include_target
        let filtered: Vec<&str> = tape_targets
            // Convert to iterator
            .into_iter()
            // Filter each target against include and exclude lists
            .filter(|name| should_include_target(name, Some(&includes), Some(&excludes)))
            // Collect the resulting targets
            .collect();

        // Verify only hypr survived both filters
        assert_eq!(filtered, vec!["hypr"]);
    }
}
