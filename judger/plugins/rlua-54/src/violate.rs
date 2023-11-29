use std::{net::TcpStream, fs::File, io::Write, ffi::CString};

pub fn cpu(){
    loop{};
}

pub fn mem(){
    let mut v = Vec::new();
    loop{
        v.push(0);
    }
}

pub fn disk(){
    let mut f = File::create("file.txt").unwrap();
    loop{
        f.write_all(b"Disk").unwrap();
    }
}

pub fn net(){
    let mut stream = TcpStream::connect("8.8.8.8").unwrap();
    loop{
        stream.write(b"Net").unwrap();
    }
}

pub fn syscall(){
    unsafe{
        let path=CString::new("/boot").unwrap();
        libc::umount2(path.as_ptr(), libc::MNT_DETACH);
    }
}