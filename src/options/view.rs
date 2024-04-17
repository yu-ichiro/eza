use crate::output::TerminalWidth::Automatic;
use std::ffi::OsString;

use crate::fs::feature::xattr;
use crate::options::parser::{ColorScaleModeArgs, Opts};
use crate::options::{vars, NumberSource, OptionsError, Vars};
use crate::output::color_scale::{ColorScaleMode, ColorScaleOptions};
use crate::output::file_name::Options as FileStyle;
use crate::output::grid_details::{self, RowThreshold};
use crate::output::table::{
    Columns, FlagsFormat, GroupFormat, Options as TableOptions, SizeFormat, TimeTypes, UserFormat,
};
use crate::output::time::TimeFormat;
use crate::output::TerminalWidth::Set;
use crate::output::{details, grid, Mode, TerminalWidth, View};

impl View {
    pub fn deduce<V: Vars>(matches: &Opts, vars: &V, strict: bool) -> Result<Self, OptionsError> {
        let mode = Mode::deduce(matches, vars, strict)?;
        let deref_links = matches.dereference > 0;
        let total_size = matches.total_size > 0;
        let width = TerminalWidth::deduce(matches, vars)?;
        let file_style = FileStyle::deduce(matches, vars, width.actual_terminal_width().is_some())?;
        Ok(Self {
            mode,
            width,
            file_style,
            deref_links,
            total_size,
        })
    }
}

impl Mode {
    /// Determine which viewing mode to use based on the user’s options.
    ///
    /// As with the other options, arguments are scanned right-to-left and the
    /// first flag found is matched, so `exa --oneline --long` will pick a
    /// details view, and `exa --long --oneline` will pick the lines view.
    ///
    /// This is complicated a little by the fact that `--grid` and `--tree`
    /// can also combine with `--long`, so care has to be taken to use the
    pub fn deduce<V: Vars>(matches: &Opts, vars: &V, strict: bool) -> Result<Self, OptionsError> {
        if !(matches.long > 0 || matches.oneline > 0 || matches.grid > 0 || matches.tree > 0) {
            if strict {
                Self::strict_check_long_flags(matches)?;
            }
            let grid = grid::Options::deduce(matches);
            return Ok(Self::Grid(grid));
        };

        if matches.long > 0 {
            let details = details::Options::deduce_long(matches, vars, strict)?;

            if matches.grid > 0 {
                let grid = grid::Options::deduce(matches);
                let row_threshold = RowThreshold::deduce(vars)?;
                let grid_details = grid_details::Options {
                    details,
                    row_threshold,
                };
                return Ok(Self::GridDetails(grid_details));
            }

            // the --tree case is handled by the DirAction parser later
            return Ok(Self::Details(details));
        }

        if strict {
            Self::strict_check_long_flags(matches)?;
        }

        if matches.tree > 0 {
            let details = details::Options::deduce_tree(matches, vars)?;
            return Ok(Self::Details(details));
        }

        if matches.oneline > 0 {
            return Ok(Self::Lines);
        }

        let grid = grid::Options::deduce(matches);
        Ok(Self::Grid(grid))
    }

    fn strict_check_long_flags(matches: &Opts) -> Result<(), OptionsError> {
        // If --long hasn’t been passed, then check if we need to warn the
        // user about flags that won’t have any effect.
        for option in &[
            (matches.binary > 0, "binary"),
            (matches.bytes > 0, "bytes"),
            (matches.inode > 0, "inode"),
            (matches.links > 0, "links"),
            (matches.header > 0, "header"),
            (matches.blocksize > 0, "blocksize"),
            (matches.time.is_some(), "time"),
            (matches.group > 0, "group"),
            (matches.numeric > 0, "numeric"),
            (matches.mounts > 0, "mounts"),
        ] {
            let (opt, name) = option;
            if *opt {
                return Err(OptionsError::Useless(name, false, "long"));
            }
        }

        if matches.git > 0 && matches.no_git == 0 {
            return Err(OptionsError::Useless("git", false, "long"));
        } else if matches.level.is_some() && matches.recurse == 0 && matches.tree == 0 {
            return Err(OptionsError::Useless2("level", "recurse", "tree"));
        }

        Ok(())
    }
}

