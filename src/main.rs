use std::{
    ffi::CStr,
    fmt::Display,
    fs::{self, File},
    io::{stdout, BufRead, BufReader, Read, Write},
    path::PathBuf,
    str::FromStr,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{digest::Update, Digest};

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
    HashObject {
        file_path: PathBuf,

        #[clap(short = 'w')]
        write: bool,
    },
}

enum FileType {
    Blob,
    Tree,
    Commit,
}

#[derive(Debug)]
struct FileTypeParseError;

impl Display for FileTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown object type")
    }
}

impl std::error::Error for FileTypeParseError {}

impl FromStr for FileType {
    type Err = FileTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blob" => Ok(FileType::Blob),
            "tree" => Ok(FileType::Tree),
            "commit" => Ok(FileType::Commit),
            _ => Err(FileTypeParseError),
        }
    }
}

struct HashObjectWriter {
    hasher: sha1::Sha1,
    writer: Box<dyn Write>,
}

impl Write for HashObjectWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Update::update(&mut self.hasher, buf);
        self.writer.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    match args.command {
        Command::Init => {
            fs::create_dir(".git").context("Failed to create .git directory")?;
            fs::create_dir(".git/objects").context("Failed to create .git/objects directory")?;
            fs::create_dir(".git/refs").context("Failed to create .git/refs directory")?;
            fs::write(".git/HEAD", "ref: refs/heads/main\n")
                .context("Failed to initialize HEAD ref")?;
            println!("Git repo inited!");
        }
        Command::CatFile {
            pretty_print,
            object_key,
        } => {
            // TODO: prefix match object key to single result
            if !pretty_print {
                return Ok(());
            }

            let file = File::open(format!(
                ".git/objects/{}/{}",
                &object_key[..2],
                &object_key[2..]
            ))
            .context("Cannot open {object_path}")?;

            let zlib_decoder = ZlibDecoder::new(file);
            let mut reader = BufReader::new(zlib_decoder);
            let mut buf: Vec<u8> = Vec::new();
            reader
                .read_until(0, &mut buf)
                .context("Reading git object header")?;

            let str = CStr::from_bytes_with_nul(&buf)
                .context("header should end with nul")?
                .to_str()
                .context("Convert CStr to str")?;

            let Some((file_type, size)) = str.split_once(' ') else {
                anyhow::bail!("Unknown header format: {str}, expecting '<object_type> <size>'");
            };
            file_type
                .parse::<FileType>()
                .context("Unknown object type: {file_type}")?;

            // TODO: dynamic type for size, big files might need more than usize for content size
            let size = size.parse::<u64>().context("Parsing content size")?;

            let mut reader = reader.take(size.try_into().context("Trying to convert size to u64")?);
            let mut stdout = stdout().lock();

            // TODO: proper handling of commit and tree objects
            std::io::copy(&mut reader, &mut stdout).context("Printing file content")?;
        }
        Command::HashObject { file_path, write } => {
            let metadata = fs::metadata(&file_path).context("Stating the file")?;
            let size = metadata.len();
            let mut file = File::open(file_path).context("Opening file")?;
            let content_sink: Box<dyn Write> = if write {
                Box::new(File::create_new(".git/objects/temp_file").context("Creating temp file")?)
            } else {
                Box::new(std::io::sink())
            };
            let mut writer = HashObjectWriter {
                hasher: sha1::Sha1::new(),
                writer: Box::new(ZlibEncoder::new(content_sink, Compression::default())),
            };

            write!(writer, "blob {size}\0").context("Writing header")?;
            std::io::copy(&mut file, &mut writer).context("Writing file content")?;
            writer.flush()?;
            let hash = writer.hasher.finalize();
            let hash = hex::encode(hash);
            println!("{hash}");
            if write {
                fs::create_dir_all(format!(".git/objects/{}", &hash[..2]))
                    .context("Creating object dir")?;
                fs::rename(
                    ".git/objects/temp_file",
                    format!(".git/objects/{}/{}", &hash[..2], &hash[2..]),
                )
                .context("Move temp file to actual file")?;
            }
        }
    }
    Ok(())
}
