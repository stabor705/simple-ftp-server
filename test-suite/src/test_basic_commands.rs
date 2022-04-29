use std::io::Cursor;
use std::iter::zip;
use std::path::PathBuf;
use std::str::from_utf8;

use crate::TestEnvironment;

use ftp_client::FtpStream;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_connect_and_quit() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.quit().unwrap();
    }

    #[test]
    fn test_nlist() {
        let env = TestEnvironment::new();
        env.create_empty_file("1");
        env.create_empty_file("2");
        env.create_empty_file("3");
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let mut list = ftp.nlst(None).unwrap();
        ftp.quit().unwrap();
        list.sort();
        assert_eq!(list, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_nlins_empy_dir() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let list = ftp.nlst(None).unwrap();
        ftp.quit().unwrap();
        let empty: Vec<String> = Vec::new();
        assert_eq!(list, empty);
    }

    #[test]
    fn test_nlist_in_dir() {
        let env = TestEnvironment::new();
        let dirname = "another dir";
        env.create_dir(dirname);
        env.create_empty_file(format!("{}/1", dirname));
        env.create_empty_file(format!("{}/2", dirname));
        env.create_empty_file(format!("{}/3", dirname));
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let mut list = ftp.nlst(Some(dirname)).unwrap();
        ftp.quit().unwrap();
        list.sort();
        assert_eq!(list, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_simple_file_receiving() {
        let env = TestEnvironment::new();
        let filename = "a very important file with a very long name lol.txt";
        let text = "Hello World!";
        env.create_file(filename, text.as_bytes());
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let cursor = ftp.simple_retr(filename).unwrap();
        assert_eq!(cursor.into_inner().as_slice(), text.as_bytes());
        ftp.quit().unwrap();
    }

    #[test]
    fn test_receiving_nonexistent_file() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        assert!(ftp.simple_retr("This file does not exists").is_err());
        ftp.quit().unwrap();
    }

    #[test]
    fn test_receiving_multiple_files() {
        let env = TestEnvironment::new();
        let contents = vec!["First file", "Second file", "Third File"];
        let filenames = vec!["1", "2", "3"];
        for (filename, content) in zip(filenames.iter(), contents.iter()) {
            env.create_file(filename, content.as_bytes());
        }
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let mut received = Vec::new();
        for filename in filenames {
            received.push(ftp.simple_retr(filename).unwrap().into_inner());
        }
        ftp.quit().unwrap();
        for (received, expected) in zip(received, contents) {
            assert_eq!(std::str::from_utf8(received.as_slice()).unwrap(), expected);
        }
    }

    #[test]
    fn test_simple_file_sending() {
        let env = TestEnvironment::new();
        let filename = "yet another very important file.txt";
        let contents = "random garbage people store in text files";
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.put(filename, &mut Cursor::new(contents)).unwrap();
        ftp.quit().unwrap();
        let data = env.read_file(filename);
        assert_eq!(contents.as_bytes(), data.as_slice());
    }

    #[test]
    fn test_sending_multiple_files() {
        let env = TestEnvironment::new();
        let contents = vec!["First file", "Second file", "Third File"];
        let filenames = vec!["1", "2", "3"];
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        for (filename, content) in zip(filenames.iter(), contents.iter()) {
            ftp.put(filename, &mut Cursor::new(content)).unwrap();
        }
        ftp.quit().unwrap();
        let mut data = Vec::new();
        for filename in filenames {
            data.push(env.read_file(filename));
        }
        for (received, expected) in zip(data.iter(), contents.iter()) {
            assert_eq!(&from_utf8(&received).unwrap(), expected);
        }
    }

    #[test]
    fn test_printing_working_directory() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let working_dir = ftp.pwd().unwrap();
        ftp.quit().unwrap();
        assert_eq!(working_dir, env.dir.path().to_string_lossy().to_string());
    }

    #[test]
    fn test_changing_working_directory() {
        let env = TestEnvironment::new();
        let dirname = "a very important directory";
        env.create_dir(dirname);
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.cwd(dirname).unwrap();
        let path = PathBuf::from(ftp.pwd().unwrap());
        ftp.quit().unwrap();
        assert!(path.ends_with(dirname));
    }

    #[test]
    fn test_changing_to_nonextistent_dir() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        assert!(ftp.cwd("This directory does not exist").is_err());
        ftp.quit().unwrap();
    }

    #[test]
    fn test_creating_directory() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let dirname = "yet another very important directory";
        ftp.mkdir(dirname).unwrap();
        ftp.quit().unwrap();
        assert!(env.dir.path().join(dirname).exists());
    }

    #[test]
    fn test_simple_file_deletion() {
        let env = TestEnvironment::new();
        let filename = "file to delete.json";
        env.create_empty_file(filename);
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.rm(filename).unwrap();
        ftp.quit().unwrap();
        assert!(!env.file_exists(filename))
    }

    #[test]
    fn test_file_renaming() {
        let env = TestEnvironment::new();
        let filename = "file to rename.doc";
        let new_filename = "file renamed.txt";
        env.create_empty_file(filename);
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.rename(filename, new_filename).unwrap();
        ftp.quit().unwrap();
        assert!(!env.file_exists(filename));
        assert!(env.file_exists(new_filename));
    }

    #[test]
    fn test_rename_nonexistent_file() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let result = ftp.rename("This file", "does not exist").is_err();
        ftp.quit().unwrap();
        assert!(result);
    }
}
