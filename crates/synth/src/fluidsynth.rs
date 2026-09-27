//! Adaptador FFI acotado al API C de FluidSynth.
//!
//! El `FluidSynthEngine` debe crearse, usarse y destruirse en el mismo worker
//! dedicado. No se llama desde el callback PipeWire: las funciones de render
//! del backend pueden bloquear y el estado interno pertenece a su hilo de síntesis.

use libloading::Library;
use std::{
    ffi::{c_char, c_double, c_int, c_void, CStr, CString},
    path::Path,
};
use thiserror::Error;

const MINIMUM_MAJOR_VERSION: i32 = 2;

#[cfg(test)]
pub(crate) static FLUIDSYNTH_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

type NewSettings = unsafe extern "C" fn() -> *mut c_void;
type DeleteSettings = unsafe extern "C" fn(*mut c_void);
type SetNum = unsafe extern "C" fn(*mut c_void, *const c_char, c_double) -> c_int;
type NewSynth = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type DeleteSynth = unsafe extern "C" fn(*mut c_void);
type SoundFontLoad = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type SoundFontById = unsafe extern "C" fn(*mut c_void, c_int) -> *mut c_void;
type SoundFontIterationStart = unsafe extern "C" fn(*mut c_void);
type SoundFontIterationNext = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type PresetNumber = unsafe extern "C" fn(*mut c_void) -> c_int;
type PresetName = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type ProgramSelect = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int, c_int) -> c_int;
type NoteOn = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int) -> c_int;
type NoteOff = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type ControlChange = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int) -> c_int;
type ChannelValue = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type KeyValue = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int) -> c_int;
type WriteFloat = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    *mut c_void,
    c_int,
    c_int,
    *mut c_void,
    c_int,
    c_int,
) -> c_int;
type Version = unsafe extern "C" fn(*mut c_int, *mut c_int, *mut c_int);

/// Versión detectada en la biblioteca compartida del sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidSynthVersion {
    pub major: i32,
    pub minor: i32,
    pub micro: i32,
}

impl std::fmt::Display for FluidSynthVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.micro)
    }
}

/// Información mínima de un preset que se puede mostrar en la UI/CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluidSynthPreset {
    pub name: String,
    pub bank: u16,
    pub program: u8,
}

#[derive(Debug, Error)]
pub enum FluidSynthError {
    #[error("no se encontró una biblioteca FluidSynth compatible ({0}); instala el runtime libfluidsynth")]
    RuntimeUnavailable(String),
    #[error("la biblioteca FluidSynth no exporta el símbolo requerido '{0}'")]
    MissingSymbol(&'static str),
    #[error(
        "FluidSynth {found} no es compatible; se requiere la versión {minimum_major}.x o posterior"
    )]
    UnsupportedVersion {
        found: FluidSynthVersion,
        minimum_major: i32,
    },
    #[error("FluidSynth no pudo crear su configuración")]
    SettingsCreation,
    #[error("FluidSynth rechazó la frecuencia de muestreo {0} Hz")]
    InvalidSampleRate(u32),
    #[error("FluidSynth no pudo crear el sintetizador")]
    SynthCreation,
    #[error("la ruta del SoundFont no es UTF-8: {0}")]
    NonUtf8Path(String),
    #[error("no se pudo cargar el SoundFont '{path}' (FluidSynth id={code})")]
    SoundFontLoad { path: String, code: i32 },
    #[error("el SoundFont no contiene presets enumerables")]
    NoPresets,
    #[error("no existe el preset banco {bank}, programa {program}")]
    PresetNotFound { bank: u16, program: u8 },
    #[error("FluidSynth rechazó la selección del preset banco {bank}, programa {program}")]
    PresetSelection { bank: u16, program: u8 },
    #[error("FluidSynth falló al enviar un evento MIDI")]
    MidiEvent,
    #[error("FluidSynth falló al renderizar {frames} frames")]
    Render { frames: usize },
    #[error("la cantidad de frames excede el límite del API C")]
    BlockTooLarge,
}

/// Tabla de funciones del ABI. Se conserva `library` durante toda su vida para
/// impedir que los punteros a funciones queden colgando por unload prematuro.
struct FluidSynthApi {
    _library: Library,
    version: Version,
    new_settings: NewSettings,
    delete_settings: DeleteSettings,
    set_num: SetNum,
    new_synth: NewSynth,
    delete_synth: DeleteSynth,
    soundfont_load: SoundFontLoad,
    get_soundfont_by_id: SoundFontById,
    iteration_start: SoundFontIterationStart,
    iteration_next: SoundFontIterationNext,
    preset_bank: PresetNumber,
    preset_program: PresetNumber,
    preset_name: PresetName,
    program_select: ProgramSelect,
    note_on: NoteOn,
    note_off: NoteOff,
    control_change: ControlChange,
    pitch_bend: ChannelValue,
    channel_pressure: ChannelValue,
    program_change: ChannelValue,
    key_pressure: KeyValue,
    write_float: WriteFloat,
}

