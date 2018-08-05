use std::{
    error::Error,
    fs::{
        File,
        read_dir,
        create_dir
    },
    io::Write,
    path::Path,
    env::{
        var,
        current_dir
    }
};

fn main() -> Result<(), Box<Error>> {
    // get path to addon lib dir, creating if nonexistant
    let addon_dir_path = current_dir()?.as_path().join("plugins");

    if let Err(e) = read_dir(addon_dir_path.clone()) {
        create_dir(addon_dir_path.clone())?;
    }

    let mut f = File::create(Path::new(&var("OUT_DIR")?).join("plugin_dir"))?;
    write!(f, "{:#?}", addon_dir_path)?;

    Ok(())
}