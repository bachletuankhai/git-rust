use std::{
    ffi::CStr,
    fs::{self, File},
    io::{stdout, BufRead, BufReader, Read, Write}, str::FromStr,
};

use clap::{
    error::{Error, ErrorKind},
    Parser, Subcommand,
};
use flate2::read::ZlibDecoder;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_key: String,
    },
}

enum FileType {
    Blob,
    Tree,
    Commit
}

#[derive(Debug, Clone)]
struct FileTypeParseError;
impl FromStr for FileType {
    type Err = FileTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blob" => Ok(FileType::Blob),
            "tree" => Ok(FileType::Tree),
            "commit" => Ok(FileType::Commit),
            _ => Err(FileTypeParseError)
        }
    }
}

fn main() -> Result<(), Error> {
    let args = Cli::parse();
    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        }
        Command::CatFile {
            pretty_print,
            object_key,
        } => {
            // TODO: prefix match object key to single result

            let object_path = format!(".git/objects/{}/{}", &object_key[..2], &object_key[2..]);

            let file = File::open(object_path).expect("Cannot open file");

            let zlib_decoder = ZlibDecoder::new(file);
            let mut reader = BufReader::new(zlib_decoder);
            let mut buf: Vec<u8> = Vec::new();
            reader.read_until(0, &mut buf).unwrap();

            let str = CStr::from_bytes_with_nul(&buf)
                .expect("header should end with nul")
                .to_str()
                .unwrap();

            let Some((file_type, size)) = str.split_once(' ') else {
                return Err(Error::new(ErrorKind::InvalidValue));
            };
            // let file_type = file_type.parse::<FileType>().unwrap();
            
            if file_type != "blob" {
                return Err(Error::new(ErrorKind::InvalidValue));
            }

            let size = size.parse::<usize>().expect("Size");
            buf.clear();
            buf.resize(size, 0);
            let _ = reader.read_exact(&mut buf).expect("read file error");

            let n = reader.read(&mut [0]).unwrap();
            assert_eq!(n, 0);

            let mut stdout = stdout().lock();
            let _ = stdout.write_all(&buf);
        }
    }
    Ok(())
}
