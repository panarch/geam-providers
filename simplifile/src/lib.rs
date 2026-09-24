use geam::provider::{BigInt, BitArrayValue, StringValue};
use std::ffi::OsString;
use std::fs::{self, FileTimes, Metadata, OpenOptions};
use std::io::{self, Write};
use std::time::SystemTime;

#[geam::provider(package = "simplifile", modules = [simplifile])]
pub struct Component;

#[geam::module(path = "simplifile")]
mod simplifile {
    use super::{BigInt, BitArrayValue, FileTimes, Metadata, OpenOptions, OsString, StringValue};
    use super::{SystemTime, Write, fs, io};
    #[cfg(windows)]
    use std::path::Path;

    #[allow(dead_code)]
    #[derive(Debug, PartialEq, Eq)]
    #[geam::custom]
    enum FileError {
        Eacces,
        Eagain,
        Ebadf,
        Ebadmsg,
        Ebusy,
        Edeadlk,
        Edeadlock,
        Edquot,
        Eexist,
        Efault,
        Efbig,
        Eftype,
        Eintr,
        Einval,
        Eio,
        Eisdir,
        Eloop,
        Emfile,
        Emlink,
        Emultihop,
        Enametoolong,
        Enfile,
        Enobufs,
        Enodev,
        Enolck,
        Enolink,
        Enoent,
        Enomem,
        Enospc,
        Enosr,
        Enostr,
        Enosys,
        Enotblk,
        Enotdir,
        Enotsup,
        Enxio,
        Eopnotsupp,
        Eoverflow,
        Eperm,
        Epipe,
        Erange,
        Erofs,
        Espipe,
        Esrch,
        Estale,
        Etxtbsy,
        Exdev,
        NotUtf8,
        Unknown { inner: StringValue },
    }

    #[allow(dead_code)]
    #[derive(Debug, PartialEq, Eq)]
    #[geam::custom]
    enum FileInfo {
        FileInfo {
            size: BigInt,
            mode: BigInt,
            nlinks: BigInt,
            inode: BigInt,
            user_id: BigInt,
            group_id: BigInt,
            dev: BigInt,
            atime_seconds: BigInt,
            mtime_seconds: BigInt,
            ctime_seconds: BigInt,
        },
    }

    #[geam::function]
    fn file_info(filepath: StringValue) -> Result<FileInfo, FileError> {
        fs::metadata(filepath.as_str())
            .map(metadata_to_info)
            .map_err(file_error)
    }

    #[geam::function]
    fn link_info(filepath: StringValue) -> Result<FileInfo, FileError> {
        fs::symlink_metadata(filepath.as_str())
            .map(metadata_to_info)
            .map_err(file_error)
    }

    #[geam::function]
    fn delete(path: StringValue) -> Result<(), FileError> {
        let metadata = fs::symlink_metadata(path.as_str()).map_err(file_error)?;
        if metadata.is_dir() {
            fs::remove_dir_all(path.as_str()).map_err(file_error)
        } else {
            remove_file_or_directory_link(path.as_str(), &metadata).map_err(file_error)
        }
    }

