use std::env::args;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::exit;
use xxhash_rust::xxh64::Xxh64;

const CHUNK_SIZE: usize = 1024 * 1024 * 10;

// print hash and inode number of file
fn file(f: &Path) {
    std::io::stdout().flush().unwrap();
    if !f.is_file() {
        eprint!("Not a file: {}", f.display());
        exit(1);
    }

    // get inode of file
    let md = f.metadata().unwrap_or_else(
        |_| panic!("Can't read metadata from file: {:?}", f));
    let inode: u64 = md.ino();

    // get hash of file
    //
    // stream file content into hash function instead of loading the complete file into memory (I
    // have some multiple GBs large files)
    let flen: u64 = md.len();
    let mut fh: File = File::open(f).unwrap_or_else(
        |_| panic!("Cannot open file: {:?}", f));

    // we have to use a vector to make sure we completely stay on the heap, lest we overflow the
    // stack. Even Box::new([0;CHUNK_SIZE]) would overflow the stack...
    let mut chunk_buf: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
    chunk_buf.resize(CHUNK_SIZE, 0);

    // read chunks, updating hasher
    let mut hasher = Xxh64::new(0);
    let mut remaining_bytes: u64 = flen;
    while remaining_bytes > CHUNK_SIZE as u64 {
        fh.read_exact(&mut chunk_buf).unwrap_or_else(
            |_| panic!("Cannot read {} bytes from file: {:?}", CHUNK_SIZE, f));
        hasher.update(chunk_buf.as_slice());
        remaining_bytes -= CHUNK_SIZE as u64;
    }

    // read last few bytes
    let mut rest: Vec<u8> = Vec::new();
    fh.read_to_end(&mut rest).unwrap_or_else(
        |_| panic!("Failed reading rest of file: {:?}", f));
    hasher.update(rest.as_slice());

    // finalize hasher
    let hash: u64 = hasher.digest();

    // print hash, inode and file name
    println!("{:016x} {} {}", hash, inode, f.display());
}

// recurse into directory
fn dir(d: &Path) {
    if !d.is_dir() {
        eprint!("Not a directory: {}", d.display());
        exit(1);
    }

    // recurse
    for entry in d.read_dir().unwrap_or_else(
        |_| panic!("Cannot read directory: {:?}", d))
    {
        let entry = entry.unwrap_or_else(
            |_| panic!("Can't read entry from directory {:?}", d));
        dispatch(&entry.path());
    }
}

fn dispatch(p: &Path) {
    if p.is_dir() {
        dir(&p);
    } else if p.is_file() {
        file(&p);
    } else {
        eprint!("Ignoring item, since it's neither a file nor a directory: {}\n", p.display());
    }
}

fn main() {

    // get arg
    let arg: String = match args().skip(1).next() {
        Some(s) => s,
        None => {
            eprint!("Need arg: File or directory to recurse into\n");
            exit(1);
        },
    };

    // arg is either a file or a dir to recurse into
    let f_or_d = Path::new(&arg);
    dispatch(&f_or_d);
}
