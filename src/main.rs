use std::io::prelude::*;
use std::mem::MaybeUninit;
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
extern crate libc;

fn main() {
    const MAX_EVENTS: usize = 10;
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut events =
        [unsafe { MaybeUninit::<libc::epoll_event>::uninit().assume_init() };
            MAX_EVENTS];

    let epollfd = unsafe { libc::epoll_create1(0) };
    let mut ev = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: listener.as_raw_fd() as u64,
    };
    unsafe {
        libc::epoll_ctl(
            epollfd,
            libc::EPOLL_CTL_ADD,
            listener.as_raw_fd(),
            &mut ev as *mut _,
        )
    };

    loop {
        let nfds = unsafe {
            libc::epoll_wait(
                epollfd,
                events.as_mut_ptr(),
                events.len() as i32,
                -1,
            )
        };
        for i in 0..nfds {
            if events[i as usize].u64 == listener.as_raw_fd() as u64 {
                let stream = listener.accept().unwrap();
                stream.0.set_nonblocking(true).unwrap();
                println!("Accept");
                let mut ev = libc::epoll_event {
                    events: libc::EPOLLIN as u32 | libc::EPOLLET as u32,
                    u64: stream.0.as_raw_fd() as u64,
                };
                unsafe {
                    libc::epoll_ctl(
                        epollfd,
                        libc::EPOLL_CTL_ADD,
                        stream.0.into_raw_fd(),
                        &mut ev as *mut _,
                    )
                };
            } else {
                let mut stream = unsafe {
                    TcpStream::from_raw_fd(events[i as usize].u64 as i32)
                };
                let mut buffer = [0; 512];
                let nb_read = stream.read(&mut buffer).unwrap();
                if nb_read == 0 {
                    // peer has performed an orderly shutdown
                    continue; // close the socket
                }
                println!("Echo");
                let msg = &buffer[0..nb_read];
                let nb_write = stream.write(msg).unwrap();
                if nb_read != nb_write {
                    println!("Need to save missing data + epoll_ctl EPOLLOUT");
                }
                stream.into_raw_fd(); // Avoid to close the socket
            }
        }
    }
}
