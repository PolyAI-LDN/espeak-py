use std::ffi::{CString, CStr, c_void};
use std::path::Path;
use std::ptr::{addr_of_mut, null, null_mut};

use espeak_sys::{espeakCHARS_AUTO ,espeak_AUDIO_OUTPUT, espeak_ERROR, espeak_Initialize, espeak_VOICE,
                 espeak_ListVoices, espeak_SetVoiceByName, espeak_SetVoiceByProperties, espeak_TextToPhonemes};
use parking_lot::{Mutex, const_mutex};
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::wrap_pyfunction;
use libc::{c_int, c_char};

/// Library functions that depend on internal lib state and must be locked
type LibFuncs = (
    unsafe extern "C" fn(name: *const c_char) -> espeak_ERROR,
    unsafe extern "C" fn(voice_spec: *mut espeak_VOICE) -> espeak_ERROR,
    unsafe extern "C" fn(textptr: *const *const c_void, textmode: c_int, phonememode: c_int) -> *const c_char
);
static LIB: Mutex<LibFuncs> = const_mutex((espeak_SetVoiceByName, espeak_SetVoiceByProperties, espeak_TextToPhonemes));

#[pymodule]
fn espeak_py(_py: Python, m: &PyModule) -> PyResult<()> {
    let try_paths = [
        "/usr/lib/x86_64-linux-gnu/espeak-ng-data", // linux apt install location
        "/usr/local/Cellar/espeak-ng/1.50/share/espeak-ng-data", // mac brew install location
        "/usr/local/share/espeak-ng-data", // source install locations
        "/usr/share/espeak-ng-data",
    ];
    let mut data_path: Option<CString> = None;
    for try_path in &try_paths {
        if Path::new(try_path).exists() {
            data_path = Some(CString::new(*try_path)?);
            break;
        }
    };
    let path_ptr = match data_path {
        None => return Err(PyRuntimeError::new_err("could not discover espeak data path; have you installed espeak data files?")),
        Some(ref c_path) => c_path.as_ptr(),
    };
    unsafe {
        let _rate = espeak_Initialize(espeak_AUDIO_OUTPUT::AUDIO_OUTPUT_RETRIEVAL, 0, path_ptr, 0);
    }
    drop(data_path);
    m.add_function(wrap_pyfunction!(text_to_phonemes, m)?)?;
    m.add_function(wrap_pyfunction!(list_voice_names, m)?)?;
    m.add_function(wrap_pyfunction!(list_languages, m)?)?;
    Ok(())
}

/// Convert a string to IPA. Raises RuntimeError if espeak doesn't have the requested voice.
#[pyfunction]
#[text_signature = "(text, language=None, voice_name=None, /)"]
fn text_to_phonemes(text: &str, language: Option<&str>, voice_name: Option<&str>) -> PyResult<String> {
    // borrow from mutex to lock entire lib until we're finished
    let (ref set_voice_by_name, ref set_voice_by_props, ref to_phonemes) = *(LIB.lock());
    match (language, voice_name) {
        (None, None) | (Some(_), Some(_)) => {
            return Err(PyRuntimeError::new_err("exactly one of 'language' and 'voice_name' must be passed"))
        }
        (None, Some(name)) => { // set voice by name
            let c_name = CString::new(name).unwrap();
            unsafe {
                match set_voice_by_name(c_name.as_ptr()) {
                    espeak_ERROR::EE_OK => (),
                    espeak_ERROR::EE_INTERNAL_ERROR => return Err(PyRuntimeError::new_err("espeak internal error while setting language")),
                    espeak_ERROR::EE_NOT_FOUND => return Err(PyRuntimeError::new_err(format!("voice '{}' not found; have you installed espeak data files?", name))),
                    _ => return Err(PyRuntimeError::new_err("espeak unknown error while setting language")),
                }
            }
            drop(c_name);
        }
        (Some(lang), None) => { // set voice by language
            let c_lang = CString::new(lang).unwrap();
            let mut voice_template = espeak_VOICE::new(null(), c_lang.as_ptr(), null(), 0, 0, 0);
            unsafe {
                match set_voice_by_props(addr_of_mut!(voice_template)) {
                    espeak_ERROR::EE_OK => (),
                    espeak_ERROR::EE_INTERNAL_ERROR => return Err(PyRuntimeError::new_err("espeak internal error while setting language")),
                    espeak_ERROR::EE_NOT_FOUND => return Err(PyRuntimeError::new_err(format!("language '{}' not found; have you installed espeak data files?", lang))),
                    _ => return Err(PyRuntimeError::new_err("espeak unknown error while setting language")),
                }
            }
            drop(lang);
        }
    }

    // run conversion
    let c_text = CString::new(text).unwrap();
    let mut c_text_ptr = c_text.as_ptr() as *const c_void;
    let espeak_text_ptr = addr_of_mut!(c_text_ptr); // pointer to pointer to the beginning of the text
    let phonememode = i32::from_be_bytes([0x00, 0x00, 0x00, 0b00000010]);
    let mut ipas = String::new();
    loop {
        let phonemes_ptr = unsafe {
            to_phonemes(espeak_text_ptr, espeakCHARS_AUTO as _, phonememode)
        };
        // copy phonemes into Rust heap
        let phonemes_cstr = unsafe { CStr::from_ptr(phonemes_ptr) };
        let phonemes_str = phonemes_cstr.to_str()?;
        ipas.push_str(phonemes_str);
        // if not null, we need to make another call
        if c_text_ptr.is_null() {
            return Ok(ipas)
        }
        // add newline to imitate espeak executable behavior
        ipas.push('\n');
    }
}

/// List the names of the voices supported by this installation of espeak
#[pyfunction]
#[text_signature = "(/)"]
fn list_voice_names() -> PyResult<Vec<String>> {
    let mut result = Vec::new();
    let mut voice_arr = unsafe { espeak_ListVoices(null_mut()) };
    while unsafe{ !(*voice_arr).is_null() } {
        let c_name = unsafe { CStr::from_ptr((**voice_arr).name) };
        let name = c_name.to_str()?;
        result.push(name.to_owned());
        voice_arr = voice_arr.wrapping_add(1);
    }
    Ok(result)
}

/// List the language codes supported by this installation of espeak
#[pyfunction]
#[text_signature = "(/)"]
fn list_languages() -> PyResult<Vec<String>> {
    let mut result = Vec::new();
    let mut voice_arr = unsafe { espeak_ListVoices(null_mut()) };
    while unsafe{ !(*voice_arr).is_null() } {
        let mut langs_ptr = unsafe { (**voice_arr).languages };
        while unsafe { *langs_ptr != 0 } {
            // get priority byte
            let _priority = unsafe { *langs_ptr };
            langs_ptr = langs_ptr.wrapping_add(1);
            // copy all language strings to rust heap
            let lang_start_ptr = langs_ptr as *const u8;
            let mut num_bytes: usize = 0;
            while unsafe { *langs_ptr != 0 } {
                num_bytes += 1;
                langs_ptr = langs_ptr.wrapping_add(1);
            }
            num_bytes += 1;
            let lang_slice = unsafe { std::slice::from_raw_parts(lang_start_ptr, num_bytes) };
            let lang_cstr = match CStr::from_bytes_with_nul(lang_slice) {
                Ok(cstr) => cstr,
                Err(_) => return Err(PyRuntimeError::new_err("espeak language string decoding error")),
            };
            let lang_str = lang_cstr.to_str()?;
            result.push(lang_str.to_owned());
        }
        voice_arr = voice_arr.wrapping_add(1);
    }
    Ok(result)
}