impl grid::Options {
    fn deduce(matches: &Opts) -> Self {
        grid::Options {
            across: matches.across > 0,
        }
    }
}

impl details::Options {
    fn deduce_tree<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        let details = details::Options {
            table: None,
            header: false,
            xattr: xattr::ENABLED && matches.extended > 0,
            secattr: xattr::ENABLED && matches.security_context > 0,
            mounts: matches.mounts > 0,
            color_scale: ColorScaleOptions::deduce(matches, vars)?,
        };

        Ok(details)
    }

    fn deduce_long<V: Vars>(matches: &Opts, vars: &V, strict: bool) -> Result<Self, OptionsError> {
        if strict {
            if matches.across > 0 && matches.grid == 0 {
                return Err(OptionsError::Useless("across", true, "long"));
            } else if matches.oneline > 0 {
                return Err(OptionsError::Useless("one-line", true, "long"));
            }
        }

        Ok(details::Options {
            table: Some(TableOptions::deduce(matches, vars)?),
            header: matches.header > 0,
            xattr: xattr::ENABLED && matches.extended > 0,
            secattr: xattr::ENABLED && matches.security_context > 0,
            mounts: matches.mounts > 0,
            color_scale: ColorScaleOptions::deduce(matches, vars)?,
        })
    }
}

impl TerminalWidth {
    fn deduce<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        if let Some(width) = matches.width {
            if width >= 1 {
                Ok(Set(width))
            } else {
                Ok(Automatic)
            }
        } else if let Some(columns) = vars.get(vars::COLUMNS).and_then(|s| s.into_string().ok()) {
            match columns.parse() {
                Ok(width) => Ok(Set(width)),
                Err(e) => {
                    let source = NumberSource::Env(vars::COLUMNS);
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        } else {
            Ok(Automatic)
        }
    }
}

impl RowThreshold {
    fn deduce<V: Vars>(vars: &V) -> Result<Self, OptionsError> {
        if let Some(columns) = vars
            .get_with_fallback(vars::EZA_GRID_ROWS, vars::EXA_GRID_ROWS)
            .and_then(|s| s.into_string().ok())
        {
            match columns.parse() {
                Ok(rows) => Ok(Self::MinimumRows(rows)),
                Err(e) => {
                    let source = NumberSource::Env(
                        vars.source(vars::EZA_GRID_ROWS, vars::EXA_GRID_ROWS)
                            .unwrap(),
                    );
                    Err(OptionsError::FailedParse(columns, source, e))
                }
            }
        } else {
            Ok(Self::AlwaysGrid)
        }
    }
}

impl TableOptions {
    fn deduce<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        let time_format = TimeFormat::deduce(matches, vars)?;
        let flags_format = FlagsFormat::deduce(vars);
        let size_format = SizeFormat::deduce(matches);
        let user_format = UserFormat::deduce(matches);
        let group_format = GroupFormat::deduce(matches);
        let columns = Columns::deduce(matches, vars)?;
        Ok(Self {
            size_format,
            time_format,
            user_format,
            group_format,
            flags_format,
            columns,
        })
    }
}

impl Columns {
    fn deduce<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        let time_types = TimeTypes::deduce(matches)?;

        let no_git_env = vars
            .get_with_fallback(vars::EXA_OVERRIDE_GIT, vars::EZA_OVERRIDE_GIT)
            .is_some();

        let git = matches.git > 0 && matches.no_git == 0 && !no_git_env;
        let subdir_git_repos = matches.git_repos > 0 && matches.no_git == 0 && !no_git_env;
        let subdir_git_repos_no_stat = !subdir_git_repos
            && matches.git_repos_no_status > 0
            && matches.no_git == 0
            && !no_git_env;

