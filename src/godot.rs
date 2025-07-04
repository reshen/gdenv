use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GodotVersion {
    pub version: Version,
    pub is_dotnet: bool,
}

impl GodotVersion {
    /// Get the platform suffix for the current OS and architecture
    pub fn get_platform_suffix() -> &'static str {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match (os, arch) {
            ("windows", "x86_64") => "win64.exe",
            ("windows", "x86") => "win32.exe",
            ("macos", _) => "macos.universal", // macOS universal binaries work on both Intel and Apple Silicon
            ("linux", "x86_64") => "linux.x86_64",
            ("linux", "x86") => "linux.x86_32",
            ("linux", "arm") => "linux.arm32",
            ("linux", "aarch64") => "linux.arm64",
            // Fallbacks for common cases
            ("windows", _) => "win64.exe", // Default to 64-bit on Windows
            ("linux", _) => "linux.x86_64", // Default to x86_64 on Linux
            _ => "linux.x86_64",           // Ultimate fallback
        }
    }
    pub fn new(version_str: &str, is_dotnet: bool) -> Result<Self> {
        let normalized = Self::normalize_version_string(version_str)?;
        let version = Version::parse(&normalized)?;
        Ok(Self { version, is_dotnet })
    }

    /// Normalize Godot version strings to be semver compatible
    /// Examples:
    /// - "4.2.1" -> "4.2.1"
    /// - "4.3.0-beta2" -> "4.3.0-beta.2"
    /// - "4.1.0-rc.1" -> "4.1.0-rc.1"
    /// - "4.2.1-stable" -> "4.2.1"
    fn normalize_version_string(version_str: &str) -> Result<String> {
        let version_str = version_str.trim();

        // Remove common suffixes that aren't standard semver
        let cleaned = version_str.strip_suffix("-stable").unwrap_or(version_str);

        // Handle short versions like "4.3" -> "4.3.0" or "4.5-beta1" -> "4.5.0-beta1"
        let parts: Vec<&str> = cleaned.split('.').collect();
        let cleaned = if parts.len() == 2 {
            // Check if the second part is numeric or starts with a number followed by a prerelease
            let second_part = parts[1];
            if second_part.chars().all(|c| c.is_numeric()) {
                // Simple case: "4.3" -> "4.3.0"
                format!("{}.0", cleaned)
            } else if second_part.chars().next().is_some_and(|c| c.is_numeric()) {
                // Complex case: "4.5-beta1" -> "4.5.0-beta1"
                if let Some(dash_pos) = second_part.find('-') {
                    let (num_part, prerelease_part) = second_part.split_at(dash_pos);
                    if num_part.chars().all(|c| c.is_numeric()) {
                        format!("{}.{}.0{}", parts[0], num_part, prerelease_part)
                    } else {
                        cleaned.to_string()
                    }
                } else {
                    cleaned.to_string()
                }
            } else {
                cleaned.to_string()
            }
        } else {
            cleaned.to_string()
        };

        // Handle beta/rc versions to be semver compatible
        if cleaned.contains("-beta") && !cleaned.contains("-beta.") {
            // Convert "4.3.0-beta2" to "4.3.0-beta.2"
            if let Some((base, beta_part)) = cleaned.split_once("-beta") {
                if let Ok(beta_num) = beta_part.parse::<u32>() {
                    return Ok(format!("{}-beta.{}", base, beta_num));
                } else if beta_part.is_empty() {
                    return Ok(format!("{}-beta", base));
                }
            }
        }

        if cleaned.contains("-rc") && !cleaned.contains("-rc.") {
            // Convert "4.1.0-rc1" to "4.1.0-rc.1"
            if let Some((base, rc_part)) = cleaned.split_once("-rc") {
                if let Ok(rc_num) = rc_part.parse::<u32>() {
                    return Ok(format!("{}-rc.{}", base, rc_num));
                } else if rc_part.is_empty() {
                    return Ok(format!("{}-rc", base));
                }
            }
        }

        if cleaned.contains("-alpha") && !cleaned.contains("-alpha.") {
            // Convert "4.3.0-alpha1" to "4.3.0-alpha.1"
            if let Some((base, alpha_part)) = cleaned.split_once("-alpha") {
                if let Ok(alpha_num) = alpha_part.parse::<u32>() {
                    return Ok(format!("{}-alpha.{}", base, alpha_num));
                } else if alpha_part.is_empty() {
                    return Ok(format!("{}-alpha", base));
                }
            }
        }

        Ok(cleaned.to_string())
    }

    pub fn godot_version_string(&self) -> String {
        // Convert back to Godot's preferred format
        let version_str = self.version.to_string();

        // Convert semver format back to Godot format for display
        version_str
            .replace("-beta.", "-beta")
            .replace("-rc.", "-rc")
            .replace("-alpha.", "-alpha")
    }

    /// Get the expected executable path within the extracted directory
    pub fn get_executable_path(&self) -> String {
        let os = std::env::consts::OS;
        let _arch = std::env::consts::ARCH;

        match os {
            "macos" => {
                if self.is_dotnet {
                    "Godot_mono.app/Contents/MacOS/Godot".to_string()
                } else {
                    "Godot.app/Contents/MacOS/Godot".to_string()
                }
            }
            "windows" => {
                let version_part = if self.version.pre.is_empty() {
                    format!("{}-stable", self.version)
                } else {
                    self.godot_version_string()
                };

                if self.is_dotnet {
                    format!(
                        "Godot_v{}_mono_{}/Godot_v{}_mono_{}.exe",
                        version_part, "win64", version_part, "win64"
                    )
                } else {
                    format!("Godot_v{}_{}.exe", version_part, "win64")
                }
            }
            "linux" => {
                let version_part = if self.version.pre.is_empty() {
                    format!("{}-stable", self.version)
                } else {
                    self.godot_version_string()
                };

                let platform_suffix = Self::get_platform_suffix();

                if self.is_dotnet {
                    // Dotnet versions extract to a subfolder
                    let folder_name = format!("Godot_v{}_mono_{}", version_part, platform_suffix);
                    let exe_name = format!("Godot_v{}_mono_{}", version_part, platform_suffix);
                    format!("{}/{}", folder_name, exe_name)
                } else {
                    // Non-dotnet versions extract directly
                    format!("Godot_v{}_{}", version_part, platform_suffix)
                }
            }
            _ => {
                // Fallback - just look for Godot executable
                "Godot".to_string()
            }
        }
    }

    pub fn installation_name(&self) -> String {
        if self.is_dotnet {
            format!("godot-{}-dotnet", self.godot_version_string())
        } else {
            format!("godot-{}", self.godot_version_string())
        }
    }

    #[allow(dead_code)]
    pub fn archive_name(&self) -> String {
        let platform_suffix = Self::get_platform_suffix();

        let version_part = if self.version.pre.is_empty() {
            format!("{}-stable", self.version)
        } else {
            self.godot_version_string()
        };

        if self.is_dotnet {
            format!("Godot_v{}_mono_{}.zip", version_part, platform_suffix)
        } else {
            format!("Godot_v{}_{}.zip", version_part, platform_suffix)
        }
    }

    #[allow(dead_code)]
    pub fn is_prerelease(&self) -> bool {
        !self.version.pre.is_empty()
    }
}

