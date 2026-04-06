#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

const _: () = assert!(
    (Verbosity::Quiet as u8) < (Verbosity::Normal as u8)
        && (Verbosity::Normal as u8) < (Verbosity::Verbose as u8),
);

#[derive(Debug, Clone, Copy)]
pub enum Reporter {
    Console(Verbosity),
    Silent,
}

impl Reporter {
    pub fn console(verbosity: Verbosity) -> Self {
        assert!(
            matches!(
                verbosity,
                Verbosity::Quiet | Verbosity::Normal | Verbosity::Verbose
            ),
            "verbosity must be a valid variant",
        );

        Self::Console(verbosity)
    }

    pub fn silent() -> Self {
        Self::Silent
    }

    pub fn info(&self, message: &str) {
        assert!(
            matches!(self, Self::Console(_) | Self::Silent),
            "reporter must be a valid variant",
        );

        match self {
            Self::Console(verbosity) => {
                if *verbosity >= Verbosity::Normal {
                    println!("{}", message);
                }
            }
            Self::Silent => {}
        }
    }

    pub fn warn(&self, message: &str) {
        assert!(
            matches!(self, Self::Console(_) | Self::Silent),
            "reporter must be a valid variant",
        );

        match self {
            Self::Console(verbosity) => {
                if *verbosity >= Verbosity::Normal {
                    println!("  WARN: {}", message);
                }
            }
            Self::Silent => {}
        }
    }

    pub fn error(&self, message: &str) {
        assert!(
            matches!(self, Self::Console(_) | Self::Silent),
            "reporter must be a valid variant",
        );

        match self {
            Self::Console(_) => {
                eprintln!("{}", message);
            }
            Self::Silent => {}
        }
    }

    pub fn debug(&self, message: &str) {
        assert!(
            matches!(self, Self::Console(_) | Self::Silent),
            "reporter must be a valid variant",
        );

        match self {
            Self::Console(verbosity) => {
                if *verbosity >= Verbosity::Verbose {
                    println!("  DEBUG: {}", message);
                }
            }
            Self::Silent => {}
        }
    }

    pub fn verbosity(&self) -> Verbosity {
        assert!(
            matches!(self, Self::Console(_) | Self::Silent),
            "reporter must be a valid variant",
        );

        match self {
            Self::Console(verbosity) => *verbosity,
            Self::Silent => Verbosity::Quiet,
        }
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::console(Verbosity::Normal)
    }
}