        let file_flags = matches.file_flags > 0;
        let blocksize = matches.blocksize > 0;
        let group = matches.group > 0;
        let inode = matches.inode > 0;
        let links = matches.links > 0;
        let octal = matches.octal > 0;
        let security_context = xattr::ENABLED && matches.security_context > 0;

        let permissions = matches.no_permissions == 0;
        let filesize = matches.no_filesize == 0;
        let user = matches.no_user == 0;

        Ok(Self {
            time_types,
            inode,
            links,
            blocksize,
            group,
            git,
            subdir_git_repos,
            subdir_git_repos_no_stat,
            octal,
            security_context,
            file_flags,
            permissions,
            filesize,
            user,
        })
    }
}

impl SizeFormat {
    /// Determine which file size to use in the file size column based on
    /// the user’s options.
    ///
    /// The default mode is to use the decimal prefixes, as they are the
    /// most commonly-understood, and don’t involve trying to parse large
    /// strings of digits in your head. Changing the format to anything else
    /// involves the `--binary` or `--bytes` flags, and these conflict with
    /// each other.
    fn deduce(matches: &Opts) -> Self {
        use SizeFormat::*;
        if matches.binary > 0 {
            BinaryBytes
        } else if matches.bytes > 0 {
            JustBytes
        } else {
            DecimalBytes
        }
    }
}

impl TimeFormat {
    /// Determine how time should be formatted in timestamp columns.
    fn deduce<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        let word = if let Some(w) = &matches.time_style {
            w.clone()
        } else {
            match vars.get(vars::TIME_STYLE) {
                Some(ref t) if !t.is_empty() => t.clone(),
                _ => return Ok(Self::DefaultFormat),
            }
        };

        match word.to_string_lossy().as_ref() {
            "default" => Ok(Self::DefaultFormat),
            "relative" => Ok(Self::Relative),
            "iso" => Ok(Self::ISOFormat),
            "long-iso" => Ok(Self::LongISO),
            "full-iso" => Ok(Self::FullISO),
            fmt if fmt.starts_with('+') => {
                let mut lines = fmt[1..].lines();

                // line 1 will be None when:
                //   - there is nothing after `+`
                // line 1 will be empty when:
                //   - `+` is followed immediately by `\n`
                let empty_non_recent_format_msg = "Custom timestamp format is empty, \
                    please supply a chrono format string after the plus sign.";
                let non_recent = lines.next().expect(empty_non_recent_format_msg);
                let non_recent = if non_recent.is_empty() {
                    panic!("{}", empty_non_recent_format_msg)
                } else {
                    non_recent.to_owned()
                };

                // line 2 will be None when:
                //   - there is not a single `\n`
                //   - there is nothing after the first `\n`
                // line 2 will be empty when:
                //   - there exist at least 2 `\n`, and no content between the 1st and 2nd `\n`
                let empty_recent_format_msg = "Custom timestamp format for recent files is empty, \
                    please supply a chrono format string at the second line.";
                let recent = lines.next().map(|rec| {
                    if rec.is_empty() {
                        panic!("{}", empty_recent_format_msg)
                    } else {
                        rec.to_owned()
                    }
                });

                Ok(Self::Custom { non_recent, recent })
            }
            _ => Err(OptionsError::BadArgument("time-style", word)),
        }
    }
}

impl UserFormat {
    fn deduce(matches: &Opts) -> Self {
        let flag = matches.numeric > 0;
        if flag {
            Self::Numeric
        } else {
            Self::Name
        }
    }
}

impl GroupFormat {
    fn deduce(matches: &Opts) -> Self {
        let flag = matches.smart_group > 0;
        if flag {
            Self::Smart
        } else {
            Self::Regular
        }
    }
}