impl FluidSynthApi {
    fn load_system() -> Result<Self, FluidSynthError> {
        Self::load_candidates(system_library_names())
    }

    fn load_candidates(candidates: &[&str]) -> Result<Self, FluidSynthError> {
        let mut errors = Vec::new();
        for candidate in candidates {
            match unsafe { Library::new(candidate) } {
                Ok(library) => return unsafe { Self::from_library(library) },
                Err(error) => errors.push(format!("{candidate}: {error}")),
            }
        }
        Err(FluidSynthError::RuntimeUnavailable(errors.join("; ")))
    }

    unsafe fn from_library(library: Library) -> Result<Self, FluidSynthError> {
        // SAFETY: cada firma se copia de la API C pública de FluidSynth. La
        // biblioteca se mueve al struct y permanece cargada mientras se usen.
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                *library
                    .get::<$ty>(concat!($name, "\0").as_bytes())
                    .map_err(|_| FluidSynthError::MissingSymbol($name))?
            }};
        }

        let api = Self {
            version: symbol!("fluid_version", Version),
            new_settings: symbol!("new_fluid_settings", NewSettings),
            delete_settings: symbol!("delete_fluid_settings", DeleteSettings),
            set_num: symbol!("fluid_settings_setnum", SetNum),
            new_synth: symbol!("new_fluid_synth", NewSynth),
            delete_synth: symbol!("delete_fluid_synth", DeleteSynth),
            soundfont_load: symbol!("fluid_synth_sfload", SoundFontLoad),
            get_soundfont_by_id: symbol!("fluid_synth_get_sfont_by_id", SoundFontById),
            iteration_start: symbol!("fluid_sfont_iteration_start", SoundFontIterationStart),
            iteration_next: symbol!("fluid_sfont_iteration_next", SoundFontIterationNext),
            preset_bank: symbol!("fluid_preset_get_banknum", PresetNumber),
            preset_program: symbol!("fluid_preset_get_num", PresetNumber),
            preset_name: symbol!("fluid_preset_get_name", PresetName),
            program_select: symbol!("fluid_synth_program_select", ProgramSelect),
            note_on: symbol!("fluid_synth_noteon", NoteOn),
            note_off: symbol!("fluid_synth_noteoff", NoteOff),
            control_change: symbol!("fluid_synth_cc", ControlChange),
            pitch_bend: symbol!("fluid_synth_pitch_bend", ChannelValue),
            channel_pressure: symbol!("fluid_synth_channel_pressure", ChannelValue),
            program_change: symbol!("fluid_synth_program_change", ChannelValue),
            key_pressure: symbol!("fluid_synth_key_pressure", KeyValue),
            write_float: symbol!("fluid_synth_write_float", WriteFloat),
            _library: library,
        };

        let mut major = 0;
        let mut minor = 0;
        let mut micro = 0;
        (api.version)(&mut major, &mut minor, &mut micro);
        let version = FluidSynthVersion {
            major,
            minor,
            micro,
        };
        if major < MINIMUM_MAJOR_VERSION {
            return Err(FluidSynthError::UnsupportedVersion {
                found: version,
                minimum_major: MINIMUM_MAJOR_VERSION,
            });
        }
        Ok(api)
    }
}

/// Instancia de FluidSynth con ownership exclusivo. Sus métodos usan buffers
/// proporcionados por el caller; `render_interleaved` no asigna memoria.
pub struct FluidSynthEngine {
    api: FluidSynthApi,
    settings: *mut c_void,
    synth: *mut c_void,
    soundfont_id: i32,
    sample_rate: u32,
}

