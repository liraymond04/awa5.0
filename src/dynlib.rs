use core::panic;
use std::{collections::HashMap, fs, path::PathBuf, ptr};

#[cfg(not(target_arch = "wasm32"))]
use libloading::{Library, Symbol};

use crate::{interpreter::Bubble, AWA_SCII};

#[cfg(target_arch = "wasm32")]
use crate::awa5_raylib;

#[cfg(target_os = "windows")]
const LIB_EXTENSION: &str = "dll";

#[cfg(target_os = "linux")]
#[cfg(not(target_arch = "wasm32"))]
const LIB_EXTENSION: &str = "so";

#[cfg(target_os = "macos")]
#[cfg(not(target_arch = "wasm32"))]
const LIB_EXTENSION: &str = "dylib";

#[cfg(target_arch = "wasm32")]
const LIB_EXTENSION: &str = "wasm";

pub fn parse_fn_name(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap()
}

pub fn parse_fn_args(bubble: &Bubble) -> Vec<u8> {
    let mut args = Vec::new();

    let bubbles = bubble.get_bubbles();
    for bubble in bubbles {
        let bubbles = bubble.get_bubbles();
        let arg_type = bubbles[0].get_val();

        match arg_type {
            // i32 or f32
            0x0 => {
                let byte_array = bubbles[1].to_u8_array();
                args.extend_from_slice(&byte_array);
            }
            // awascii char
            0x1 => {
                let char = AWA_SCII.chars().nth(bubbles[1].get_val() as usize).unwrap();
                args.push(char as u8);
            }
            // ascii char
            0x2 => {
                args.push(bubbles[1].get_val() as u8);
            }
            // awascii string
            0x3 => {
                let mut new_string = String::new();
                for b in bubbles[1].get_bubbles().iter().rev() {
                    new_string.push(AWA_SCII.chars().nth(b.get_val() as usize).unwrap());
                }
                args.extend_from_slice(new_string.as_bytes());
                args.push('\0' as u8)
            }
            // ascii string
            0x4 => {
                for b in bubbles[1].get_bubbles().iter().rev() {
                    args.push(b.get_val() as u8);
                }
                args.push('\0' as u8)
            }
            // simple bubble value
            0x5 => {
                let byte_array = bubbles[1].get_val().to_le_bytes();
                args.extend_from_slice(&byte_array);
            }
            _ => {
                panic!("Invalid argument type provided: {}.", arg_type);
            }
        }
    }

    args
}

