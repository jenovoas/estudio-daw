// Adaptador de escritorio. main.js sólo consume esta interfaz de plataforma;
// una compilación web podrá proporcionar otra implementación sin tocar la UI.
const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

window.estudioPlatform = {
  newProject: () => invoke("new_project"),
  demoMidiProject: () => invoke("demo_midi_project"),
  addTrack: (kind) => invoke("add_track", { kind }),
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
  setTransport: (command) => invoke("set_transport", { command }),
  historyAction: (action) => invoke("history_action", { action }),
  audioRuntimeSettings: () => invoke("audio_runtime_settings"),
  saveAudioSettings: (settings) => invoke("save_audio_settings", { settings }),
};
