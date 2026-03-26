use std::env::args;
use std::fs::File;
use std::io::{Read, Result};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::exit;
use xxhash_rust::xxh64::Xxh64;

const CHUNK_SIZE: usize = 1024 * 1024 * 10;

// print hash and inode number of file
fn file(f: &Path) -> Result<()> {
    if !f.is_file() {
        eprint!("Not a file: {}", f.display());
        exit(1);
    }

    // get inode of file
    let md = f.metadata()?;
    let inode: u64 = md.ino();

    // get hash of file
    //
    // stream file content into hash function instead of loading the complete file into memory (I
    // have some multiple GBs large files)
    let flen: u64 = md.len();
    let mut fh: File = File::open(f)?;

    // we have to use a vector to make sure we completely stay on the heap, lest we overflow the
    // stack. Even Box::new([0;CHUNK_SIZE]) would overflow the stack...
    let mut chunk_buf: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
    chunk_buf.resize(CHUNK_SIZE, 0);

    // read chunks, updating hasher
    let mut hasher = Xxh64::new(0);
    let mut remaining_bytes: u64 = flen;
    while remaining_bytes > CHUNK_SIZE as u64 {
        fh.read_exact(&mut chunk_buf)?;
        hasher.update(chunk_buf.as_slice());
        remaining_bytes -= CHUNK_SIZE as u64;
    }

    // read last few bytes
    let mut rest: Vec<u8> = Vec::new();
    fh.read_to_end(&mut rest)?;
    hasher.update(rest.as_slice());

    // finalize hasher
    let hash: u64 = hasher.digest();

    // print hash, inode, file size and file name
    println!("{:016x} {} ({} bytes) {}", hash, inode, flen, f.display());

    Ok(())
}

// recurse into directory
fn dir(d: &Path) {
    if !d.is_dir() {
        eprint!("Not a directory: {}", d.display());
        exit(1);
    }

    // recurse
    for entry in d.read_dir()
        .unwrap_or_else(|_| panic!("Can't read directory: {:?}", d))
    {
        let entry = entry.unwrap_or_else(
            |_| panic!("Can't read entry from directory {:?}", d));
        dispatch(&entry.path());
    }
}

fn dispatch(p: &Path) {
    // ignore symlinks. Either they point outside the tree we're traversing (in which case the
    // target isn't relevant) or they don't (in which case we'll process the target sooner or later
    // anyway)
    if p.is_symlink() {
        return;
    }

    if p.is_dir() {
        dir(&p);
    } else if p.is_file() {
        match file(&p) {
            Ok(()) => {},
            Err(e) => eprintln!("Error in file {p:?}: {e}"),
        };
    } else {
        eprint!("Ignoring item, since it's none of file, directory, symlink: {}\n", p.display());
    }
}

fn main() {

    // process all arguments
    let mut args = args().skip(1);
    let mut processed = 0;
    loop {
        match args.next() {
            // arg is either a file or a dir to recurse into
            Some(arg) => dispatch(&Path::new(&arg)),
            None => break,
        };
        processed += 1;
    }

    // if no arguments were processed, user is stoopid
    if processed == 0 {
        eprint!("Need arg(s): File or directory to recurse into\n");
        exit(1);
    }
}