pub fn get_shared_library_paths(lib_dirs: &[&str]) -> Vec<String> {
    let mut lib_paths = Vec::new();

    for dir in lib_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path: PathBuf = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension == LIB_EXTENSION {
                            if let Some(path_str) = path.to_str() {
                                lib_paths.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    lib_paths
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_libs(lib_paths: &[&str]) -> HashMap<String, Library> {
    let mut libs = HashMap::new();

    for path in lib_paths {
        // Loading all candidate shared libraries can fail if some of them have
        // missing system dependencies (e.g. raylib on a headless machine).
        // We skip failures so that other libraries (like `libfoo.so`) still work.
        if let Ok(lib) = unsafe { Library::new(path) } {
            libs.insert(path.to_string(), lib);
        }
    }

    libs
}

#[cfg(target_arch = "wasm32")]
pub type LibFnWithArgs = unsafe extern "C" fn(*const u8, *mut *mut u8, *mut usize);
#[cfg(target_arch = "wasm32")]
pub type LibFnNoArgs = unsafe extern "C" fn();

#[cfg(target_arch = "wasm32")]
pub enum LibFn {
    WithArgs(LibFnWithArgs),
    NoArgs(LibFnNoArgs),
}

#[cfg(target_arch = "wasm32")]
pub fn load_libs(_lib_paths: &[&str]) -> HashMap<String, LibFn> {
    let mut libs = HashMap::new();

    libs.insert("initwindow".to_string(), LibFn::WithArgs(awa5_raylib::initwindow));
    libs.insert("settargetfps".to_string(), LibFn::WithArgs(awa5_raylib::settargetfps));
    libs.insert("clearbackground".to_string(), LibFn::WithArgs(awa5_raylib::clearbackground));
    libs.insert("drawtext".to_string(), LibFn::WithArgs(awa5_raylib::drawtext));
    libs.insert("iskeydown".to_string(), LibFn::WithArgs(awa5_raylib::iskeydown));
    libs.insert("drawcircle".to_string(), LibFn::WithArgs(awa5_raylib::drawcircle));
    libs.insert("setcameraposition".to_string(), LibFn::WithArgs(awa5_raylib::setcameraposition));
    libs.insert("setcameratarget".to_string(), LibFn::WithArgs(awa5_raylib::setcameratarget));
    libs.insert("setcameraup".to_string(), LibFn::WithArgs(awa5_raylib::setcameraup));
    libs.insert("setcamerafovy".to_string(), LibFn::WithArgs(awa5_raylib::setcamerafovy));
    libs.insert("setcameraprojection".to_string(), LibFn::WithArgs(awa5_raylib::setcameraprojection));
    libs.insert("beginmode3d".to_string(), LibFn::WithArgs(awa5_raylib::beginmode3d));
    libs.insert("drawcube".to_string(), LibFn::WithArgs(awa5_raylib::drawcube));
    libs.insert("drawcubewires".to_string(), LibFn::WithArgs(awa5_raylib::drawcubewires));
    libs.insert("drawgrid".to_string(), LibFn::WithArgs(awa5_raylib::drawgrid));
    libs.insert("loadmodel".to_string(), LibFn::WithArgs(awa5_raylib::loadmodel));
    libs.insert("unloadmodel".to_string(), LibFn::WithArgs(awa5_raylib::unloadmodel));
    libs.insert("drawmodel".to_string(), LibFn::WithArgs(awa5_raylib::drawmodel));
    libs.insert("drawmodelex".to_string(), LibFn::WithArgs(awa5_raylib::drawmodelex));
    libs.insert("loadtexture".to_string(), LibFn::WithArgs(awa5_raylib::loadtexture));
    libs.insert("setmaterialtexture".to_string(), LibFn::WithArgs(awa5_raylib::setmaterialtexture));
    libs.insert("addfloat".to_string(), LibFn::WithArgs(awa5_raylib::addfloat));
    libs.insert("BeginDrawing".to_string(), LibFn::NoArgs(awa5_raylib::BeginDrawing));
    libs.insert("EndDrawing".to_string(), LibFn::NoArgs(awa5_raylib::EndDrawing));
    libs.insert("EndMode3D".to_string(), LibFn::NoArgs(awa5_raylib::EndMode3D));

    libs
}

#[cfg(not(target_arch = "wasm32"))]
pub fn call_lib_fn(libs: &HashMap<String, Library>, fn_name: &str, args: Vec<u8>) -> Vec<u8> {
    let mut buffer: *mut u8 = ptr::null_mut();
    let mut buffer_len: usize = 0;

    for (_path, lib) in libs {
        unsafe {
            if let Ok(f) = lib
                .get::<Symbol<unsafe extern "C" fn(*const u8, *mut *mut u8, *mut usize)>>(
                    fn_name.as_bytes(),
                )
            {
                f(args.as_ptr(), &mut buffer, &mut buffer_len);
                if !buffer.is_null() {
                    return Vec::from_raw_parts(buffer, buffer_len, buffer_len);
                }
                return vec![];
            }
        }
    }

    panic!("Function not found: {}", fn_name);
}

#[cfg(target_arch = "wasm32")]
pub fn call_lib_fn(libs: &HashMap<String, LibFn>, fn_name: &str, args: Vec<u8>) -> Vec<u8> {
    match libs.get(fn_name) {
        Some(LibFn::WithArgs(lib_fn)) => {
            let mut buffer: *mut u8 = ptr::null_mut();
            let mut buffer_len: usize = 0;

            unsafe {
                lib_fn(args.as_ptr(), &mut buffer, &mut buffer_len);

                if !buffer.is_null() {
                    return Vec::from_raw_parts(buffer, buffer_len, buffer_len);
                }
                return vec![];
            }
        }
        Some(LibFn::NoArgs(lib_fn)) => {
            unsafe {
                lib_fn();
            }
            Vec::new()
        }
        None => {
            eprintln!("Function not found in library");
            Vec::new()
        }
    }
}
