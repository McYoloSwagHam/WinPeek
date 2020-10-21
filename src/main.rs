mod util;
use std::ptr::{null_mut, size_of, uninitialized};
use std::mem;
use winapi::um::winuser;
use winapi::um::wingdi;
use winapi::shared::windef;

fn handle_message( window_handle : windef::HWND) -> bool {
    unsafe {
        let mut message : winuser::MSG = uninitialized();
        if winuser::GetMessageW( &mut message as *mut winuser::MSG, window_handle, 0, 0 ) > 0 {
            winuser::TranslateMessage( &message as *const winuser::MSG );
            winuser::DispatchMessageW( &message as *const winuser::MSG );
            true
        } else {
            false
        }
    }
}

//AlphaKey on Transparent windows affects the border
//we want the borders to remain opaque
unsafe fn draw_transparent_parts(window_handle : windef::HWND) {

    let general_dc = winuser::GetDC(window_handle);

    let window_rect : windef::RECT;

    winuser::GetWindowRect(window_handle, &mut window_rect as *mut windef::RECT);

    let window_width = window_rect.right - window_rect.left;
    let window_height = window_rect.bottom - window_rect.top;

    if general_dc == null_mut() {
        panic!("Couldn't get DC for window");
    }

    //zero everything
    let mut bitmap_header : wingdi::BITMAPINFOHEADER = mem::zeroed();

    bitmap_header.biSize = size_of::<wingdi::BITMAPINFOHEADER>();
    bitmap_header.biBitCount = 32;
    bitmap_header.biWidth = window_width;
    bitmap_header.biHeight = window_height;
    bitmap_header.biPlanes = 1;

    let mut bitmap_info : wingdi::BITMAPINFO = mem::zeroed();

    bitmap_info.bmiHeader = bitmap_header;

    let mut bits: *mut winapi::ctypes::c_void = mem::zeroed();

    //Create bitmap to color in
    let bitmap_handle = wingdi::CreateDIBSection(general_dc,
                             &bitmap_info,
                             wingdi::DIB_RGB_COLORS, 
                             &mut bits,
                             null_mut(),
                             0);


    //let compat_dc = wingdi::CreateCompatibleDC(general_dc);

    //these are the pixels we are given access to
    let pixels : *mut u32 = mem::transmute::<*mut winapi::ctypes::c_void, *mut u32>(bits);




}


fn main() {
    println!("Hello, world!");

    let class_name = util::to_wstring("WinPeekClass");
    let window_name = util::to_wstring("WinPeek");


    let wnd_class = winuser::WNDCLASSEXW {
        cbSize : size_of::<winuser::WNDCLASSEXW>() as u32,
        style : 0,
        lpfnWndProc : Some(winuser::DefWindowProcW),
        cbClsExtra : 0,
        cbWndExtra : 0,
        hInstance : null_mut(),
        hIcon : null_mut(),
        hCursor : null_mut(),
        hbrBackground : null_mut(),
        lpszMenuName : null_mut(),
        lpszClassName : class_name.as_ptr(),
        hIconSm : null_mut(),
    };

    unsafe { 
        if winuser::RegisterClassExW(&wnd_class) == 0 {
            panic!("RegisterClass Failed");
        }
    };

    let window_handle = unsafe {
        winuser::CreateWindowExW(winuser::WS_EX_LAYERED,
                             class_name.as_ptr(),
                             window_name.as_ptr(),
                             winuser::WS_OVERLAPPEDWINDOW, 
                             winuser::CW_USEDEFAULT,
                             winuser::CW_USEDEFAULT,
                             winuser::CW_USEDEFAULT,
                             winuser::CW_USEDEFAULT,
                             null_mut(),
                             null_mut(),
                             null_mut(),
                             null_mut())
    };

    unsafe {
        draw_transparent_parts(window_handle);
    }

    loop {
        if !handle_message(window_handle) {
            break;
        }
    }

}
