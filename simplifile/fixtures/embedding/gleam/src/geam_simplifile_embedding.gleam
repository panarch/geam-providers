import filepath
import simplifile

pub fn verify(root: String) -> Bool {
  let assert Ok(Nil) = simplifile.create_directory(root)
  let file = filepath.join(root, "embedded.txt")
  let assert Ok(Nil) = simplifile.write(to: file, contents: "native")
  let assert Ok(Nil) = simplifile.append(to: file, contents: " provider")
  let assert Ok("native provider") = simplifile.read(file)
  let assert Ok(info) = simplifile.file_info(file)
  assert info.size == 15
  assert simplifile.file_info_type(info) == simplifile.File
  let assert Ok(["embedded.txt"]) = simplifile.read_directory(root)
  let assert Ok(Nil) = simplifile.delete(root)
  assert simplifile.exists(root, False) == Ok(False)
  True
}
