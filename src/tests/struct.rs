#[cfg(test)]
mod tests {
    // Import definitions from parent module
    use super::*;

    #[test]
    fn test_insert_command_args_include() {
        // Parse CLI arguments with --include-targets using commas
        let cli = Cli::try_parse_from([
            "tape",
            "insert",
            "caelestia",
            "--include-targets",
            "hyprland,quickshell",
        ])
        .expect("CLI arguments should parse successfully");

        // Verify the command is Insert and check parsed include targets
        if let Commands::Insert {
            name,
            include_targets,
            exclude_targets,
            ..
        } = cli.command
        {
            // Verify tape name
            assert_eq!(name, Some("caelestia".to_string()));
            // Verify include targets parsed into list
            assert_eq!(
                include_targets,
                Some(vec!["hyprland".to_string(), "quickshell".to_string()])
            );
            // Verify exclude targets is None
            assert_eq!(exclude_targets, None);
        } else {
            // Fail if not Insert command
            panic!("Expected Commands::Insert");
        }
    }

    #[test]
    fn test_insert_command_args_exclude() {
        // Parse CLI arguments with --exclude-targets
        let cli = Cli::try_parse_from([
            "tape",
            "insert",
            "caelestia",
            "--exclude-targets",
            "quickshell",
        ])
        .expect("CLI arguments should parse successfully");

        // Verify the command is Insert and check parsed exclude targets
        if let Commands::Insert {
            name,
            include_targets,
            exclude_targets,
            ..
        } = cli.command
        {
            // Verify tape name
            assert_eq!(name, Some("caelestia".to_string()));
            // Verify include targets is None
            assert_eq!(include_targets, None);
            // Verify exclude targets parsed
            assert_eq!(exclude_targets, Some(vec!["quickshell".to_string()]));
        } else {
            // Fail if not Insert command
            panic!("Expected Commands::Insert");
        }
    }

    #[test]
    fn test_insert_command_args_multiple_flags() {
        // Parse CLI arguments with multiple separate --include-targets flags
        let cli = Cli::try_parse_from([
            "tape",
            "insert",
            "caelestia",
            "--include-targets",
            "hyprland",
            "--include-targets",
            "quickshell",
        ])
        .expect("CLI arguments should parse successfully");

        // Verify the command is Insert and check accumulated include targets
        if let Commands::Insert {
            include_targets, ..
        } = cli.command
        {
            // Verify multiple flags were accumulated into a list
            assert_eq!(
                include_targets,
                Some(vec!["hyprland".to_string(), "quickshell".to_string()])
            );
        } else {
            // Fail if not Insert command
            panic!("Expected Commands::Insert");
        }
    }
}

