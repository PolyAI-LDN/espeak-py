use std::ffi::{CString, CStr, c_void};
use std::path::Path;
use std::ptr::{addr_of_mut, null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};

use espeak_sys::{espeakCHARS_AUTO, espeakINITIALIZE_DONT_EXIT, espeak_AUDIO_OUTPUT, espeak_ERROR,
    espeak_Initialize, espeak_TextToPhonemes, espeak_SetVoiceByProperties, espeak_SetVoiceByName,
    espeak_ListVoices ,espeak_VOICE};
use parking_lot::{Mutex, const_mutex};
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::wrap_pyfunction;
use libc::{c_int, c_char};

/// Library functions that depend on internal lib state and must be locked
struct EspeakLib {
    initialize: unsafe extern "C" fn(output: espeak_AUDIO_OUTPUT, buflength: c_int, path: *const c_char, options: c_int) -> c_int,
    set_voice_by_name: unsafe extern "C" fn(name: *const c_char) -> espeak_ERROR,
    set_voice_by_props: unsafe extern "C" fn(voice_spec: *mut espeak_VOICE) -> espeak_ERROR,
    text_to_phonemes: unsafe extern "C" fn(textptr: *const *const c_void, textmode: c_int, phonememode: c_int) -> *const c_char,
}
static LIB: Mutex<EspeakLib> = const_mutex(EspeakLib {
    initialize: espeak_Initialize,
    set_voice_by_name: espeak_SetVoiceByName,
    set_voice_by_props: espeak_SetVoiceByProperties,
    text_to_phonemes: espeak_TextToPhonemes,
});
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Lazily initialize the library, but ensure we only do it once
fn ensure_initialized() -> PyResult<()> {
    if !INITIALIZED.load(Ordering::Acquire) {
        let lib = LIB.lock();
        if INITIALIZED.load(Ordering::Acquire) {
            return Ok(())
        }
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

        #[cfg(target_os = "macos")]
        let msg = r#"Error while initializing espeak.  If it's not installed, try running:
        `brew install anarchivist/espeak-ng/espeak-ng --without-pcaudiolib --without-waywardgeek-sonic`"#;
        #[cfg(target_os = "linux")]
        let msg = r#"Error while initializing espeak. If you haven't installed the data files, run:
        `sudo apt install espeak-ng-data`"#;
        let path_ptr = match data_path {
            None => return Err(PyRuntimeError::new_err(msg)),
            Some(ref c_path) => c_path.as_ptr(),
        };
        let rate = unsafe {
            (lib.initialize)(espeak_AUDIO_OUTPUT::AUDIO_OUTPUT_RETRIEVAL, 0, path_ptr, espeakINITIALIZE_DONT_EXIT)
        };
        if rate == -1 {
            return Err(PyRuntimeError::new_err(msg));
        }
        drop(data_path); // ensure data_path outlives path_ptr
        INITIALIZED.store(true, Ordering::Release);
    }
    Ok(())
}

/// Convert a string to IPA. Raises RuntimeError if espeak doesn't have the requested voice.
#[pyfunction]
#[text_signature = "(text, language=None, voice_name=None, /)"]
pub fn text_to_phonemes(text: &str, language: Option<&str>, voice_name: Option<&str>) -> PyResult<String> {
    // borrow from mutex to lock entire lib until we're finished
    let lib = LIB.lock();
    ensure_initialized()?;
    match (language, voice_name) {
        (None, None) | (Some(_), Some(_)) => {
            return Err(PyRuntimeError::new_err("exactly one of 'language' and 'voice_name' must be passed"))
        }
        (None, Some(name)) => { // set voice by name
            let c_name = CString::new(name).unwrap();
            unsafe {
                match (lib.set_voice_by_name)(c_name.as_ptr()) {
                    espeak_ERROR::EE_OK => (),
                    espeak_ERROR::EE_INTERNAL_ERROR => return Err(PyRuntimeError::new_err("espeak internal error while setting language")),
                    espeak_ERROR::EE_NOT_FOUND => return Err(PyRuntimeError::new_err(format!("voice '{}' not found; have you installed espeak data files?", name))),
                    _ => return Err(PyRuntimeError::new_err("espeak unknown error while setting language")),
                }
            }
        }
        (Some(lang), None) => { // set voice by language
            let c_lang = CString::new(lang).unwrap();
            let mut voice_template = espeak_VOICE::new(null(), c_lang.as_ptr(), null(), 0, 0, 0);
            unsafe {
                match (lib.set_voice_by_props)(addr_of_mut!(voice_template)) {
                    espeak_ERROR::EE_OK => (),
                    espeak_ERROR::EE_INTERNAL_ERROR => return Err(PyRuntimeError::new_err("espeak internal error while setting language")),
                    espeak_ERROR::EE_NOT_FOUND => return Err(PyRuntimeError::new_err(format!("language '{}' not found; have you installed espeak data files? see https://github.com/PolyAI-LDN/espeak-py", lang))),
                    _ => return Err(PyRuntimeError::new_err("espeak unknown error while setting language")),
                }
            }
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
            (lib.text_to_phonemes)(espeak_text_ptr, espeakCHARS_AUTO as _, phonememode)
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
pub fn list_voice_names() -> PyResult<Vec<String>> {
    ensure_initialized()?;
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
pub fn list_languages() -> PyResult<Vec<String>> {
    ensure_initialized()?;
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

#[pymodule]
fn espeak_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(list_voice_names, m)?)?;
    m.add_function(wrap_pyfunction!(list_languages, m)?)?;
    m.add_function(wrap_pyfunction!(text_to_phonemes, m)?)?;
    Ok(())
}
