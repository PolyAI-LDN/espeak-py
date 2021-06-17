use std::ffi::{CString, CStr, c_void};
use std::ptr::{addr_of_mut, null, null_mut};

use espeak_sys::{espeakCHARS_AUTO ,espeak_AUDIO_OUTPUT, espeak_ERROR, espeak_Initialize,
                 espeak_ListVoices, espeak_SetVoiceByName, espeak_TextToPhonemes};
use parking_lot::{Mutex, const_mutex};
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::wrap_pyfunction;
use libc::{c_int, c_char};

/// Library functions that depend on internal lib state
type LibFuncs = (
    unsafe extern "C" fn(*const c_char) -> espeak_ERROR,
    unsafe extern "C" fn(textptr: *const *const c_void, textmode: c_int, phonememode: c_int) -> *const c_char
);
static LIB: Mutex<LibFuncs> = const_mutex((espeak_SetVoiceByName, espeak_TextToPhonemes));

#[pymodule]
fn espeak_py(_py: Python, m: &PyModule) -> PyResult<()> {
    let data_path;
    let path_ptr;
    #[cfg(target_os = "linux")]
    {
        data_path = CString::new("/usr/lib/x86_64-linux-gnu/espeak-ng-data")?;
        path_ptr = data_path.as_ptr();
    }
    #[cfg(not(target_os = "linux"))]
    {
        path_ptr = null();
    }
    unsafe {
        let _rate = espeak_Initialize(espeak_AUDIO_OUTPUT::AUDIO_OUTPUT_RETRIEVAL, 0, path_ptr, 0);
    }
    drop(data_path);
    m.add_function(wrap_pyfunction!(text_to_phonemes, m)?).unwrap();
    m.add_function(wrap_pyfunction!(list_voices, m)?).unwrap();
    Ok(())
}

/// Convert a string to IPA. Raises RuntimeError if espeak doesn't have the specified language.
#[pyfunction]
#[text_signature = "(text, language, /)"]
fn text_to_phonemes(text: &str, language: &str) -> PyResult<String> {
    // set language
    let lang = CString::new(language).unwrap();
    let (set_voice, to_phonemes) = *(LIB.lock());
    unsafe {
        match set_voice(lang.as_ptr()) {
            espeak_ERROR::EE_OK => (),
            espeak_ERROR::EE_INTERNAL_ERROR => return Err(PyRuntimeError::new_err("espeak internal error while setting language")),
            _ => return Err(PyRuntimeError::new_err("espeak unknown error while setting language")),
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
fn list_voices() -> PyResult<Vec<String>> {
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