impl TimeTypes {
    /// Determine which of a file’s time fields should be displayed for it
    /// based on the user’s options.
    ///
    /// There are two separate ways to pick which fields to show: with a
    /// flag (such as `--modified`) or with a parameter (such as
    /// `--time=modified`). An error is signaled if both ways are used.
    ///
    /// It’s valid to show more than one column by passing in more than one
    /// option, but passing *no* options means that the user just wants to
    /// see the default set.
    fn deduce(matches: &Opts) -> Result<Self, OptionsError> {
        let possible_word = &matches.time;
        let modified = matches.modified > 0;
        let changed = matches.changed > 0;
        let accessed = matches.accessed > 0;
        let created = matches.created > 0;

        let no_time = matches.no_time > 0;

        #[rustfmt::skip]
        let time_types = if no_time {
            Self {
                modified: false,
                changed: false,
                accessed: false,
                created: false,
            }
        } else if let Some(word) = possible_word {
            if modified {
                return Err(OptionsError::Useless("modified", true, "time"));
            } else if changed {
                return Err(OptionsError::Useless("changed", true, "time"));
            } else if accessed {
                return Err(OptionsError::Useless("accessed", true, "time"));
            } else if created {
                return Err(OptionsError::Useless("created", true, "time"));
            } else if word == "mod" || word == "modified" {
                Self { modified: true,  changed: false, accessed: false, created: false }
            } else if word == "ch" || word == "changed" {
                Self { modified: false, changed: true,  accessed: false, created: false }
            } else if word == "acc" || word == "accessed" {
                Self { modified: false, changed: false, accessed: true,  created: false }
            } else if word == "cr" || word == "created" {
                Self { modified: false, changed: false, accessed: false, created: true  }
            } else {
                return Err(OptionsError::BadArgument("time", word.into()));
            }
        } else if modified || changed || accessed || created {
            Self {
                modified,
                changed,
                accessed,
                created,
            }
        } else {
            Self::default()
        };

        Ok(time_types)
    }
}

impl ColorScaleOptions {
    pub fn deduce<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        let min_luminance =
            match vars.get_with_fallback(vars::EZA_MIN_LUMINANCE, vars::EXA_MIN_LUMINANCE) {
                Some(var) => match var.to_string_lossy().parse() {
                    Ok(luminance) if (-100..=100).contains(&luminance) => luminance,
                    _ => 40,
                },
                None => 40,
            };

        let mode = match matches.color_scale_mode {
            ColorScaleModeArgs::Fixed => ColorScaleMode::Fixed,
            ColorScaleModeArgs::Gradient => ColorScaleMode::Gradient,
        };

        let mut options = ColorScaleOptions {
            mode,
            min_luminance,
            size: false,
            age: false,
        };

        let words = if let Some(w) = match &matches.color_scale {
            Some(w) => Some(w),
            None => None,
        } {
            w.clone()
        } else {
            return Ok(options);
        };

        for word in words.to_string_lossy().split(',') {
            match word {
                "all" => {
                    options.size = true;
                    options.age = true;
                }
                "age" => options.age = true,
                "size" => options.size = true,
                _ => Err(OptionsError::BadArgument(
                    "color-scale",
                    OsString::from(word),
                ))?,
            };
        }

        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use crate::options::vars::MockVars;
    use std::num::ParseIntError;

    use super::*;

