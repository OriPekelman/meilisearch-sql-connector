use tracing::{debug, error, info, warn};

pub fn init_logging() {
    tracing_subscriber::fmt::init();
}

pub fn log_error(err: &crate::error::ConnectorError, context: &str) {
    error!(error = ?err, context = context, "Error occurred");
}

pub fn log_warning(warning: &str, context: &str) {
    warn!(warning = warning, context = context, "Warning");
}

pub fn log_info(message: &str, context: &str) {
    info!(message = message, context = context, "Info");
}

pub fn log_debug(message: &str, context: &str) {
    debug!(message = message, context = context, "Debug");
}