impl FromStr for GodotVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        // Default to non-.NET version
        Self::new(s, false)
    }
}

impl fmt::Display for GodotVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dotnet {
            write!(f, "{} (.NET)", self.godot_version_string())
        } else {
            write!(f, "{}", self.godot_version_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        // Test stable versions
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        assert_eq!(v1.godot_version_string(), "4.2.1");
        assert!(!v1.is_prerelease());

        // Test stable with suffix
        let v2 = GodotVersion::new("4.2.1-stable", false).unwrap();
        assert_eq!(v2.godot_version_string(), "4.2.1");
        assert!(!v2.is_prerelease());

        // Test beta versions
        let v3 = GodotVersion::new("4.3.0-beta2", false).unwrap();
        assert_eq!(v3.godot_version_string(), "4.3.0-beta2");
        assert!(v3.is_prerelease());

        // Test rc versions
        let v4 = GodotVersion::new("4.1.0-rc.1", false).unwrap();
        assert_eq!(v4.godot_version_string(), "4.1.0-rc1");
        assert!(v4.is_prerelease());

        // Test .NET versions
        let v5 = GodotVersion::new("4.2.1", true).unwrap();
        assert_eq!(v5.to_string(), "4.2.1 (.NET)");
        assert_eq!(v5.installation_name(), "godot-4.2.1-dotnet");

        // Test short prerelease versions like "4.5-beta1"
        let v6 = GodotVersion::new("4.5-beta1", false).unwrap();
        assert_eq!(v6.godot_version_string(), "4.5.0-beta1");
        assert!(v6.is_prerelease());
    }

    #[test]
    fn test_archive_names() {
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        let archive = v1.archive_name();
        assert!(archive.contains("Godot_v4.2.1-stable_"));
        assert!(archive.ends_with(".zip"));

        let v2 = GodotVersion::new("4.3.0-beta2", true).unwrap();
        let archive = v2.archive_name();
        assert!(archive.contains("Godot_v4.3.0-beta2_mono_"));
        assert!(archive.ends_with(".zip"));
    }

    #[test]
    fn test_platform_suffix_detection() {
        // Test that we get a valid platform suffix (this tests the current system)
        let suffix = GodotVersion::get_platform_suffix();
        assert!(!suffix.is_empty());

        // Should be one of the expected patterns
        let valid_suffixes = [
            "win64.exe",
            "win32.exe",
            "macos.universal",
            "linux.x86_64",
            "linux.x86_32",
            "linux.arm32",
            "linux.arm64",
        ];
        assert!(
            valid_suffixes.contains(&suffix),
            "Got unexpected suffix: {}",
            suffix
        );
    }

    #[test]
    fn test_executable_path_construction() {
        // Test that we can construct executable paths
        let v1 = GodotVersion::new("4.2.1", false).unwrap();
        let exe_path = v1.get_executable_path();
        assert!(!exe_path.is_empty());

        let v2 = GodotVersion::new("4.2.1", true).unwrap();
        let dotnet_exe_path = v2.get_executable_path();
        assert!(!dotnet_exe_path.is_empty());

        // Paths should be different for dotnet vs non-dotnet
        assert_ne!(exe_path, dotnet_exe_path);
    }
}