    #[test]
    fn deduce_time_types_no_time() {
        let options = Opts {
            no_time: 1,
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_default() {
        assert_eq!(
            TimeTypes::deduce(&Opts::default()),
            Ok(TimeTypes::default())
        );
    }

    #[test]
    fn deduce_time_types_modified_word() {
        let options = Opts {
            time: Some(OsString::from("modified")),
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_accessed_word() {
        let options = Opts {
            time: Some(OsString::from("accessed")),
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                accessed: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_changed_word() {
        let options = Opts {
            time: Some(OsString::from("changed")),
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: false,
                changed: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_created_word() {
        let options = Opts {
            time: Some(OsString::from("created")),
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: false,
                created: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_modified() {
        let options = Opts {
            modified: 1,
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_accessed() {
        let options = Opts {
            accessed: 1,
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                accessed: true,
                modified: false,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_changed() {
        let options = Opts {
            changed: 1,
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: false,
                changed: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_time_types_created() {
        let options = Opts {
            created: 1,
            ..Opts::default()
        };

        assert_eq!(
            TimeTypes::deduce(&options),
            Ok(TimeTypes {
                modified: false,
                created: true,
                ..TimeTypes::default()
            })
        );
    }

    #[test]
    fn deduce_group_format_on() {
        let options = Opts {
            smart_group: 1,
            ..Opts::default()
        };

        assert_eq!(GroupFormat::deduce(&options), GroupFormat::Smart);
    }

    #[test]
    fn deduce_group_format_off() {
        let options = Opts { ..Opts::default() };

        assert_eq!(GroupFormat::deduce(&options), GroupFormat::Regular);
    }

    #[test]
    fn deduce_user_format_on() {
        let options = Opts {
            numeric: 1,
            ..Opts::default()
        };

        assert_eq!(UserFormat::deduce(&options), UserFormat::Numeric);
    }

    #[test]
    fn deduce_user_format_off() {
        let options = Opts { ..Opts::default() };

        assert_eq!(UserFormat::deduce(&options), UserFormat::Name);
    }

    #[test]
    fn deduce_size_format_off() {
        let options = Opts { ..Opts::default() };

        assert_eq!(SizeFormat::deduce(&options), SizeFormat::DecimalBytes);
    }

    #[test]
    fn deduce_user_format_bytes() {
        let options = Opts {
            bytes: 1,
            ..Opts::default()
        };

        assert_eq!(SizeFormat::deduce(&options), SizeFormat::JustBytes);
    }

    #[test]
    fn deduce_user_format_binary() {
        let options = Opts {
            binary: 1,
            ..Opts::default()
        };

        assert_eq!(SizeFormat::deduce(&options), SizeFormat::BinaryBytes);
    }

    #[test]
    fn deduce_grid_options() {
        let options = Opts {
            across: 1,
            ..Opts::default()
        };

        assert_eq!(
            grid::Options::deduce(&options),
            grid::Options { across: true }
        );
    }

    #[test]
    fn deduce_time_style_iso_env() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts { ..Opts::default() };

        vars.set(vars::TIME_STYLE, &OsString::from("iso"));
        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::ISOFormat)
        );
    }

    #[test]
    fn deduce_time_style_iso_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from("iso")),
            ..Opts::default()
        };

        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::ISOFormat)
        );
    }

    #[test]
    fn deduce_time_style_long_iso_env() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts { ..Opts::default() };

        vars.set(vars::TIME_STYLE, &OsString::from("long-iso"));
        assert_eq!(TimeFormat::deduce(&options, &vars), Ok(TimeFormat::LongISO));
    }

    #[test]
    fn deduce_time_style_long_iso_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from("long-iso")),
            ..Opts::default()
        };

        assert_eq!(TimeFormat::deduce(&options, &vars), Ok(TimeFormat::LongISO));
    }

    #[test]
    fn deduce_time_style_full_iso_env() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts { ..Opts::default() };

        vars.set(vars::TIME_STYLE, &OsString::from("full-iso"));
        assert_eq!(TimeFormat::deduce(&options, &vars), Ok(TimeFormat::FullISO));
    }

    #[test]
    fn deduce_time_style_full_iso_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from("full-iso")),
            ..Opts::default()
        };

        assert_eq!(TimeFormat::deduce(&options, &vars), Ok(TimeFormat::FullISO));
    }

    #[test]
    fn deduce_time_style_relative_env() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts { ..Opts::default() };

