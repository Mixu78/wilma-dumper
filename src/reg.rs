use winreg::{enums::*, RegKey};

use anyhow::{anyhow, Result};

pub fn register_wilma_handler() -> Result<()> {
    let path = std::env::current_exe()?.into_os_string();
    let path = path.to_str().ok_or_else(|| anyhow!("Path contains non-unicode characters"))?.replace(r"\\?\", "");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey("SOFTWARE\\Classes")?;

    let wilma = classes.create_subkey("wilma")?.0;
    wilma.set_value("", &"URL:wilma")?;
    wilma.set_value("URL Protocol", &"")?;

    let command = wilma.create_subkey("shell\\open\\command")?.0;
    command.set_value("", &format!(r#""{}" "__OAUTH" "%1""#, path))?;

    Ok(())
}

pub fn unregister_wilma_handler() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.delete_subkey_all("SOFTWARE\\Classes\\wilma")?;

    Ok(())
}
