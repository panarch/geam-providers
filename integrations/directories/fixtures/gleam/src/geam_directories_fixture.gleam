import directories
import envoy
import filepath
import gleam/list
import platform
import simplifile

pub fn main() {
  let assert Ok(cwd) = simplifile.current_directory()
  assert verify(filepath.join(cwd, ".geam_directories_fixture"))
}

pub fn seed_is(value: String) -> Bool {
  envoy.get("GEAM_DIRECTORIES_SEED") == Ok(value)
}

pub fn verify(root: String) -> Bool {
  let home = filepath.join(root, "home")
  let temp_first = filepath.join(root, "temp_first")
  let temp_second = filepath.join(root, "temp_second")
  let temp_third = filepath.join(root, "temp_third")
  let app_data = filepath.join(root, "app_data")
  let local_app_data = filepath.join(root, "local_app_data")
  let xdg_cache = filepath.join(root, "xdg_cache")
  let xdg_config = filepath.join(root, "xdg_config")
  let xdg_data = filepath.join(root, "xdg_data")
  let xdg_bin = filepath.join(root, "xdg_bin")
  let xdg_runtime = filepath.join(root, "xdg_runtime")
  let xdg_state = filepath.join(root, "xdg_state")
  let home_cache = filepath.join(home, ".cache")
  let mac_cache = filepath.join(home, "Library/Caches")
  let mac_support = filepath.join(home, "Library/Application Support")
  let mac_preferences = filepath.join(home, "Library/Preferences")

  list.each(
    [
      home,
      temp_first,
      temp_second,
      temp_third,
      app_data,
      local_app_data,
      xdg_cache,
      xdg_config,
      xdg_data,
      xdg_bin,
      xdg_runtime,
      xdg_state,
      home_cache,
      mac_cache,
      mac_support,
      mac_preferences,
    ],
    fn(path) {
      let assert Ok(Nil) = simplifile.create_directory_all(path)
    },
  )

  envoy.set("HOME", home)
  envoy.set("UserProfile", home)
  envoy.set("Profile", temp_second)
  envoy.set("TMPDIR", temp_first)
  envoy.set("TEMP", temp_second)
  envoy.set("TMP", temp_third)
  envoy.set("APPDATA", app_data)
  envoy.set("LOCALAPPDATA", local_app_data)
  envoy.set("XDG_CACHE_HOME", xdg_cache)
  envoy.set("XDG_CONFIG_HOME", xdg_config)
  envoy.set("XDG_DATA_HOME", xdg_data)
  envoy.set("XDG_BIN_HOME", xdg_bin)
  envoy.set("XDG_RUNTIME_DIR", xdg_runtime)
  envoy.set("XDG_STATE_HOME", xdg_state)

  assert simplifile.is_directory(temp_first) == Ok(True)
  assert directories.tmp_dir() == Ok(temp_first)
  assert directories.tmp_dir() != Ok(temp_second)

  case platform.os() {
    platform.Win32 -> {
      assert directories.home_dir() == Ok(home)
      assert directories.cache_dir() == Ok(app_data)
      assert directories.config_dir() == Ok(app_data)
      assert directories.config_local_dir() == Ok(local_app_data)
      assert directories.data_dir() == Ok(app_data)
      assert directories.data_local_dir() == Ok(local_app_data)
      assert directories.executable_dir() == Error(Nil)
      assert directories.preference_dir() == Ok(app_data)
      assert directories.runtime_dir() == Error(Nil)
      assert directories.state_dir() == Error(Nil)

      envoy.set("UserProfile", filepath.join(root, "missing"))
      assert directories.home_dir() == Ok(temp_second)
      envoy.set("APPDATA", "")
      assert directories.config_dir() == Error(Nil)
    }
    platform.Darwin -> {
      assert directories.home_dir() == Ok(home)
      assert directories.cache_dir() == Ok(mac_cache)
      assert directories.config_dir() == Ok(mac_support)
      assert directories.config_local_dir() == Ok(mac_support)
      assert directories.data_dir() == Ok(mac_support)
      assert directories.data_local_dir() == Ok(mac_support)
      assert directories.executable_dir() == Error(Nil)
      assert directories.preference_dir() == Ok(mac_preferences)
      assert directories.runtime_dir() == Error(Nil)
      assert directories.state_dir() == Error(Nil)

      envoy.unset("HOME")
      assert directories.home_dir() == Error(Nil)
      envoy.set("HOME", home)
    }
    platform.Linux -> {
      assert directories.home_dir() == Ok(home)
      assert directories.cache_dir() == Ok(xdg_cache)
      assert directories.config_dir() == Ok(xdg_config)
      assert directories.config_local_dir() == Ok(xdg_config)
      assert directories.data_dir() == Ok(xdg_data)
      assert directories.data_local_dir() == Ok(xdg_data)
      assert directories.executable_dir() == Ok(xdg_bin)
      assert directories.preference_dir() == Ok(xdg_config)
      assert directories.runtime_dir() == Ok(xdg_runtime)
      assert directories.state_dir() == Ok(xdg_state)

      envoy.set("XDG_CACHE_HOME", filepath.join(root, "missing"))
      assert directories.cache_dir() == Ok(home_cache)
      envoy.set("XDG_CACHE_HOME", "")
      assert directories.cache_dir() == Ok(home_cache)
    }
    _ -> panic as "This fixture requires Linux, macOS, or Windows"
  }

  envoy.set("TMPDIR", filepath.join(root, "missing"))
  assert directories.tmp_dir() == Ok(temp_second)
  envoy.set("TEMP", "")
  assert directories.tmp_dir() == Ok(temp_third)

  let assert Ok(Nil) = simplifile.delete(root)
  True
}