        vars.set(vars::TIME_STYLE, &OsString::from("relative"));
        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::Relative)
        );
    }

    #[test]
    fn deduce_time_style_relative_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from("relative")),
            ..Opts::default()
        };

        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::Relative)
        );
    }

    #[test]
    fn deduce_time_style_custom_env() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts { ..Opts::default() };

        vars.set(vars::TIME_STYLE, &OsString::from("+%Y-%b-%d"));
        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::Custom {
                recent: None,
                non_recent: String::from("%Y-%b-%d")
            })
        );
    }

    #[test]
    fn deduce_time_style_custom_arg() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from("+%Y-%b-%d")),
            ..Opts::default()
        };

        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::Custom {
                recent: None,
                non_recent: String::from("%Y-%b-%d")
            })
        );
    }

    #[test]
    fn deduce_time_style_non_recent_and_recent() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from(
                "+%Y-%m-%d %H
--%m-%d %H:%M",
            )),
            ..Opts::default()
        };

        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Ok(TimeFormat::Custom {
                recent: Some(String::from("--%m-%d %H:%M")),
                non_recent: String::from("%Y-%m-%d %H")
            })
        );
    }

    #[test]
    fn deduce_time_style_error() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            time_style: Some(OsString::from("nice")),
            ..Opts::default()
        };

        assert_eq!(
            TimeFormat::deduce(&options, &vars),
            Err(OptionsError::BadArgument(
                "time-style",
                OsString::from("nice")
            ))
        );
    }

    #[test]
    fn deduce_color_scale_size_age_luminance_40_gradient() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            color_scale: Some(OsString::from("size,age")),
            ..Opts::default()
        };

        assert_eq!(
            ColorScaleOptions::deduce(&options, &vars),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 40,
                size: true,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_color_scale_size_luminance_60_gradient() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            color_scale: Some(OsString::from("size")),
            ..Opts::default()
        };

        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("60"));

        assert_eq!(
            ColorScaleOptions::deduce(&options, &vars),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Gradient,
                min_luminance: 60,
                size: true,
                age: false,
            })
        );
    }

    #[test]
    fn deduce_color_scale_age_luminance_60_fixed() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            color_scale_mode: ColorScaleModeArgs::Fixed,
            color_scale: Some(OsString::from("age")),
            ..Opts::default()
        };

        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("60"));

        assert_eq!(
            ColorScaleOptions::deduce(&options, &vars),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Fixed,
                min_luminance: 60,
                size: false,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_color_scale_size_age_luminance_99_fixed() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            color_scale_mode: ColorScaleModeArgs::Fixed,
            color_scale: Some(OsString::from("size,age")),
            ..Opts::default()
        };

        vars.set(vars::EZA_MIN_LUMINANCE, &OsString::from("99"));

        assert_eq!(
            ColorScaleOptions::deduce(&options, &vars),
            Ok(ColorScaleOptions {
                mode: ColorScaleMode::Fixed,
                min_luminance: 99,
                size: true,
                age: true,
            })
        );
    }

    #[test]
    fn deduce_mode_grid() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            grid: 1,
            ..Opts::default()
        };

        assert_eq!(
            Mode::deduce(&options, &vars, false),
            Ok(Mode::Grid(grid::Options { across: false }))
        );
    }

    #[test]
    fn deduce_mode_grid_across() {
        let vars = MockVars {
            ..MockVars::default()
        };

        let options = Opts {
            grid: 1,
            across: 1,
            ..Opts::default()
        };

        assert_eq!(
            Mode::deduce(&options, &vars, false),
            Ok(Mode::Grid(grid::Options { across: true }))
        );
    }
    /*
    fn deduce_tree<V: Vars>(matches: &Opts, vars: &V) -> Result<Self, OptionsError> {
        let details = details::Options {
            table: None,
            header: false,
            xattr: xattr::ENABLED && matches.extended > 0,
            secattr: xattr::ENABLED && matches.security_context > 0,
            mounts: matches.mounts > 0,
            color_scale: ColorScaleOptions::deduce(matches, vars)?,
        };

        Ok(details)
    }
    */
    #[test]
    fn deduce_details_options_tree() {
        let options = Opts {
            tree: 1,
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            details::Options::deduce_tree(&options, &vars),
            Ok(details::Options {
                table: None,
                header: false,
                xattr: xattr::ENABLED && options.extended > 0,
                secattr: xattr::ENABLED && options.security_context > 0,
                mounts: options.mounts > 0,
                color_scale: ColorScaleOptions::deduce(&options, &vars).unwrap(),
            })
        );
    }

    #[test]
    fn deduce_details_options_tree_mounts() {
        let options = Opts {
            tree: 1,
            mounts: 1,
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            details::Options::deduce_tree(&options, &vars),
            Ok(details::Options {
                table: None,
                header: false,
                xattr: false,
                secattr: false,
                mounts: true,
                color_scale: ColorScaleOptions::deduce(&options, &vars).unwrap(),
            })
        );
    }

    #[test]
    fn deduce_details_options_tree_xattr() {
        let options = Opts {
            tree: 1,
            extended: 1,
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            details::Options::deduce_tree(&options, &vars),
            Ok(details::Options {
                table: None,
                header: false,
                xattr: xattr::ENABLED && options.extended > 0,
                secattr: xattr::ENABLED && options.security_context > 0,
                mounts: false,
                color_scale: ColorScaleOptions::deduce(&options, &vars).unwrap(),
            })
        );
    }

    #[test]
    fn deduce_details_options_tree_secattr() {
        let options = Opts {
            tree: 1,
            security_context: 1,
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            details::Options::deduce_tree(&options, &vars),
            Ok(details::Options {
                table: None,
                header: false,
                xattr: xattr::ENABLED && options.extended > 0,
                secattr: xattr::ENABLED && options.security_context > 0,
                mounts: false,
                color_scale: ColorScaleOptions::deduce(&options, &vars).unwrap(),
            })
        );
    }

    #[test]
    fn deduce_details_long_strict_across() {
        let options = Opts {
            long: 1,
            across: 1,
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            details::Options::deduce_long(&options, &vars, true),
            Err(OptionsError::Useless("across", true, "long"))
        );
    }

    #[test]
    fn deduce_details_long_strict_one_line() {
        let options = Opts {
            long: 1,
            oneline: 1,
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(
            details::Options::deduce_long(&options, &vars, true),
            Err(OptionsError::Useless("one-line", true, "long"))
        );
    }

    #[test]
    fn deduce_terminal_width_automatic() {
        let options = Opts { ..Opts::default() };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(TerminalWidth::deduce(&options, &vars), Ok(Automatic));
    }

    #[test]
    fn deduce_terminal_width_set_arg() {
        let options = Opts {
            width: Some(80),
            ..Opts::default()
        };

        let vars = MockVars {
            ..MockVars::default()
        };

        assert_eq!(TerminalWidth::deduce(&options, &vars), Ok(Set(80)));
    }

    #[test]
    fn deduce_terminal_width_set_env() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        vars.set(vars::COLUMNS, &OsString::from("80"));

        let options = Opts { ..Opts::default() };

        assert_eq!(TerminalWidth::deduce(&options, &vars), Ok(Set(80)));
    }

    #[test]
    fn deduce_terminal_width_set_env_bad() {
        let mut vars = MockVars {
            ..MockVars::default()
        };

        vars.set(vars::COLUMNS, &OsString::from("bad"));

        let options = Opts { ..Opts::default() };

        let e: Result<i64, ParseIntError> =
            vars.get(vars::COLUMNS).unwrap().to_string_lossy().parse();

        assert_eq!(
            TerminalWidth::deduce(&options, &vars),
            Err(OptionsError::FailedParse(
                String::from("bad"),
                NumberSource::Env(vars::COLUMNS),
                e.unwrap_err()
            ))
        );
    }
}