    #[geam::function]
    fn delete_file(path: StringValue) -> Result<(), FileError> {
        fs::remove_file(path.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn read_bits(filepath: StringValue) -> Result<BitArrayValue, FileError> {
        fs::read(filepath.as_str())
            .map(BitArrayValue::from_bytes)
            .map_err(file_error)
    }

    #[geam::function]
    fn write_bits(filepath: StringValue, bits: BitArrayValue) -> Result<(), FileError> {
        if !bits.bit_len().is_multiple_of(8) {
            return Err(FileError::Einval);
        }
        fs::write(filepath.as_str(), bits.bytes()).map_err(file_error)
    }

    #[geam::function]
    fn append_bits(filepath: StringValue, bits: BitArrayValue) -> Result<(), FileError> {
        if !bits.bit_len().is_multiple_of(8) {
            return Err(FileError::Einval);
        }
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(filepath.as_str())
            .map_err(file_error)?;
        file.write_all(bits.bytes()).map_err(file_error)
    }

    #[geam::function]
    fn create_directory(filepath: StringValue) -> Result<(), FileError> {
        fs::create_dir(filepath.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn create_symlink(target: StringValue, symlink: StringValue) -> Result<(), FileError> {
        create_host_symlink(target.as_str(), symlink.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn create_link(target: StringValue, link: StringValue) -> Result<(), FileError> {
        fs::hard_link(target.as_str(), link.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn read_directory(path: StringValue) -> Result<Vec<StringValue>, FileError> {
        fs::read_dir(path.as_str())
            .map_err(file_error)?
            .map(|entry| directory_entry_name(entry.map(|entry| entry.file_name())))
            .collect()
    }

    fn directory_entry_name(name: io::Result<OsString>) -> Result<StringValue, FileError> {
        unicode_file_name(name.map_err(file_error)?)
    }

    #[geam::function]
    fn do_create_dir_all(dirpath: StringValue) -> Result<(), FileError> {
        fs::create_dir_all(dirpath.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn do_copy_file(src: StringValue, dest: StringValue) -> Result<BigInt, FileError> {
        fs::copy(src.as_str(), dest.as_str())
            .map(BigInt::from)
            .map_err(file_error)
    }

    #[geam::function]
    fn rename_file(src: StringValue, dest: StringValue) -> Result<(), FileError> {
        fs::rename(src.as_str(), dest.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn rename(src: StringValue, dest: StringValue) -> Result<(), FileError> {
        fs::rename(src.as_str(), dest.as_str()).map_err(file_error)
    }

    #[geam::function]
    fn set_permissions_octal(filepath: StringValue, permissions: BigInt) -> Result<(), FileError> {
        let mode = u32::try_from(permissions).map_err(|_| FileError::Einval)?;
        if mode > 0o7777 {
            return Err(FileError::Einval);
        }
        set_host_permissions(filepath.as_str(), mode).map_err(file_error)
    }

    #[geam::function]
    fn erl_do_current_directory() -> Result<Vec<char>, FileError> {
        unicode_current_directory(std::env::current_dir())
    }

    fn unicode_current_directory(
        path: io::Result<std::path::PathBuf>,
    ) -> Result<Vec<char>, FileError> {
        let path = path.map_err(file_error)?;
        let text = path.to_str().ok_or(FileError::Einval)?;
        Ok(text.chars().collect())
    }

    #[geam::function]
    fn do_resolve(path: StringValue) -> StringValue {
        let absolute = std::path::absolute(path.as_str());
        resolve_absolute(path, absolute)
    }

    fn resolve_absolute(
        path: StringValue,
        absolute: io::Result<std::path::PathBuf>,
    ) -> StringValue {
        match absolute {
            Ok(absolute) => absolute.to_string_lossy().into_owned().into(),
            Err(_) => path,
        }
    }

    #[geam::function]
    fn touch(path: StringValue) -> Result<(), FileError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.as_str())
            .map_err(file_error)?;
        let now = SystemTime::now();
        file.set_times(FileTimes::new().set_accessed(now).set_modified(now))
            .map_err(file_error)
    }

    fn file_error(error: io::Error) -> FileError {
        match error.kind() {
            io::ErrorKind::NotFound => FileError::Enoent,
            io::ErrorKind::PermissionDenied => FileError::Eacces,
            io::ErrorKind::AlreadyExists => FileError::Eexist,
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => FileError::Einval,
            io::ErrorKind::IsADirectory => FileError::Eisdir,
            io::ErrorKind::NotADirectory => FileError::Enotdir,
            io::ErrorKind::Unsupported => FileError::Enotsup,
            _ => FileError::Unknown {
                inner: error.to_string().into(),
            },
        }
    }

    fn unicode_file_name(name: OsString) -> Result<StringValue, FileError> {
        name.into_string()
            .map(StringValue::from)
            .map_err(|_| FileError::Einval)
    }

    #[cfg(unix)]
    fn metadata_to_info(metadata: Metadata) -> FileInfo {
        use std::os::unix::fs::MetadataExt;

        FileInfo::FileInfo {
            size: metadata.size().into(),
            mode: metadata.mode().into(),
            nlinks: metadata.nlink().into(),
            inode: metadata.ino().into(),
            user_id: metadata.uid().into(),
            group_id: metadata.gid().into(),
            dev: metadata.dev().into(),
            atime_seconds: metadata.atime().into(),
            mtime_seconds: metadata.mtime().into(),
            ctime_seconds: metadata.ctime().into(),
        }
    }

    #[cfg(windows)]
    fn metadata_to_info(metadata: Metadata) -> FileInfo {
        use std::os::windows::fs::MetadataExt;

        let file_type = metadata.file_type();
        let type_bits = if file_type.is_symlink() {
            0o120000
        } else if file_type.is_dir() {
            0o040000
        } else {
            // Windows FileType classifies every other entry as a regular file.
            0o100000
        };
        let write_bits = if metadata.permissions().readonly() {
            0
        } else {
            0o222
        };
        let execute_bits = if file_type.is_dir() { 0o111 } else { 0 };

        FileInfo::FileInfo {
            size: metadata.file_size().into(),
            mode: BigInt::from(type_bits | 0o444 | write_bits | execute_bits),
            // The stable Windows MetadataExt API does not expose these fields.
            nlinks: 0.into(),
            inode: 0.into(),
            user_id: 0.into(),
            group_id: 0.into(),
            dev: 0.into(),
            atime_seconds: windows_filetime_seconds(metadata.last_access_time()),
            mtime_seconds: windows_filetime_seconds(metadata.last_write_time()),
            ctime_seconds: 0.into(),
        }
    }

    #[cfg(windows)]
    fn windows_filetime_seconds(ticks: u64) -> BigInt {
        if ticks == 0 {
            return 0.into();
        }
        BigInt::from(ticks / 10_000_000) - BigInt::from(11_644_473_600_u64)
    }

    #[cfg(unix)]
    fn create_host_symlink(target: &str, link: &str) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_host_symlink(target: &str, link: &str) -> io::Result<()> {
        let target_path = Path::new(target);
        let resolved_target = Path::new(link)
            .parent()
            .map(|parent| parent.join(target_path));
        if resolved_target.is_some_and(|path| path.is_dir()) {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }

    #[cfg(unix)]
    fn remove_file_or_directory_link(path: &str, _metadata: &Metadata) -> io::Result<()> {
        fs::remove_file(path)
    }

    #[cfg(windows)]
    fn remove_file_or_directory_link(path: &str, metadata: &Metadata) -> io::Result<()> {
        use std::os::windows::fs::FileTypeExt;

        if metadata.file_type().is_symlink_dir() {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
    }

    #[cfg(unix)]
    fn set_host_permissions(path: &str, mode: u32) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(mode))
    }

    #[cfg(windows)]
    fn set_host_permissions(_path: &str, _mode: u32) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    #[cfg(test)]
    mod tests {
        use super::{
            BigInt, BitArrayValue, FileError, FileInfo, FileTimes, StringValue, SystemTime,
            append_bits, create_directory, create_link, create_symlink, delete, delete_file,
            do_copy_file, do_create_dir_all, do_resolve, erl_do_current_directory, file_error,
            file_info, link_info, read_bits, read_directory, rename, rename_file,
            set_permissions_octal, touch, write_bits,
        };
        use std::fs;
        use std::io;
        use std::path::Path;
        use std::time::Duration;

        fn source_path(path: &Path) -> StringValue {
            path.to_str().expect("fixture path is Unicode").into()
        }

        #[test]
        fn byte_operations_preserve_contents_and_reject_partial_bytes() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            let file = source_path(&temp.path().join("record.bin"));
            assert_eq!(read_bits(file.clone()), Err(FileError::Enoent));

            write_bits(file.clone(), BitArrayValue::from_bytes(vec![0, 255]))
                .expect("write byte-aligned contents");
            append_bits(file.clone(), BitArrayValue::from_bytes(vec![7]))
                .expect("append byte-aligned contents");
            assert_eq!(
                read_bits(file.clone()),
                Ok(BitArrayValue::from_bytes(vec![0, 255, 7]))
            );

            let partial = BitArrayValue::try_from_parts(vec![0b1010_0000], 4)
                .expect("four bits fit in one byte");
            assert_eq!(
                write_bits(file.clone(), partial.clone()),
                Err(FileError::Einval)
            );
            assert_eq!(append_bits(file.clone(), partial), Err(FileError::Einval));
            let missing_parent = source_path(&temp.path().join("absent/new.bin"));
            assert_eq!(
                append_bits(missing_parent, BitArrayValue::from_bytes(vec![1])),
                Err(FileError::Enoent)
            );
            assert_eq!(
                read_bits(file.clone()),
                Ok(BitArrayValue::from_bytes(vec![0, 255, 7]))
            );

            write_bits(file.clone(), BitArrayValue::from_bytes(Vec::new()))
                .expect("empty write truncates");
            assert_eq!(read_bits(file), Ok(BitArrayValue::from_bytes(Vec::new())));
        }

        #[test]
        fn directory_copy_rename_and_delete_observe_real_effects() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            let nested = source_path(&temp.path().join("one/two"));
            do_create_dir_all(nested.clone()).expect("create parents");
            do_create_dir_all(nested.clone()).expect("existing directory is allowed");
            let another = source_path(&temp.path().join("one/three"));
            create_directory(another.clone()).expect("create one directory");
            assert_eq!(create_directory(another.clone()), Err(FileError::Eexist));

            let original = source_path(&temp.path().join("one/two/first.txt"));
            write_bits(
                original.clone(),
                BitArrayValue::from_bytes(b"hello".to_vec()),
            )
            .expect("write original");
            let copied = source_path(&temp.path().join("one/three/copy.txt"));
            assert_eq!(
                do_copy_file(original.clone(), copied.clone()),
                Ok(BigInt::from(5))
            );
            let renamed = source_path(&temp.path().join("one/three/renamed.txt"));
            rename_file(copied.clone(), renamed.clone()).expect("deprecated rename");
            assert_eq!(read_bits(copied), Err(FileError::Enoent));
            let final_path = source_path(&temp.path().join("one/three/final.txt"));
            rename(renamed, final_path.clone()).expect("rename");
            assert_eq!(
                read_bits(final_path.clone()),
                Ok(BitArrayValue::from_bytes(b"hello".to_vec()))
            );

            let mut entries = read_directory(another).expect("directory contents");
            entries.sort();
            assert_eq!(entries, vec![StringValue::from("final.txt")]);
            assert_eq!(
                read_directory(source_path(&temp.path().join("missing"))),
                Err(FileError::Enoent)
            );
            assert_eq!(
                super::directory_entry_name(Err(io::Error::other("directory entry vanished"))),
                Err(FileError::Unknown {
                    inner: "directory entry vanished".into()
                })
            );
            delete_file(final_path.clone()).expect("delete only one file");
            assert_eq!(read_bits(final_path), Err(FileError::Enoent));
            delete(source_path(&temp.path().join("one"))).expect("recursive directory removal");
            assert!(!temp.path().join("one").exists());
            assert_eq!(delete(nested), Err(FileError::Enoent));
        }

        #[cfg(unix)]
        #[test]
        fn links_and_metadata_distinguish_target_from_link() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            let file = source_path(&temp.path().join("target.txt"));
            write_bits(file.clone(), BitArrayValue::from_bytes(b"abc".to_vec()))
                .expect("write link target");
            let hard_link = source_path(&temp.path().join("hard.txt"));
            create_link(file.clone(), hard_link.clone()).expect("hard link");
            let symbolic_link = source_path(&temp.path().join("symbolic.txt"));
            create_symlink("target.txt".into(), symbolic_link.clone()).expect("symbolic link");

            let FileInfo::FileInfo {
                size, mode, nlinks, ..
            } = file_info(symbolic_link.clone()).expect("follows symbolic link");
            assert_eq!(size, BigInt::from(3));
            assert_eq!(mode & BigInt::from(0o170000), BigInt::from(0o100000));
            assert_eq!(nlinks, BigInt::from(2));
            let FileInfo::FileInfo { mode, .. } =
                link_info(symbolic_link.clone()).expect("reads symbolic link itself");
            assert_eq!(mode & BigInt::from(0o170000), BigInt::from(0o120000));

            delete(symbolic_link).expect("delete link without touching target");
            assert_eq!(
                read_bits(file.clone()),
                Ok(BitArrayValue::from_bytes(b"abc".to_vec()))
            );
            delete(hard_link).expect("delete hard link");
            delete(file).expect("delete plain file");

            let directory = source_path(&temp.path().join("target-directory"));
            create_directory(directory.clone()).expect("create directory target");
            let directory_link = source_path(&temp.path().join("directory-link"));
            create_symlink("target-directory".into(), directory_link.clone())
                .expect("symbolic link to directory");
            delete(directory_link).expect("delete directory link only");
            assert!(temp.path().join("target-directory").is_dir());
        }

        #[cfg(windows)]
        #[test]
        fn windows_links_select_file_or_directory_and_delete_only_the_link() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            let file = source_path(&temp.path().join("target.txt"));
            write_bits(file.clone(), BitArrayValue::from_bytes(b"target".to_vec()))
                .expect("create file target");
            let hard_link = source_path(&temp.path().join("hard-link"));
            create_link(file.clone(), hard_link.clone()).expect("hard link to file");
            let FileInfo::FileInfo { size, .. } =
                file_info(hard_link.clone()).expect("hard link metadata");
            assert_eq!(size, BigInt::from(6));
            delete_file(hard_link).expect("delete hard link only");
            assert!(temp.path().join("target.txt").is_file());

            let file_link = source_path(&temp.path().join("file-link"));
            create_symlink("target.txt".into(), file_link.clone()).expect("symbolic link to file");
            let FileInfo::FileInfo { mode, .. } =
                link_info(file_link.clone()).expect("file link metadata");
            assert_eq!(mode & BigInt::from(0o170000), BigInt::from(0o120000));
            delete(file_link).expect("delete file link only");
            assert!(temp.path().join("target.txt").is_file());

            let directory = source_path(&temp.path().join("target-directory"));
            create_directory(directory.clone()).expect("create directory target");
            let FileInfo::FileInfo { mode, .. } = file_info(directory).expect("directory metadata");
            assert_eq!(mode & BigInt::from(0o170111), BigInt::from(0o040111));
            let directory_link = source_path(&temp.path().join("directory-link"));
            create_symlink("target-directory".into(), directory_link.clone())
                .expect("symbolic link to directory");
            let FileInfo::FileInfo { mode, .. } =
                link_info(directory_link.clone()).expect("directory link metadata");
            assert_eq!(mode & BigInt::from(0o170000), BigInt::from(0o120000));
            delete(directory_link).expect("delete directory link only");
            assert!(temp.path().join("target-directory").is_dir());
        }

        #[test]
        fn current_directory_and_resolution_use_the_host_path() {
            let cwd = std::env::current_dir().expect("host working directory");
            assert_eq!(
                erl_do_current_directory(),
                Ok(cwd
                    .to_str()
                    .expect("Unicode working directory")
                    .chars()
                    .collect())
            );
            assert_eq!(
                do_resolve("src/../Cargo.toml".into()),
                StringValue::from(
                    std::path::absolute("src/../Cargo.toml")
                        .expect("lexical absolute path")
                        .to_string_lossy()
                        .into_owned()
                )
            );
            let original: StringValue = "relative.txt".into();
            assert_eq!(
                super::resolve_absolute(
                    original.clone(),
                    Err(io::Error::from(io::ErrorKind::NotFound))
                ),
                original
            );
            assert_eq!(
                super::unicode_current_directory(Err(io::Error::from(io::ErrorKind::NotFound))),
                Err(FileError::Enoent)
            );
        }

        #[test]
        fn touch_updates_an_existing_file_and_creates_a_missing_file() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            assert_eq!(
                touch(source_path(&temp.path().join("absent/touched.txt"))),
                Err(FileError::Enoent)
            );
            let file_path = temp.path().join("touched.txt");
            let file = source_path(&file_path);
            touch(file.clone()).expect("create missing file");
            assert_eq!(
                fs::read(&file_path).expect("empty new file"),
                Vec::<u8>::new()
            );
            fs::write(&file_path, b"keep existing contents").expect("populate touched file");
            let old_time = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
            fs::OpenOptions::new()
                .write(true)
                .open(&file_path)
                .expect("created file")
                .set_times(
                    FileTimes::new()
                        .set_accessed(old_time)
                        .set_modified(old_time),
                )
                .expect("set old timestamps");
            touch(file).expect("update existing file");
            assert!(
                fs::metadata(&file_path)
                    .expect("touched metadata")
                    .modified()
                    .expect("modification time")
                    > old_time
            );
            assert_eq!(
                fs::read(&file_path).expect("existing contents survive touch"),
                b"keep existing contents"
            );
        }

        #[test]
        fn error_mapping_preserves_named_boundaries_and_unknown_reason() {
            let known = [
                (io::ErrorKind::NotFound, FileError::Enoent),
                (io::ErrorKind::PermissionDenied, FileError::Eacces),
                (io::ErrorKind::AlreadyExists, FileError::Eexist),
                (io::ErrorKind::InvalidInput, FileError::Einval),
                (io::ErrorKind::InvalidData, FileError::Einval),
                (io::ErrorKind::IsADirectory, FileError::Eisdir),
                (io::ErrorKind::NotADirectory, FileError::Enotdir),
                (io::ErrorKind::Unsupported, FileError::Enotsup),
            ];
            for (kind, expected) in known {
                assert_eq!(file_error(io::Error::from(kind)), expected);
            }
            assert_eq!(
                file_error(io::Error::other("unclassified filesystem error")),
                FileError::Unknown {
                    inner: "unclassified filesystem error".into()
                }
            );
        }

        #[test]
        fn invalid_permission_values_are_rejected_before_the_host_call() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            let file = source_path(&temp.path().join("permissions.txt"));
            write_bits(file.clone(), BitArrayValue::from_bytes(Vec::new())).expect("create file");
            assert_eq!(
                set_permissions_octal(file.clone(), BigInt::from(-1)),
                Err(FileError::Einval)
            );
            assert_eq!(
                set_permissions_octal(file.clone(), BigInt::from(0o10000)),
                Err(FileError::Einval)
            );
            #[cfg(unix)]
            {
                set_permissions_octal(file.clone(), BigInt::from(0o600))
                    .expect("set Unix permissions");
                let FileInfo::FileInfo { mode, .. } =
                    file_info(file).expect("read Unix permissions");
                assert_eq!(mode & BigInt::from(0o777), BigInt::from(0o600));
            }
            #[cfg(windows)]
            assert_eq!(
                set_permissions_octal(file, BigInt::from(0o600)),
                Err(FileError::Enotsup)
            );
        }

        #[cfg(unix)]
        #[test]
        fn non_unicode_directory_names_report_invalid_input() {
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;

            let bad_name = OsString::from_vec(vec![0xff]);
            assert_eq!(
                super::unicode_file_name(bad_name.clone()),
                Err(FileError::Einval)
            );
            assert_eq!(
                super::unicode_current_directory(Ok(bad_name.into())),
                Err(FileError::Einval)
            );
        }

        #[cfg(windows)]
        #[test]
        fn non_unicode_directory_names_report_invalid_input() {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;

            let bad_name = OsString::from_wide(&[0xd800]);
            assert_eq!(
                super::unicode_file_name(bad_name.clone()),
                Err(FileError::Einval)
            );
            assert_eq!(
                super::unicode_current_directory(Ok(bad_name.into())),
                Err(FileError::Einval)
            );
        }

        #[cfg(windows)]
        #[test]
        fn windows_metadata_uses_documented_missing_field_sentinels() {
            let temp = tempfile::tempdir().expect("isolated file tree");
            let file = source_path(&temp.path().join("record.txt"));
            write_bits(file.clone(), BitArrayValue::from_bytes(b"ok".to_vec()))
                .expect("write file");
            let FileInfo::FileInfo {
                size,
                mode,
                nlinks,
                inode,
                user_id,
                group_id,
                dev,
                atime_seconds,
                mtime_seconds,
                ctime_seconds,
            } = file_info(file).expect("Windows file info");
            assert_eq!(size, BigInt::from(2));
            assert_eq!(mode & BigInt::from(0o170000), BigInt::from(0o100000));
            assert_eq!(nlinks, BigInt::from(0));
            assert_eq!(inode, BigInt::from(0));
            assert_eq!(user_id, BigInt::from(0));
            assert_eq!(group_id, BigInt::from(0));
            assert_eq!(dev, BigInt::from(0));
            assert!(atime_seconds > BigInt::from(0));
            assert!(mtime_seconds > BigInt::from(0));
            assert_eq!(ctime_seconds, BigInt::from(0));
            assert_eq!(super::windows_filetime_seconds(0), BigInt::from(0));

            let file_path = temp.path().join("record.txt");
            let mut permissions = fs::metadata(&file_path)
                .expect("writable file metadata")
                .permissions();
            permissions.set_readonly(true);
            fs::set_permissions(&file_path, permissions.clone()).expect("mark file readonly");
            let FileInfo::FileInfo { mode, .. } =
                file_info(source_path(&file_path)).expect("readonly Windows metadata");
            assert_eq!(mode & BigInt::from(0o222), BigInt::from(0));
            permissions.set_readonly(false);
            fs::set_permissions(&file_path, permissions).expect("restore writable file");
        }
    }
}
