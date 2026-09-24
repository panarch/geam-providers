import filepath
import simplifile

pub fn main() {
  let assert Ok(cwd) = simplifile.current_directory()
  assert verify(filepath.join(cwd, ".geam_simplifile_fixture"))
}

pub fn verify(root: String) -> Bool {
  let assert Ok(Nil) = simplifile.create_directory(root)
  let assert Error(simplifile.Eexist) = simplifile.create_directory(root)

  let source = filepath.join(root, "source.txt")
  let assert Ok(Nil) = simplifile.write(to: source, contents: "hello")
  let assert Ok(Nil) = simplifile.append(to: source, contents: " world")
  let assert Ok("hello world") = simplifile.read(source)
  let assert Ok(info) = simplifile.file_info(source)
  assert info.size == 11
  assert simplifile.file_info_type(info) == simplifile.File
  assert simplifile.exists(source, True) == Ok(True)
  assert simplifile.is_file(source) == Ok(True)
  assert simplifile.is_directory(source) == Ok(False)
  assert simplifile.is_symlink(source) == Ok(False)

  let missing = filepath.join(root, "missing.txt")
  assert simplifile.read(missing) == Error(simplifile.Enoent)
  assert simplifile.exists(missing, True) == Ok(False)
  assert simplifile.write_bits(to: missing, bits: <<5:size(3)>>)
    == Error(simplifile.Einval)
  assert simplifile.exists(missing, False) == Ok(False)

  let invalid_utf8 = filepath.join(root, "invalid.bin")
  let assert Ok(Nil) = simplifile.write_bits(to: invalid_utf8, bits: <<255>>)
  assert simplifile.read(invalid_utf8) == Error(simplifile.NotUtf8)
  let assert Ok(<<255>>) = simplifile.read_bits(invalid_utf8)

  let copied = filepath.join(root, "copied.txt")
  let assert Ok(Nil) = simplifile.copy_file(source, copied)
  let assert Ok("hello world") = simplifile.read(copied)
  let renamed = filepath.join(root, "renamed.txt")
  let assert Ok(Nil) = simplifile.rename(copied, renamed)
  assert simplifile.exists(copied, False) == Ok(False)
  let assert Ok("hello world") = simplifile.read(renamed)

  let hard_link = filepath.join(root, "hard.txt")
  let assert Ok(Nil) = simplifile.create_link(source, hard_link)
  let assert Ok("hello world") = simplifile.read(hard_link)
  let symbolic_link = filepath.join(root, "symbolic.txt")
  let assert Ok(Nil) = simplifile.create_symlink("source.txt", symbolic_link)
  let assert Ok(link) = simplifile.link_info(symbolic_link)
  assert simplifile.file_info_type(link) == simplifile.Symlink
  assert simplifile.is_symlink(symbolic_link) == Ok(True)
  let assert Ok(target) = simplifile.file_info(symbolic_link)
  assert simplifile.file_info_type(target) == simplifile.File
  let assert Ok(Nil) = simplifile.delete(symbolic_link)
  assert simplifile.exists(source, True) == Ok(True)

  let nested = filepath.join(root, "nested")
  let assert Ok(Nil) = simplifile.create_directory_all(nested)
  let nested_file = filepath.join(nested, "data.txt")
  let assert Ok(Nil) = simplifile.write(to: nested_file, contents: "nested")
  let copied_directory = filepath.join(root, "copied_directory")
  let assert Ok(Nil) = simplifile.copy_directory(nested, copied_directory)
  let assert Ok("nested") =
    simplifile.read(filepath.join(copied_directory, "data.txt"))
  let assert Ok(["data.txt"]) = simplifile.read_directory(nested)
  assert simplifile.is_directory(nested) == Ok(True)

  let touched = filepath.join(root, "touched.txt")
  let assert Ok(Nil) = simplifile.touch(touched)
  let assert Ok("") = simplifile.read(touched)
  let assert Ok(Nil) = simplifile.touch(touched)

  let assert Ok(cwd) = simplifile.current_directory()
  let assert Ok(_) = simplifile.resolve(".")
  assert cwd != ""

  let windows =
    filepath.split("C:\\work\\file.txt") == ["c:/", "work", "file.txt"]
  case windows {
    True -> {
      assert simplifile.set_permissions_octal(source, 0o600)
        == Error(simplifile.Enotsup)
    }
    False -> {
      let assert Ok(Nil) = simplifile.set_permissions_octal(source, 0o600)
      let assert Ok(updated) = simplifile.file_info(source)
      assert simplifile.file_info_permissions_octal(updated) == 0o600
    }
  }

  let assert Ok(Nil) = simplifile.delete_file(hard_link)
  assert simplifile.exists(hard_link, False) == Ok(False)
  let assert Ok(Nil) = simplifile.delete(root)
  assert simplifile.exists(root, False) == Ok(False)
  True
}
