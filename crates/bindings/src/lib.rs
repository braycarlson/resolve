use std::path::PathBuf;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use ::resolve::compiler::Compiler;
use ::resolve::config::Config;
use ::resolve::reporter::{Reporter, Verbosity};

const TEMPLATE_DIRECTORIES_MAX: u32 = 256;
const TEMPLATE_PATH_LENGTH_MAX: u32 = 4096;
const DIRECTORY_PATH_LENGTH_MAX: u32 = 4096;

const DEFAULT_CACHE_DIRECTORY: &str = ".resolve_cache";
const DEFAULT_VENDOR_DIRECTORY: &str = ".resolve_vendor";

fn make_config(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    no_cache: bool,
    no_vendor: bool,
) -> PyResult<Config> {
    assert!(
        !template_directories.is_empty(),
        "template_directories must not be empty",
    );

    let directory_count = u32::try_from(template_directories.len())
        .map_err(|_| PyRuntimeError::new_err("template_directories length exceeds u32"))?;

    assert!(
        directory_count <= TEMPLATE_DIRECTORIES_MAX,
        "template_directories count exceeds maximum",
    );

    assert!(
        !output_directory.is_empty(),
        "output_directory must not be empty",
    );

    let output_length = u32::try_from(output_directory.len())
        .map_err(|_| PyRuntimeError::new_err("output_directory length exceeds u32"))?;

    assert!(
        output_length <= DIRECTORY_PATH_LENGTH_MAX,
        "output_directory length exceeds maximum",
    );

    let template_paths: Vec<PathBuf> = template_directories.iter().map(PathBuf::from).collect();

    let cache_path = PathBuf::from(cache_directory.unwrap_or(DEFAULT_CACHE_DIRECTORY));

    let vendor_path = PathBuf::from(vendor_directory.unwrap_or(DEFAULT_VENDOR_DIRECTORY));

    let venv_path = virtual_environment_path.map(PathBuf::from);

    Config::from_params(
        template_paths,
        PathBuf::from(output_directory),
        cache_path,
        vendor_path,
        venv_path,
    )
    .map(|mut config| {
        if no_cache {
            config.incremental.enabled = false;
        }

        if no_vendor {
            config.vendor.auto_detect = false;
        }

        config
    })
    .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

fn make_reporter(verbose: bool) -> Reporter {
    let verbosity = if verbose {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };

    Reporter::console(verbosity)
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    template_directories,
    output_directory,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
    no_cache = false,
    no_vendor = false,
))]
fn compile_all(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
    no_cache: bool,
    no_vendor: bool,
) -> PyResult<()> {
    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        no_cache,
        no_vendor,
    )?;

    let reporter = make_reporter(verbose);
    let compiler = Compiler::new(&config, &reporter);

    compiler
        .compile_all()
        .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
#[pyo3(signature = (
    template_directories,
    output_directory,
    template,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
    no_cache = false,
    no_vendor = false,
))]
fn compile_single(
    template_directories: Vec<String>,
    output_directory: &str,
    template: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
    no_cache: bool,
    no_vendor: bool,
) -> PyResult<()> {
    assert!(!template.is_empty(), "template must not be empty",);

    let template_length = u32::try_from(template.len())
        .map_err(|_| PyRuntimeError::new_err("template length exceeds u32"))?;

    assert!(
        template_length <= TEMPLATE_PATH_LENGTH_MAX,
        "template length exceeds maximum",
    );

    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        no_cache,
        no_vendor,
    )?;

    let reporter = make_reporter(verbose);
    let compiler = Compiler::new(&config, &reporter);

    compiler
        .compile_single(template)
        .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

#[pyfunction]
#[pyo3(signature = (
    template_directories,
    output_directory,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
))]
fn dry_run(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
) -> PyResult<()> {
    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        false,
        false,
    )?;

    let reporter = make_reporter(verbose);
    let compiler = Compiler::new(&config, &reporter);

    compiler
        .dry_run()
        .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

#[pyfunction]
#[pyo3(signature = (
    template_directories,
    output_directory,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
))]
fn validate(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
) -> PyResult<()> {
    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        false,
        false,
    )?;

    let reporter = make_reporter(verbose);
    let compiler = Compiler::new(&config, &reporter);

    compiler
        .validate()
        .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

#[pyfunction]
#[pyo3(signature = (
    template_directories,
    output_directory,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
))]
fn vendor_sync(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
) -> PyResult<()> {
    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        false,
        false,
    )?;

    let reporter = make_reporter(verbose);

    ::resolve::vendor::sync(&config, &reporter)
        .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

#[pyfunction]
#[pyo3(signature = (
    template_directories,
    output_directory,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
))]
fn vendor_status(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
) -> PyResult<()> {
    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        false,
        false,
    )?;

    let reporter = make_reporter(verbose);

    ::resolve::vendor::status(&config, &reporter)
        .map_err(|error| PyRuntimeError::new_err(format!("{error}")))
}

#[pyfunction]
#[pyo3(signature = (
    template_directories,
    output_directory,
    cache_directory = None,
    vendor_directory = None,
    virtual_environment_path = None,
    verbose = false,
))]
fn clean(
    template_directories: Vec<String>,
    output_directory: &str,
    cache_directory: Option<&str>,
    vendor_directory: Option<&str>,
    virtual_environment_path: Option<&str>,
    verbose: bool,
) -> PyResult<()> {
    let config = make_config(
        template_directories,
        output_directory,
        cache_directory,
        vendor_directory,
        virtual_environment_path,
        false,
        false,
    )?;

    let reporter = make_reporter(verbose);

    let directories: [PathBuf; 3] = [
        config.cache_path().to_path_buf(),
        config.vendor_path().to_path_buf(),
        config.output_path().to_path_buf(),
    ];

    for directory in &directories {
        if directory.exists() {
            std::fs::remove_dir_all(directory)
                .map_err(|error| PyRuntimeError::new_err(format!("{error}")))?;

            reporter.info(&format!("Removed: {}", directory.display()));
        }
    }

    Ok(())
}

#[pymodule]
fn resolve(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(compile_all, module)?)?;
    module.add_function(wrap_pyfunction!(compile_single, module)?)?;
    module.add_function(wrap_pyfunction!(dry_run, module)?)?;
    module.add_function(wrap_pyfunction!(validate, module)?)?;
    module.add_function(wrap_pyfunction!(vendor_sync, module)?)?;
    module.add_function(wrap_pyfunction!(vendor_status, module)?)?;
    module.add_function(wrap_pyfunction!(clean, module)?)?;

    Ok(())
}
