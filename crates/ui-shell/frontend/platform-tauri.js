// Adaptador de escritorio. main.js sólo consume esta interfaz de plataforma;
// una compilación web podrá proporcionar otra implementación sin tocar la UI.
const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

window.estudioPlatform = {
  newProject: () => invoke("new_project"),
  demoMidiProject: () => invoke("demo_midi_project"),
  addTrack: (kind) => invoke("add_track", { kind }),
  async selectAudioFile() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Importar archivo de audio",
      filters: [{ name: "Audio", extensions: ["wav", "aiff", "aif", "flac", "ogg", "opus", "mp3", "m4a"] }],
    });
    return typeof path === "string" ? path : null;
  },
  importAudio({ path, trackId, copyIntoProject, startTick, sourceChannelSelection }) {
    return invoke("import_audio", { path, trackId, copyIntoProject, startTick, sourceChannelSelection });
  },
  editAudioRegion({ action, clipId, startTick, sourceStartSamples, durationSamples }) {
    return invoke("edit_audio_region", { action, clipId, startTick, sourceStartSamples, durationSamples });
  },
  audioWaveform: (sourceId) => invoke("audio_waveform", { sourceId }),
  audioPreview: (sourceId) => invoke("audio_preview", { sourceId }),
  audioPreviewFile: (path) => invoke("audio_preview_file", { path }),
  inspectAudioFile: (path) => invoke("inspect_audio_file", { path }),
  async openProject() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Abrir proyecto Estudio DAW",
      filters: [{ name: "Proyecto Estudio DAW", extensions: ["json"] }],
    });
    if (typeof path !== "string") return null;
    return invoke("open_project", { path });
  },
  saveProject: () => invoke("save_project"),
  async saveProjectAs() {
    const path = await save({
      title: "Guardar proyecto Estudio DAW como",
      defaultPath: "proyecto.json",
      filters: [{ name: "Proyecto Estudio DAW", extensions: ["json"] }],
    });
    if (!path) return null;
    return invoke("save_project_as", { path });
  },
  setTransport: (command, positionTicks = null) => invoke("set_transport", { command, positionTicks }),
  setTrackMixer: (trackId, mixer) => invoke("set_track_mixer", { trackId, ...mixer }),
  setTrackOutput: (trackId, outputTrackId) => invoke("set_track_output", { trackId, outputTrackId }),
  setTracksGroup: (trackIds, groupName) => invoke("set_tracks_group", { trackIds, groupName }),
  transportPosition: () => invoke("transport_position"),
  trackMeters: () => invoke("track_meters"),
  setLoopRange: (startTick, endTick) => invoke("set_loop_range", { startTick, endTick }),
  historyAction: (action) => invoke("history_action", { action }),
  audioRuntimeSettings: () => invoke("audio_runtime_settings"),
  saveAudioSettings: (settings) => invoke("save_audio_settings", { settings }),
};
