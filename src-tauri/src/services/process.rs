use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn configure_background_command(command: &mut Command) -> &mut Command {
  #[cfg(target_os = "windows")]
  {
    use std::os::windows::process::CommandExt;

    command.creation_flags(CREATE_NO_WINDOW);
  }

  command
}
