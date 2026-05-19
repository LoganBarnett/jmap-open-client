use jmap_open_client_lib::{LogFormat, LogLevel};
use rust_template_foundation::MergeConfig;

#[derive(Debug, Clone, MergeConfig)]
#[merge_config(app_name = "jmap-open-client")]
pub struct Config {
  #[merge_config(common)]
  pub log_level: LogLevel,
  #[merge_config(common)]
  pub log_format: LogFormat,
  /// Name to greet.
  #[merge_config(short, default = "\"World\".to_string()")]
  pub name: String,
}