impl FluidSynthEngine {
    /// Carga el runtime, crea el synth y lee el banco en el hilo actual.
    /// El caller debe invocar esto desde el worker de instrumento, nunca RT.
    pub fn open(
        soundfont_path: impl AsRef<Path>,
        sample_rate: u32,
    ) -> Result<Self, FluidSynthError> {
        if sample_rate == 0 {
            return Err(FluidSynthError::InvalidSampleRate(sample_rate));
        }
        let path = soundfont_path.as_ref();
        let path_text = path.to_string_lossy().into_owned();
        let path_c = CString::new(
            path.to_str()
                .ok_or_else(|| FluidSynthError::NonUtf8Path(path_text.clone()))?,
        )
        .map_err(|_| FluidSynthError::NonUtf8Path(path_text.clone()))?;
        let api = FluidSynthApi::load_system()?;

        // SAFETY: el ABI valida punteros nulos en creación; settings y synth
        // quedan propiedad exclusiva de esta instancia y se destruyen en Drop.
        let settings = unsafe { (api.new_settings)() };
        if settings.is_null() {
            return Err(FluidSynthError::SettingsCreation);
        }
        let rate_key = b"synth.sample-rate\0";
        let rate_set =
            unsafe { (api.set_num)(settings, rate_key.as_ptr().cast(), sample_rate as f64) };
        if rate_set != 0 {
            unsafe { (api.delete_settings)(settings) };
            return Err(FluidSynthError::InvalidSampleRate(sample_rate));
        }
        let synth = unsafe { (api.new_synth)(settings) };
        if synth.is_null() {
            unsafe { (api.delete_settings)(settings) };
            return Err(FluidSynthError::SynthCreation);
        }
        let soundfont_id = unsafe { (api.soundfont_load)(synth, path_c.as_ptr(), 1) };
        if soundfont_id < 0 {
            unsafe {
                (api.delete_synth)(synth);
                (api.delete_settings)(settings);
            }
            return Err(FluidSynthError::SoundFontLoad {
                path: path_text,
                code: soundfont_id,
            });
        }

        Ok(Self {
            api,
            settings,
            synth,
            soundfont_id,
            sample_rate,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn version(&self) -> FluidSynthVersion {
        let mut major = 0;
        let mut minor = 0;
        let mut micro = 0;
        unsafe { (self.api.version)(&mut major, &mut minor, &mut micro) };
        FluidSynthVersion {
            major,
            minor,
            micro,
        }
    }

    /// Enumera los presets del banco cargado; debe ejecutarse fuera de RT.
    pub fn presets(&mut self) -> Result<Vec<FluidSynthPreset>, FluidSynthError> {
        let soundfont = unsafe { (self.api.get_soundfont_by_id)(self.synth, self.soundfont_id) };
        if soundfont.is_null() {
            return Err(FluidSynthError::NoPresets);
        }
        unsafe { (self.api.iteration_start)(soundfont) };
        let mut presets = Vec::new();
        loop {
            let preset = unsafe { (self.api.iteration_next)(soundfont) };
            if preset.is_null() {
                break;
            }
            let name = unsafe { (self.api.preset_name)(preset) };
            if name.is_null() {
                continue;
            }
            // SAFETY: FluidSynth mantiene el string del preset mientras el
            // SoundFont permanezca cargado; se copia al modelo Rust aquí.
            let name = unsafe { CStr::from_ptr(name) }
                .to_string_lossy()
                .into_owned();
            let bank = unsafe { (self.api.preset_bank)(preset) };
            let program = unsafe { (self.api.preset_program)(preset) };
            if (0..=u16::MAX as i32).contains(&bank) && (0..=u8::MAX as i32).contains(&program) {
                presets.push(FluidSynthPreset {
                    name,
                    bank: bank as u16,
                    program: program as u8,
                });
            }
        }
        if presets.is_empty() {
            Err(FluidSynthError::NoPresets)
        } else {
            Ok(presets)
        }
    }

    pub fn select_preset(&mut self, bank: u16, program: u8) -> Result<(), FluidSynthError> {
        let result = unsafe {
            (self.api.program_select)(
                self.synth,
                0,
                self.soundfont_id,
                bank as i32,
                program as i32,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::PresetSelection { bank, program })
        }
    }

    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<(), FluidSynthError> {
        if velocity == 0 {
            return self.note_off(channel, note);
        }
        let result =
            unsafe { (self.api.note_on)(self.synth, channel as i32, note as i32, velocity as i32) };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    pub fn note_off(&mut self, channel: u8, note: u8) -> Result<(), FluidSynthError> {
        let result = unsafe { (self.api.note_off)(self.synth, channel as i32, note as i32) };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    pub fn control_change(
        &mut self,
        channel: u8,
        controller: u8,
        value: u8,
    ) -> Result<(), FluidSynthError> {
        if channel >= 16 || controller >= 128 || value >= 128 {
            return Err(FluidSynthError::MidiEvent);
        }
        let result = unsafe {
            (self.api.control_change)(self.synth, channel as i32, controller as i32, value as i32)
        };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    pub fn pitch_bend(&mut self, channel: u8, value: i16) -> Result<(), FluidSynthError> {
        if channel >= 16 || !(-8_192..=8_191).contains(&value) {
            return Err(FluidSynthError::MidiEvent);
        }
        // El proyecto guarda el centro firmado MIDI (-8192..8191); FluidSynth
        // recibe el valor crudo no firmado (0..16383, centro 8192).
        let result =
            unsafe { (self.api.pitch_bend)(self.synth, channel as i32, i32::from(value) + 8_192) };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    pub fn channel_pressure(&mut self, channel: u8, pressure: u8) -> Result<(), FluidSynthError> {
        if channel >= 16 || pressure >= 128 {
            return Err(FluidSynthError::MidiEvent);
        }
        let result =
            unsafe { (self.api.channel_pressure)(self.synth, channel as i32, pressure as i32) };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    pub fn program_change(&mut self, channel: u8, program: u8) -> Result<(), FluidSynthError> {
        if channel >= 16 || program >= 128 {
            return Err(FluidSynthError::MidiEvent);
        }
        let result =
            unsafe { (self.api.program_change)(self.synth, channel as i32, program as i32) };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    pub fn key_pressure(
        &mut self,
        channel: u8,
        note: u8,
        pressure: u8,
    ) -> Result<(), FluidSynthError> {
        if channel >= 16 || note >= 128 || pressure >= 128 {
            return Err(FluidSynthError::MidiEvent);
        }
        let result = unsafe {
            (self.api.key_pressure)(self.synth, channel as i32, note as i32, pressure as i32)
        };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::MidiEvent)
        }
    }

    /// Renderiza stereo intercalado a un slice ya reservado por el worker.
    pub fn render_interleaved(&mut self, output: &mut [f32]) -> Result<(), FluidSynthError> {
        if output.len() % 2 != 0 {
            return Err(FluidSynthError::BlockTooLarge);
        }
        let frames = output.len() / 2;
        let frames_i32 = c_int::try_from(frames).map_err(|_| FluidSynthError::BlockTooLarge)?;
        let result = unsafe {
            (self.api.write_float)(
                self.synth,
                frames_i32,
                output.as_mut_ptr().cast::<c_void>(),
                0,
                2,
                output.as_mut_ptr().cast::<c_void>(),
                1,
                2,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(FluidSynthError::Render { frames })
        }
    }
}

impl Drop for FluidSynthEngine {
    fn drop(&mut self) {
        // SAFETY: estos punteros nacieron de la API cargada y se destruyen una
        // sola vez, primero synth y luego settings, en el hilo dueño.
        unsafe {
            (self.api.delete_synth)(self.synth);
            (self.api.delete_settings)(self.settings);
        }
    }
}

fn system_library_names() -> &'static [&'static str] {
    #[cfg(target_os = "linux")]
    {
        return &[
            "libfluidsynth.so.3",
            "libfluidsynth.so.2",
            "libfluidsynth.so",
        ];
    }
    #[cfg(target_os = "macos")]
    {
        return &["libfluidsynth.3.dylib", "libfluidsynth.dylib"];
    }
    #[cfg(target_os = "windows")]
    {
        return &["libfluidsynth-3.dll", "libfluidsynth.dll"];
    }
    #[allow(unreachable_code)]
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_runtime_reports_installation_guidance() {
        let message =
            match FluidSynthApi::load_candidates(&["libfluidsynth-estudio-daw-test-missing.so"]) {
                Err(FluidSynthError::RuntimeUnavailable(message)) => message,
                Err(other) => panic!("unexpected runtime diagnostic: {other}"),
                Ok(_) => panic!("a deliberately missing library unexpectedly loaded"),
            };
        assert!(message.contains("libfluidsynth"));
    }

    #[test]
    fn local_soundfont_renders_finite_non_silent_stereo_when_available() {
        let _guard = FLUIDSYNTH_TEST_LOCK.lock().unwrap();
        let path = Path::new("/usr/share/soundfonts/FluidR3_GM.sf2");
        if !path.is_file() {
            return;
        }
        let mut synth = FluidSynthEngine::open(path, 48_000).unwrap();
        synth.select_preset(0, 0).unwrap();
        synth.note_on(0, 69, 100).unwrap();
        let mut output = [0.0_f32; 2 * 512];
        synth.render_interleaved(&mut output).unwrap();
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > 1.0e-6));
        assert_eq!(synth.version().major, 2);
    }
}
