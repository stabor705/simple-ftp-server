use std::fs::File;
use std::io::Write;

use crate::TestEnvironment;

use ftp_client::FtpStream;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

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
    fn test_simple_file_sending() {
        let env = TestEnvironment::new();
        let filename = "yet another very important file.txt";
        let filepath = std::env::temp_dir().join(filename);
        let mut file = File::create(filepath.clone()).unwrap();
        let contents = "random garbage people store in text files";
        file.write_all(contents.as_bytes()).unwrap();
        let mut file = File::open(filepath.clone()).unwrap();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.put(filename, &mut file).unwrap();
        ftp.quit().unwrap();
        let mut file = File::open(env.dir.path().join(filename)).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        assert_eq!(contents, data);
        std::fs::remove_file(filepath).unwrap();
    }

    #[test]
    fn test_printing_working_directory() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let working_dir = ftp.pwd().unwrap();
        ftp.quit().unwrap();
        assert_eq!(working_dir, env.dir.path().to_string_lossy().to_string());
    }
}
