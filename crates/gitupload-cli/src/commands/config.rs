use crate::cli::{ConfigArgs, ConfigSubcommands};
use crate::output::print_result;
use colored::Colorize;
use gitupload_storage::{SettingsRepository, StoragePool};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
}

pub fn execute_config(args: &ConfigArgs, is_json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let pool = StoragePool::init_default().unwrap_or_else(|_| StoragePool::in_memory().unwrap());
    let repo = SettingsRepository::new(&pool);

    match &args.action {
        ConfigSubcommands::List => {
            let conn = pool.conn()?;
            let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
            let rows = stmt.query_map([], |row| {
                Ok(ConfigItem {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })?;
            let mut items = Vec::new();
            for r in rows.flatten() {
                items.push(r);
            }
            print_result(is_json, &items, |_| {
                println!("{}", "GitUpload Configuration Settings:".bold());
                for item in &items {
                    println!("  {:<30} = {}", item.key.cyan(), item.value);
                }
            });
        }
        ConfigSubcommands::Get { key } => {
            let val = repo.get_raw(key)?.unwrap_or_default();
            let item = ConfigItem {
                key: key.clone(),
                value: val.clone(),
            };
            print_result(is_json, &item, |_| {
                println!("{} = {}", key.cyan(), val);
            });
        }
        ConfigSubcommands::Set { key, value } => {
            repo.set_raw(key, value)?;
            let item = ConfigItem {
                key: key.clone(),
                value: value.clone(),
            };
            print_result(is_json, &item, |_| {
                println!("{} Set {} = {}", "✔".green(), key.cyan(), value);
            });
        }
        ConfigSubcommands::SwitchProfile { profile_name } => {
            repo.set_raw("active_profile", profile_name)?;
            let item = serde_json::json!({
                "active_profile": profile_name,
                "status": "switched"
            });
            print_result(is_json, &item, |_| {
                println!(
                    "{} Switched active configuration profile to '{}'",
                    "✔".green(),
                    profile_name.cyan()
                );
            });
        }
    }

    Ok(())
}
