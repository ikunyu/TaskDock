// Native acceptance fixture: reserves Ctrl+Alt+Y until a line is received.
// No keyboard hooks or input collection. Never linked into TaskDock.
#[link(name="user32")]
unsafe extern "system" {
    fn RegisterHotKey(hwnd:*mut std::ffi::c_void,id:i32,modifiers:u32,key:u32)->i32;
    fn UnregisterHotKey(hwnd:*mut std::ffi::c_void,id:i32)->i32;
}
fn main(){
    let registered=unsafe{RegisterHotKey(std::ptr::null_mut(),41,0x4000|0x2|0x1,0x59)};
    if registered==0 {eprintln!("RESERVATION_FAILED");std::process::exit(2);}
    println!("READY_CTRL_ALT_Y");
    let mut line=String::new();let _=std::io::stdin().read_line(&mut line);
    unsafe{UnregisterHotKey(std::ptr::null_mut(),41);}
    println!("RELEASED");
}
