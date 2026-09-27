const platform = window.estudioPlatform;

const elements = {
  open: document.querySelector("#open-project"),
  browserOpen: document.querySelector("#browser-open"),
  importAudio: document.querySelector("#browser-import-audio"),
  importPreview: document.querySelector("#audio-import-preview"),
  importCommit: document.querySelector("#audio-import-commit"),
  importSelected: document.querySelector("#audio-import-selected"),
  importTrack: document.querySelector("#audio-import-track"),
  importBar: document.querySelector("#audio-import-bar"),
  editCursorPosition: document.querySelector("#edit-cursor-position"),
  importMode: document.querySelector("#audio-import-mode"),
  importChannels: document.querySelector("#audio-import-channels"),
  browserDemo: document.querySelector("#browser-demo"),
  newProject: document.querySelector("#new-project"),
  demoProject: document.querySelector("#demo-project"),
  addMidiTrack: document.querySelector("#add-midi-track"),
  addAudioTrack: document.querySelector("#add-audio-track"),
  addBusTrack: document.querySelector("#add-bus-track"),
  save: document.querySelector("#save-project"),
  saveAs: document.querySelector("#save-project-as"),
  zoomIn: document.querySelector("#zoom-in"),
  zoomOut: document.querySelector("#zoom-out"),
  zoomReset: document.querySelector("#zoom-reset"),
  zoomLevel: document.querySelector("#zoom-level"),
  applicationMenu: document.querySelector("#application-menu"),
  contextMenu: document.querySelector("#action-context-menu"),
  path: document.querySelector("#project-path"),
  name: document.querySelector("#project-name"),
  browserName: document.querySelector("#browser-project-name"),
  browserMediaSearch: document.querySelector("#browser-media-search"),
  browserMediaChannels: document.querySelector("#browser-media-channels"),
  browserMediaList: document.querySelector("#browser-media-list"),
  tempo: document.querySelector("#tempo"),
  transportTempo: document.querySelector("#transport-tempo"),
  transport: document.querySelector("#transport-state"),
  transportPosition: document.querySelector("#transport-position"),
  loopRangeReadout: document.querySelector("#loop-range-readout"),
  loopPointA: document.querySelector("#loop-point-a"),
  loopPointB: document.querySelector("#loop-point-b"),
  loopRangeClear: document.querySelector("#loop-range-clear"),
  tracks: document.querySelector("#track-list"),
  lanes: document.querySelector("#arrangement-lanes"),
  ruler: document.querySelector("#timeline-ruler"),
  arrangementView: document.querySelector(".arrangement-scroll"),
  sessionView: document.querySelector("#session-view"),
  mixerView: document.querySelector("#mixer-view"),
  arrangementLegend: document.querySelector("#arrangement-legend"),
  showArrangement: document.querySelector("#show-arrangement"),
  showSession: document.querySelector("#show-session"),
  showMixer: document.querySelector("#show-mixer"),
  gridSnap: document.querySelector("#grid-snap"),
  railArrangement: document.querySelector("#rail-arrangement"),
  railSession: document.querySelector("#rail-session"),
  railSettings: document.querySelector("#rail-settings"),
  audioSettings: document.querySelector("#audio-settings"),
  clipInspector: document.querySelector("#clip-inspector"),
  trackCount: document.querySelector("#track-count"),
  midiCount: document.querySelector("#midi-count"),
  audioCount: document.querySelector("#audio-count"),
  revision: document.querySelector("#revision"),
  projectStatus: document.querySelector("#project-status"),
  engine: document.querySelector("#engine-status"),
  noticeTitle: document.querySelector("#notice-title"),
  noticeText: document.querySelector("#notice-text"),
  play: document.querySelector("#play"),
  record: document.querySelector("#record"),
  pause: document.querySelector("#pause"),
  stop: document.querySelector("#stop"),
  panic: document.querySelector("#panic"),
  metronome: document.querySelector("#metronome"),
  undo: document.querySelector("#undo"),
  redo: document.querySelector("#redo"),
  audioProfile: document.querySelector("#audio-profile"),
  audioOutputDevice: document.querySelector("#audio-output-device"),
  devicePeriod: document.querySelector("#device-period"),
  playbackSafety: document.querySelector("#playback-safety"),
  saveAudioSettings: document.querySelector("#save-audio-settings"),
  audioApplyState: document.querySelector("#audio-apply-state"),
  audioSampleRate: document.querySelector("#audio-sample-rate"),
  audioRequestedPeriod: document.querySelector("#audio-requested-period"),
  audioEffectivePeriod: document.querySelector("#audio-effective-period"),
  audioSafetyDuration: document.querySelector("#audio-safety-duration"),
};

let hasProject = false;
let lastSnapshot = null;
let audioSettings = null;
let audioInputDevices = [];
let audioRecording = false;
let projectTransportState = "stopped";
let transportPositionTick = 0;
let transportPositionPollPending = false;
let transportLoopErrorReported = false;
let metronomeEnabled = false;
let loopRange = null;
let pendingLoopStartTick = null;
let selectedTrackIds = new Set();
let trackGroupDraft = "";
let pendingAudioPath = null;
let editCursorTick = 0;
let editCursorProjectId = null;
let selectedClipId = null;
const waveformCache = new Map();
let previewContext = null;
let currentPreview = null;
const UI_ZOOM_STORAGE_KEY = "estudio-daw.ui-zoom.v1";
const UI_ZOOM_MIN = 0.8;
const UI_ZOOM_MAX = 1.5;
const UI_ZOOM_STEP = 0.1;
let uiZoom = 1;

/** @typedef {{ id: string, label: string, menu: string, shortcut?: string, target?: string, contexts?: string[], requiresProject?: boolean }} UiAction */
/** @type {UiAction[]} */
const UI_ACTIONS = [
  { id: "project.new", label: "Nuevo proyecto", menu: "Proyecto", target: "newProject" },
  { id: "project.open", label: "Abrir proyecto…", menu: "Proyecto", target: "open" },
  { id: "project.demo", label: "Cargar Demo MIDI", menu: "Proyecto", target: "demoProject" },
  { id: "project.importAudio", label: "Importar audio…", menu: "Proyecto", target: "importAudio", requiresProject: true },
  { id: "project.save", label: "Guardar", menu: "Proyecto", shortcut: "Ctrl+S", target: "save", requiresProject: true },
  { id: "project.saveAs", label: "Guardar como…", menu: "Proyecto", shortcut: "Ctrl+Mayús+S", target: "saveAs", requiresProject: true },
  { id: "edit.undo", label: "Deshacer", menu: "Edición", shortcut: "Ctrl+Z", target: "undo", requiresProject: true },
  { id: "edit.redo", label: "Rehacer", menu: "Edición", shortcut: "Ctrl+Mayús+Z", target: "redo", requiresProject: true },
  { id: "track.addMidi", label: "Añadir pista MIDI", menu: "Crear", target: "addMidiTrack", requiresProject: true },
  { id: "track.addAudio", label: "Añadir pista de audio", menu: "Crear", target: "addAudioTrack", requiresProject: true },
  { id: "track.addBus", label: "Añadir bus", menu: "Crear", target: "addBusTrack", requiresProject: true },
  { id: "scene.add", label: "Añadir escena", menu: "Sesión", requiresProject: true, handler: () => runCommand("Escena añadida", () => platform.addScene()) },
  { id: "view.arrangement", label: "Arreglo", menu: "Vista", shortcut: "Ctrl+1", target: "showArrangement" },
  { id: "view.session", label: "Sesión", menu: "Vista", shortcut: "Ctrl+2", target: "showSession" },
  { id: "view.mixer", label: "Mezclador", menu: "Vista", shortcut: "Ctrl+3", target: "showMixer" },
  { id: "view.audioSettings", label: "Preferencias de audio…", menu: "Vista", target: "railSettings" },
  { id: "transport.play", label: "Reproducir", menu: "Transporte", target: "play", requiresProject: true },
  { id: "transport.pause", label: "Pausar", menu: "Transporte", target: "pause", requiresProject: true },
  { id: "transport.stop", label: "Detener", menu: "Transporte", target: "stop", requiresProject: true },
  { id: "transport.record", label: "Grabar", menu: "Transporte", target: "record", requiresProject: true },
  { id: "transport.panic", label: "Apagar notas MIDI", menu: "Transporte", target: "panic", requiresProject: true },
  { id: "transport.metronome", label: "Metrónomo", menu: "Transporte", target: "metronome", requiresProject: true },
  { id: "transport.loopA", label: "Fijar inicio del rango A", menu: "Transporte", target: "loopPointA", requiresProject: true },
  { id: "transport.loopB", label: "Fijar fin del rango B", menu: "Transporte", target: "loopPointB", requiresProject: true },
  { id: "transport.loopClear", label: "Limpiar rango A/B", menu: "Transporte", target: "loopRangeClear", requiresProject: true },
  { id: "clip.select", label: "Seleccionar clip o región", menu: "Contexto", contexts: ["clip", "audio", "midi"] },
  { id: "midi.splitAtCursor", label: "Dividir en cursor", menu: "Contexto", contexts: ["midi"] },
  { id: "midi.duplicate", label: "Duplicar clip MIDI", menu: "Contexto", contexts: ["midi"] },
  { id: "midi.quantize", label: "Cuantizar clip MIDI a rejilla actual", menu: "Contexto", contexts: ["midi"] },
  { id: "audio.preview", label: "Preescuchar región", menu: "Contexto", contexts: ["audio"] },
  { id: "audio.remove", label: "Quitar región", menu: "Contexto", contexts: ["audio"] },
  { id: "track.moveUp", label: "Mover pista antes", menu: "Contexto", contexts: ["track"] },
  { id: "track.moveDown", label: "Mover pista después", menu: "Contexto", contexts: ["track"] },
  { id: "track.duplicate", label: "Duplicar pista", menu: "Contexto", contexts: ["track"] },
  { id: "track.remove", label: "Quitar pista", menu: "Contexto", contexts: ["track"] },
];

function actionTarget(action) {
  return action.target ? elements[action.target] : null;
}

function executeUiAction(action, context = null) {
  if (typeof action.handler === "function") {
    action.handler(context);
  } else if (action.id === "clip.select") {
    context?.clip?.focus();
    context?.clip?.click();
  } else if (action.id === "audio.preview") {
    context?.clip?.querySelector(".audio-preview-button")?.click();
  } else if (action.id === "audio.remove") {
    context?.clip?.querySelector(".audio-region-remove")?.click();
  } else if (action.id === "midi.quantize") {
    const clip = lastSnapshot?.midiClips.find((item) => item.id === context?.clip?.dataset.clipId);
    if (clip) {
      const grid = elements.gridSnap.value;
      const beatsPerGrid = grid === "bar" ? lastSnapshot.beatsPerBar : Number(grid);
      if (beatsPerGrid > 0) {
        const gridTicks = Math.max(1, Math.round(clip.ppq * beatsPerGrid));
        void runCommand("Clip MIDI cuantizado", () => platform.quantizeMidiClip(clip.id, gridTicks));
      }
    }
  } else if (action.id === "midi.duplicate") {
    const clipId = context?.clip?.dataset.clipId;
    if (clipId) void runCommand("Clip MIDI duplicado", () => platform.duplicateMidiClip(clipId));
  } else if (action.id === "midi.splitAtCursor") {
    const clip = lastSnapshot?.midiClips.find((item) => item.id === context?.clip?.dataset.clipId);
    if (clip) {
      const absoluteTick = Math.round(editCursorTick * clip.ppq / 480);
      const splitTick = absoluteTick - clip.startTick;
      void runCommand("Clip MIDI dividido", () => platform.splitMidiClip(clip.id, splitTick));
    }
  } else if (action.id === "track.moveUp") {
    context?.track?.querySelector('[data-track-order="up"]:not(:disabled)')?.click();
  } else if (action.id === "track.moveDown") {
    context?.track?.querySelector('[data-track-order="down"]:not(:disabled)')?.click();
  } else if (action.id === "track.duplicate") {
    const trackId = context?.track?.dataset.trackId;
    if (trackId) void runCommand("Pista duplicada", () => platform.duplicateTrack(trackId));
  } else if (action.id === "track.remove") {
    context?.track?.querySelector(".track-remove-button")?.click();
  } else {
    const target = actionTarget(action);
    if (target && !target.disabled) target.click();
  }
  closeContextMenu();
}

function renderApplicationMenu() {
  if (!elements.applicationMenu) return;
  const groups = [...new Set(UI_ACTIONS.filter((action) => action.target || action.handler).map((action) => action.menu))];
  elements.applicationMenu.replaceChildren();
  for (const group of groups) {
    const wrapper = document.createElement("div");
    wrapper.className = "application-menu-group";
    const toggle = document.createElement("button");
    toggle.type = "button";
    toggle.className = "application-menu-toggle";
    toggle.textContent = group;
    toggle.setAttribute("aria-haspopup", "menu");
    toggle.setAttribute("aria-expanded", "false");
    const popup = document.createElement("div");
    popup.className = "application-menu-popup";
    popup.setAttribute("role", "menu");
    for (const action of UI_ACTIONS.filter((item) => item.menu === group && (item.target || item.handler))) {
      const item = document.createElement("button");
      item.type = "button";
      item.className = "application-menu-item";
      item.setAttribute("role", "menuitem");
      item.dataset.actionId = action.id;
      item.setAttribute("aria-label", action.shortcut ? `${action.label} (${action.shortcut})` : action.label);
      const label = document.createElement("span");
      label.textContent = action.label;
      item.append(label);
      if (action.shortcut) {
        const shortcut = document.createElement("kbd");
        shortcut.textContent = action.shortcut;
        item.append(shortcut);
      }
      item.addEventListener("click", () => {
        executeUiAction(action);
        wrapper.classList.remove("is-open");
        toggle.setAttribute("aria-expanded", "false");
      });
      popup.append(item);
    }
    toggle.addEventListener("click", () => {
      refreshActionAvailability();
      const opening = !wrapper.classList.contains("is-open");
      elements.applicationMenu.querySelectorAll(".application-menu-group.is-open").forEach((node) => {
        node.classList.remove("is-open");
        node.querySelector(".application-menu-toggle")?.setAttribute("aria-expanded", "false");
      });
      wrapper.classList.toggle("is-open", opening);
      toggle.setAttribute("aria-expanded", String(opening));
      if (opening) popup.querySelector("button:not(:disabled)")?.focus();
    });
    wrapper.append(toggle, popup);
    elements.applicationMenu.append(wrapper);
  }
  refreshActionAvailability();
}

function refreshActionAvailability() {
  for (const action of UI_ACTIONS) {
    const target = actionTarget(action);
    const item = elements.applicationMenu?.querySelector(`[data-action-id="${action.id}"]`);
    if (item) item.disabled = (target ? target.disabled : typeof action.handler !== "function") || (action.requiresProject && !hasProject);
  }
}

function closeContextMenu() {
  if (!elements.contextMenu) return;
  elements.contextMenu.hidden = true;
  elements.contextMenu.replaceChildren();
}

function showContextMenu(event) {
  const clip = event.target.closest(".audio-clip, .midi-clip");
  const track = event.target.closest("[data-track-id]");
  const context = { clip, track };
  const kind = clip?.classList.contains("audio-clip") ? "audio" : clip?.classList.contains("midi-clip") ? "midi" : track ? "track" : null;
  if (!kind) return;
  const actions = UI_ACTIONS.filter((action) => action.menu === "Contexto" && action.contexts.includes(kind)).filter((action) => {
    if (action.id === "midi.splitAtCursor") {
      const midiClip = lastSnapshot?.midiClips.find((item) => item.id === clip?.dataset.clipId);
      if (!midiClip) return false;
      const absoluteTick = Math.round(editCursorTick * midiClip.ppq / 480);
      const relativeTick = absoluteTick - midiClip.startTick;
      return relativeTick > 0 && relativeTick < midiClip.durationTicks;
    }
    if (action.id === "audio.preview") return Boolean(clip?.querySelector(".audio-preview-button:not(:disabled)"));
    if (action.id === "audio.remove") return Boolean(clip?.querySelector(".audio-region-remove:not(:disabled)"));
    if (action.id === "midi.quantize") return Boolean(clip?.dataset.ppq && elements.gridSnap.value !== "0");
    if (action.id === "track.moveUp") return Boolean(track?.querySelector('[data-track-order="up"]:not(:disabled)'));
    if (action.id === "track.moveDown") return Boolean(track?.querySelector('[data-track-order="down"]:not(:disabled)'));
    if (action.id === "track.duplicate") return Boolean(track?.querySelector(".track-remove-button"));
    if (action.id === "track.remove") return Boolean(track?.querySelector(".track-remove-button:not(:disabled)"));
    return true;
  });
  if (!actions.length) return;
  event.preventDefault();
  elements.contextMenu.replaceChildren();
  for (const action of actions) {
    const item = document.createElement("button");
    item.type = "button";
    item.setAttribute("role", "menuitem");
    item.className = "context-menu-item";
    item.textContent = action.label;
    item.setAttribute("aria-label", action.label);
    item.addEventListener("click", () => executeUiAction(action, context));
    elements.contextMenu.append(item);
  }
  elements.contextMenu.hidden = false;
  const bounds = elements.contextMenu.getBoundingClientRect();
  elements.contextMenu.style.left = `${Math.max(6, Math.min(event.clientX, innerWidth - bounds.width - 6))}px`;
  elements.contextMenu.style.top = `${Math.max(6, Math.min(event.clientY, innerHeight - bounds.height - 6))}px`;
  elements.contextMenu.querySelector("button")?.focus();
}

function selectedAudioProfile() {
  return audioSettings?.[elements.audioProfile.value];
}

function renderAudioProfile(view = null) {
  if (!audioSettings) return;
  const profile = selectedAudioProfile();
  elements.devicePeriod.value = profile.devicePeriodFrames;
  elements.playbackSafety.value = profile.playbackSafetyFrames;
  if (view) {
    const rate = view.sampleRateHz;
    elements.audioSampleRate.textContent = `${rate.toLocaleString()} Hz`;
    elements.audioRequestedPeriod.textContent = `${view.requestedPeriodFrames} frames (${view.requestedPeriodMs.toFixed(2)} ms)`;
    elements.audioEffectivePeriod.textContent = view.effectivePeriodFrames == null
      ? "No disponible"
      : `${view.effectivePeriodFrames} frames (${(view.effectivePeriodFrames * 1000 / rate).toFixed(2)} ms)`;
    elements.audioSafetyDuration.textContent = `${view.playbackSafetyFrames} frames (${view.playbackSafetyMs.toFixed(2)} ms)`;
    elements.audioApplyState.textContent = view.applyState === "nextStream" ? "Pendiente · próximo inicio" : view.applyState;
  } else {
    const rate = 48_000;
    const requested = Number(profile.devicePeriodFrames);
    const safety = Number(profile.playbackSafetyFrames);
    elements.audioSampleRate.textContent = `${rate.toLocaleString()} Hz`;
    elements.audioRequestedPeriod.textContent = `${requested} frames (${(requested * 1000 / rate).toFixed(2)} ms)`;
    elements.audioEffectivePeriod.textContent = "No disponible";
    elements.audioSafetyDuration.textContent = `${safety} frames (${(safety * 1000 / rate).toFixed(2)} ms)`;
  }
}

async function loadAudioSettings() {
  try {
    const [devices, inputs, view] = await Promise.all([
      platform.audioOutputDevices().catch((error) => {
        setNotice("No se pudo consultar PipeWire", String(error));
        return [];
      }),
      platform.audioInputDevices().catch((error) => {
        setNotice("No se pudo consultar las entradas PipeWire", String(error));
        return [];
      }),
      platform.audioRuntimeSettings(),
    ]);
    audioInputDevices = inputs;
    audioSettings = view.settings;
    elements.audioProfile.value = audioSettings.activeProfile;
    const options = [new Option("Automática (AudioBox si está disponible)", "pipewire:default")];
    for (const device of devices) {
      options.push(new Option(device.description ? `${device.description} · ${device.name}` : device.name, device.key));
    }
    if (!options.some((option) => option.value === audioSettings.backendDeviceKey)) {
      options.push(new Option(`No disponible · ${audioSettings.backendDeviceKey}`, audioSettings.backendDeviceKey));
    }
    elements.audioOutputDevice.replaceChildren(...options);
    elements.audioOutputDevice.value = audioSettings.backendDeviceKey;
    renderAudioProfile(view);
  } catch (error) {
    elements.audioApplyState.textContent = "No se pudo cargar";
    setNotice("Configuración de audio no disponible", String(error));
  }
}

function setNotice(title, text) {
  elements.noticeTitle.textContent = title;
  elements.noticeText.textContent = text;
}

function updateUiZoomControls() {
  const percent = Math.round(uiZoom * 100);
  elements.zoomLevel.textContent = `${percent}%`;
  elements.zoomIn.disabled = uiZoom >= UI_ZOOM_MAX;
  elements.zoomOut.disabled = uiZoom <= UI_ZOOM_MIN;
  elements.zoomReset.title = `Restablecer interfaz (Ctrl+0); actual ${percent}%`;
}

async function setUiZoom(scale, persist = true) {
  const clamped = Math.min(UI_ZOOM_MAX, Math.max(UI_ZOOM_MIN, scale));
  const next = Math.round(clamped * 100) / 100;
  try {
    await platform.setUiZoom(next);
  } catch (error) {
    setNotice("No se pudo cambiar el tamaño", String(error));
    return;
  }
  uiZoom = next;
  if (persist) {
    try {
      localStorage.setItem(UI_ZOOM_STORAGE_KEY, String(next));
    } catch {
      // El zoom sigue disponible en la sesión aunque el WebView no guarde preferencias.
    }
  }
  updateUiZoomControls();
}

function initializeUiZoom() {
  let stored = 1;
  try {
    stored = Number(localStorage.getItem(UI_ZOOM_STORAGE_KEY));
  } catch {
    stored = 1;
  }
  const initial = Number.isFinite(stored) && stored > 0 ? stored : 1;
  uiZoom = 1;
  updateUiZoomControls();
  if (initial !== 1) void setUiZoom(initial, false);
}

function handleUiZoomShortcut(event) {
  if (!event.ctrlKey || event.altKey || event.metaKey) return;
  if (event.target instanceof Element && event.target.closest("input, textarea, select, [contenteditable='true']")) return;
  if (["+", "="].includes(event.key) || ["NumpadAdd"].includes(event.code)) {
    event.preventDefault();
    void setUiZoom(uiZoom + UI_ZOOM_STEP);
  } else if (["-", "_"].includes(event.key) || event.code === "NumpadSubtract") {
    event.preventDefault();
    void setUiZoom(uiZoom - UI_ZOOM_STEP);
  } else if (event.key === "0" || event.code === "Numpad0") {
    event.preventDefault();
    void setUiZoom(1);
  }
}

function handleWorkstationShortcut(event) {
  if (!event.ctrlKey || event.altKey || event.metaKey || event.repeat) return;
  if (event.target instanceof Element && event.target.closest("input, textarea, select, [contenteditable='true']")) return;
  const key = event.key.toLowerCase();
  const shortcut = key === "s" && event.shiftKey ? "Ctrl+Mayús+S"
    : key === "s" ? "Ctrl+S"
      : key === "z" && event.shiftKey ? "Ctrl+Mayús+Z"
        : key === "z" ? "Ctrl+Z"
          : ["1", "2", "3"].includes(key) && !event.shiftKey ? `Ctrl+${key}` : null;
  const action = UI_ACTIONS.find((item) => item.shortcut === shortcut);
  const button = action && actionTarget(action);
  if (!button) return;
  if (button.disabled) return;
  event.preventDefault();
  executeUiAction(action);
}

async function whileBusy(buttons, operation) {
  const previous = buttons.map((button) => button.disabled);
  for (const button of buttons) {
    button.disabled = true;
    button.setAttribute("aria-busy", "true");
  }
  try {
    return await operation();
  } finally {
    buttons.forEach((button, index) => {
      button.disabled = previous[index];
      button.removeAttribute("aria-busy");
    });
  }
}

function setProjectEnabled(enabled) {
  hasProject = enabled;
  for (const button of [elements.save, elements.saveAs, elements.play, elements.record, elements.pause, elements.stop, elements.addMidiTrack, elements.addAudioTrack, elements.addBusTrack]) {
    button.disabled = !enabled;
    if (!enabled) button.title = `${button.getAttribute("aria-label") ?? "Acción"}: abre o crea un proyecto primero`;
  }
  for (const button of [elements.loopPointA, elements.loopPointB, elements.loopRangeClear]) {
    button.disabled = !enabled;
  }
  elements.metronome.disabled = !enabled;
  elements.panic.disabled = !enabled || !["playing", "paused"].includes(projectTransportState);
  for (const control of [elements.importAudio, elements.importTrack, elements.importBar, elements.importMode, elements.importChannels]) {
    control.disabled = !enabled;
  }
  elements.importPreview.disabled = !enabled || !pendingAudioPath;
  elements.importCommit.disabled = !enabled || !pendingAudioPath;
  refreshActionAvailability();
}

function selectSurface(surface) {
  const selected = {
    arrangement: elements.showArrangement,
    session: elements.showSession,
    mixer: elements.showMixer,
  };
  const rail = {
    arrangement: elements.railArrangement,
    session: elements.railSession,
  };
  elements.arrangementView.hidden = surface !== "arrangement";
  elements.sessionView.hidden = surface !== "session";
  elements.mixerView.hidden = surface !== "mixer";
  elements.arrangementLegend.hidden = surface !== "arrangement";
  for (const [name, button] of Object.entries(selected)) {
    const active = name === surface;
    button.classList.toggle("is-selected", active);
    button.setAttribute("aria-selected", String(active));
  }
  for (const [name, button] of Object.entries(rail)) {
    const active = name === surface;
    button.classList.toggle("is-active", active);
    if (active) button.setAttribute("aria-current", "page");
    else button.removeAttribute("aria-current");
  }
}

function trackChannelDescription(track) {
  const input = track.inputChannels == null ? "sin entrada asignada" : `${track.inputChannels} canales de entrada`;
  return `${input} · ${track.outputChannels} canales de salida`;
}

function trackSignalFlow(track, tracks) {
  if (track.virtualMaster || track.role === "master") {
    return "Suma MIDI/audio/metrónomo → ganancia/silencio Master → medidor Master → salida del plan";
  }
  const output = tracks.find((candidate) => candidate.id === track.outputTrackId);
  const destination = output?.name ?? "Master";
  if (track.role === "bus") {
    return `Suma de pistas enrutadas → ganancia/pan de bus → medidor → ${destination}`;
  }
  if (track.kind === "audio") {
    const sources = ["Regiones (ganancia/desvanecimientos)"];
    if (track.inputRoute) {
      sources.unshift(`Entrada física ${track.inputRoute.channels.map((channel) => channel + 1).join("+")}`);
    }
    return `${sources.join(" + ")} → ganancia/pan → medidor → ${destination}`;
  }
  return `Eventos MIDI → instrumento → ganancia/pan → medidor → ${destination}`;
}

function createTrackOutputControl(track, tracks) {
  if (track.virtualMaster || track.role === "master") return null;
  const targets = tracks.filter((candidate) => ["audio", "bus", "master"].includes(candidate.role));
  const hasPersistedMaster = targets.some((candidate) => candidate.role === "master");
  if (!hasPersistedMaster) {
    targets.push({ id: "", name: "Master", role: "master", virtualMaster: true });
  }
  const field = document.createElement("label");
  field.className = "mixer-output-select";
  const caption = document.createElement("span");
  caption.textContent = "Salida";
  const select = document.createElement("select");
  select.setAttribute("aria-label", `Salida de ${track.name}`);
  for (const target of targets) {
    if (target.id === track.id) continue;
    const option = document.createElement("option");
    option.value = target.virtualMaster ? "" : target.id;
    option.textContent = target.role === "master" ? "Master" : target.name;
    select.append(option);
  }
  select.value = track.outputTrackId ?? targets.find((target) => target.role === "master")?.id ?? "";
  select.addEventListener("change", () => updateTrackOutput(track, select.value));
  field.append(caption, select);
  return field;
}

function createTrackInputControl(track) {
  if (track.virtualMaster || track.role !== "audio") return null;
  const field = document.createElement("div");
  field.className = "mixer-output-select";
  const caption = document.createElement("span");
  caption.textContent = "Entrada física · próximo inicio";
  const select = document.createElement("select");
  select.setAttribute("aria-label", `Entrada física de ${track.name}`);
  select.disabled = projectTransportState !== "stopped";
  select.append(new Option("Sin entrada asignada", ""));
  for (const device of audioInputDevices) {
    select.append(new Option(device.description ? `${device.description} · ${device.name}` : device.name, device.key));
  }
  const existing = track.inputRoute?.deviceKey;
  if (existing && !audioInputDevices.some((device) => device.key === existing)) {
    select.append(new Option(`No disponible · ${existing}`, existing));
  }
  select.value = existing ?? "";
  const channelField = document.createElement("label");
  channelField.textContent = "Canales de entrada";
  channelField.className = "mixer-output-select";
  const channels = document.createElement("select");
  channels.setAttribute("aria-label", `Canales de entrada de ${track.name}`);
  channels.disabled = projectTransportState !== "stopped";
  channels.append(
    new Option("1 + 2 · estéreo", "0,1"),
    new Option("1 · mono", "0"),
    new Option("2 · mono", "1"),
  );
  channels.value = track.inputRoute?.channels?.join(",") ?? "0,1";
  const saveRoute = () => {
    const inputRoute = select.value
      ? { deviceKey: select.value, channels: channels.value.split(",").map(Number) }
      : null;
    runCommand("Entrada de pista actualizada", () => platform.setTrackInputRoute(track.id, inputRoute));
  };
  select.addEventListener("change", saveRoute);
  channels.addEventListener("change", saveRoute);
  channelField.append(channels);
  const arm = document.createElement("button");
  arm.type = "button";
  arm.className = `mixer-toggle${track.recordArmed ? " is-selected" : ""}`;
  arm.textContent = track.recordArmed ? "REC ARM" : "ARMAR REC";
  arm.setAttribute("aria-pressed", String(track.recordArmed));
  arm.setAttribute("aria-label", `${track.recordArmed ? "Desarmar" : "Armar"} grabación ${track.name}`);
  arm.title = existing
    ? "La entrada se grabará al pulsar ● en la barra de transporte"
    : "Asigna una entrada física antes de armar la pista";
  arm.disabled = !existing || projectTransportState !== "stopped";
  arm.addEventListener("click", () => runCommand(
    track.recordArmed ? "Pista desarmada" : "Pista armada para grabación",
    () => platform.setTrackRecordArm(track.id, !track.recordArmed),
  ));
  field.append(caption, select, channelField, arm);
  return field;
}

function createTrackMixerControls(track, compact = false) {
  if (track.virtualMaster) return null;
  const isMaster = track.role === "master";
  const controls = document.createElement("div");
  controls.className = compact ? "mixer-controls mixer-controls-compact" : "mixer-controls";
  const toggle = (label, property, value) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "mixer-toggle";
    button.textContent = label;
    button.title = `${label}: ${track.name}`;
    button.setAttribute("aria-label", `${label} ${track.name}`);
    button.setAttribute("aria-pressed", String(value));
    button.classList.toggle("is-selected", value);
    button.addEventListener("click", () => updateTrackMixer(track, { [property]: !value }, `${label} actualizado`));
    controls.append(button);
  };
  toggle("ACT", "active", track.active);
  toggle("M", "mute", track.mute);
  if (!isMaster) toggle("S", "solo", track.solo);

  const slider = (label, property, value, min, max, step, suffix = "") => {
    const field = document.createElement("label");
    field.className = "mixer-slider";
    const caption = document.createElement("span");
    caption.textContent = `${label} ${Number(value).toFixed(property === "pan" ? 2 : 1)}${suffix}`;
    const input = document.createElement("input");
    input.type = "range";
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(value);
    input.setAttribute("aria-label", `${label} de ${track.name}`);
    input.addEventListener("change", () => updateTrackMixer(track, { [property]: Number(input.value) }, `${label} actualizado`));
    field.append(caption, input);
    controls.append(field);
  };
  slider("Ganancia", "gainDb", track.gainDb, -60, 12, 0.5, " dB");
  if (!isMaster) slider("Pan", "pan", track.pan, -1, 1, 0.05);
  return controls;
}

function createTrackMeter(track) {
  const meter = document.createElement("div");
  meter.className = "track-meter";
  meter.dataset.trackId = track.role === "master" ? "__master__" : track.id;
  meter.setAttribute("role", "meter");
  meter.setAttribute("aria-valuemin", "0");
  meter.setAttribute("aria-valuemax", "100");
  meter.setAttribute("aria-valuenow", "0");
  meter.setAttribute("aria-label", `Nivel de ${track.name}`);
  const fill = document.createElement("span");
  fill.className = "track-meter-fill";
  meter.append(fill);
  return meter;
}

function syncTrackSelectionUi() {
  for (const checkbox of document.querySelectorAll("[data-track-select]")) {
    checkbox.checked = selectedTrackIds.has(checkbox.dataset.trackSelect);
    checkbox.closest(".track-row, .session-track-card, .mixer-channel")
      ?.classList.toggle("is-track-selected", checkbox.checked);
  }
  const count = document.querySelector(".track-group-selection-count");
  if (count) count.textContent = `${selectedTrackIds.size} seleccionadas`;
  const canApply = selectedTrackIds.size > 0;
  for (const button of document.querySelectorAll("[data-track-group-action]")) {
    button.disabled = !canApply || (button.dataset.trackGroupAction === "assign" && !trackGroupDraft.trim());
  }
}

function createTrackSelectionControl(track) {
  if (track.virtualMaster) return null;
  const label = document.createElement("label");
  label.className = "track-select";
  label.title = `Seleccionar pista ${track.name}`;
  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.dataset.trackSelect = track.id;
  checkbox.checked = selectedTrackIds.has(track.id);
  checkbox.setAttribute("aria-label", `Seleccionar pista ${track.name}`);
  checkbox.addEventListener("change", () => {
    if (checkbox.checked) selectedTrackIds.add(track.id);
    else selectedTrackIds.delete(track.id);
    syncTrackSelectionUi();
  });
  label.append(checkbox);
  return label;
}

function createTrackRemovalButton(track) {
  if (track.virtualMaster || track.role === "master") return null;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "track-remove-button";
  button.textContent = "×";
  button.title = `Quitar ${track.name}; se puede deshacer`;
  button.setAttribute("aria-label", `Quitar pista ${track.name}`);
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    removeTrackFromProject(track, button);
  });
  return button;
}

function createTrackOrderControls(track, tracks) {
  if (track.virtualMaster || track.role === "master") return null;
  const index = tracks.findIndex((candidate) => candidate.id === track.id);
  const masterIndex = tracks.findIndex((candidate) => candidate.role === "master");
  const controls = document.createElement("span");
  controls.className = "track-order-controls";
  for (const [offset, symbol, label] of [[-1, "↑", "antes"], [1, "↓", "después"]]) {
    const button = document.createElement("button");
    const destination = index + offset;
    const reordered = tracks.filter((candidate) => candidate.id !== track.id);
    reordered.splice(Math.max(0, Math.min(destination, reordered.length)), 0, track);
    const nextMasterIndex = reordered.findIndex((candidate) => candidate.role === "master");
    const crossesMaster = masterIndex >= 0 && (index < masterIndex) !== (destination < nextMasterIndex);
    button.type = "button";
    button.className = "track-order-button";
    button.dataset.trackOrder = offset < 0 ? "up" : "down";
    button.textContent = symbol;
    button.disabled = index < 0 || destination < 0 || destination >= tracks.length || crossesMaster;
    button.title = `Mover ${track.name} ${label}`;
    button.setAttribute("aria-label", button.title);
    button.addEventListener("click", () => runCommand(
      "Orden de pistas actualizado",
      () => platform.moveTrack(track.id, destination),
    ));
    controls.append(button);
  }
  return controls;
}

async function removeTrackFromProject(track, button) {
  await whileBusy([button], async () => {
    try {
      const snapshot = await platform.removeTrack(track.id);
      selectedTrackIds.delete(track.id);
      renderSnapshot(snapshot);
      setNotice(
        "Pista quitada",
        "Se quitaron la pista y sus clips del proyecto. Puedes deshacerlo; los archivos de medios no se borraron.",
      );
    } catch (error) {
      setNotice("No se pudo quitar la pista", String(error));
    }
  });
}

function createTrackGroupToolbar() {
  const toolbar = document.createElement("div");
  toolbar.className = "track-group-toolbar";
  const input = document.createElement("input");
  input.type = "text";
  input.maxLength = 64;
  input.placeholder = "Nombre del grupo";
  input.value = trackGroupDraft;
  input.setAttribute("aria-label", "Nombre del grupo de pistas");
  input.addEventListener("input", () => {
    trackGroupDraft = input.value;
    syncTrackSelectionUi();
  });
  const assign = document.createElement("button");
  assign.type = "button";
  assign.textContent = "Agrupar selección";
  assign.dataset.trackGroupAction = "assign";
  assign.addEventListener("click", () => updateSelectedTrackGroup(trackGroupDraft.trim() || null));
  const clear = document.createElement("button");
  clear.type = "button";
  clear.textContent = "Quitar del grupo";
  clear.dataset.trackGroupAction = "clear";
  clear.addEventListener("click", () => updateSelectedTrackGroup(null));
  const count = document.createElement("span");
  count.className = "track-group-selection-count";
  toolbar.append(input, assign, clear, count);
  return toolbar;
}

async function updateSelectedTrackGroup(groupName) {
  if (selectedTrackIds.size === 0) return;
  await runCommand(
    groupName ? "Grupo de pistas actualizado" : "Pistas desagrupadas",
    () => platform.setTracksGroup([...selectedTrackIds], groupName),
  );
}

function updateTrackMeters(meters) {
  for (const meter of document.querySelectorAll(".track-meter")) {
    const reading = meters[meter.dataset.trackId];
    const peak = Math.max(0, Number(reading?.peak) || 0);
    const rms = Math.max(0, Number(reading?.rms) || 0);
    const level = Math.min(100, peak * 100);
    meter.style.setProperty("--meter-level", `${level}%`);
    meter.setAttribute("aria-valuenow", level.toFixed(0));
    meter.title = `Pico ${(20 * Math.log10(Math.max(peak, 1e-6))).toFixed(1)} dBFS · RMS ${(20 * Math.log10(Math.max(rms, 1e-6))).toFixed(1)} dBFS`;
  }
}

function renderSessionSurface(snapshot) {
  const tracks = snapshot.tracks.filter((track) => !["master", "bus", "return"].includes(track.role));
  elements.sessionView.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "surface-heading";
  const title = document.createElement("strong");
  title.textContent = "SESSION";
  const hint = document.createElement("span");
  hint.textContent = "Matriz de escenas y clips · lanzamiento pendiente del planificador";
  const addScene = document.createElement("button");
  addScene.className = "button button-accent session-add-scene";
  addScene.type = "button";
  addScene.textContent = "+ Escena";
  addScene.title = "Añadir una escena vacía al proyecto";
  addScene.addEventListener("click", () => runCommand("Escena añadida", () => platform.addScene()));
  heading.append(title, hint, addScene);
  elements.sessionView.append(heading);
  if (tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "surface-empty";
    empty.textContent = "Añade una pista MIDI o de audio para crear casillas de Session.";
    elements.sessionView.append(empty);
    return;
  }
  const grid = document.createElement("div");
  grid.className = "session-matrix";
  grid.style.setProperty("--session-track-columns", String(tracks.length));
  const corner = document.createElement("div");
  corner.className = "session-matrix-corner";
  corner.textContent = "ESCENA";
  grid.append(corner);
  for (const track of tracks) {
    const header = document.createElement("div");
    header.className = "session-track-header";
    header.dataset.trackId = track.id;
    header.style.setProperty("--track-color", track.color);
    header.append(createTrackSelectionControl(track));
    const name = document.createElement("strong");
    name.textContent = track.name;
    const type = document.createElement("small");
    type.textContent = track.kind.toUpperCase();
    const meter = createTrackMeter(track);
    header.append(name, type);
    if (meter) header.append(meter);
    const controls = createTrackMixerControls(track, true);
    if (controls) header.append(controls);
    const removeButton = createTrackRemovalButton(track);
    const orderControls = createTrackOrderControls(track, snapshot.tracks);
    if (orderControls) header.append(orderControls);
    if (removeButton) header.append(removeButton);
    grid.append(header);
  }
  for (const [sceneIndex, scene] of (snapshot.scenes ?? []).entries()) {
    const sceneHeader = document.createElement("div");
    sceneHeader.className = "session-scene-header";
    const sceneName = document.createElement("input");
    sceneName.value = scene.name;
    sceneName.setAttribute("aria-label", `Nombre de escena ${scene.name}`);
    sceneName.title = "Renombrar escena";
    sceneName.addEventListener("change", () => {
      const name = sceneName.value.trim();
      if (name && name !== scene.name) {
        runCommand("Escena renombrada", () => platform.renameScene(scene.id, name));
      } else {
        sceneName.value = scene.name;
      }
    });
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "icon-button session-remove-scene";
    remove.textContent = "×";
    remove.title = `Quitar escena ${scene.name} y sus casillas`;
    remove.setAttribute("aria-label", remove.title);
    remove.addEventListener("click", () => runCommand("Escena quitada", () => platform.removeScene(scene.id)));
    const moveUp = document.createElement("button");
    moveUp.type = "button";
    moveUp.className = "icon-button session-order-button";
    moveUp.textContent = "↑";
    moveUp.disabled = sceneIndex === 0;
    moveUp.title = `Mover ${scene.name} antes`;
    moveUp.setAttribute("aria-label", moveUp.title);
    moveUp.addEventListener("click", () => runCommand("Orden de escenas actualizado", () => platform.moveScene(scene.id, sceneIndex - 1)));
    const moveDown = document.createElement("button");
    moveDown.type = "button";
    moveDown.className = "icon-button session-order-button";
    moveDown.textContent = "↓";
    moveDown.disabled = sceneIndex === snapshot.scenes.length - 1;
    moveDown.title = `Mover ${scene.name} después`;
    moveDown.setAttribute("aria-label", moveDown.title);
    moveDown.addEventListener("click", () => runCommand("Orden de escenas actualizado", () => platform.moveScene(scene.id, sceneIndex + 1)));
    const launchScene = document.createElement("button");
    launchScene.type = "button";
    launchScene.className = "session-launch session-launch-scene";
    launchScene.textContent = "▶";
    launchScene.disabled = true;
    launchScene.title = "El lanzamiento de escenas requiere conectar el planificador cuantizado de Session";
    launchScene.setAttribute("aria-label", launchScene.title);
    sceneHeader.append(launchScene, sceneName, moveUp, moveDown, remove);
    grid.append(sceneHeader);
    for (const track of tracks) {
      const cell = document.createElement("div");
      cell.className = "session-cell";
      const slot = (snapshot.clipSlots ?? []).find((item) => item.sceneId === scene.id && item.trackId === track.id);
      const select = document.createElement("select");
      select.setAttribute("aria-label", `Clip de ${track.name} en ${scene.name}`);
      select.title = "Asignar un clip existente a esta casilla";
      select.append(new Option("Casilla vacía", ""));
      for (const clip of snapshot.midiClips.filter((item) => item.trackId === track.id)) {
        select.append(new Option(`MIDI · ${clip.name}`, `midi:${clip.id}`));
      }
      for (const clip of (snapshot.audioClips ?? []).filter((item) => item.trackId === track.id)) {
        select.append(new Option(`Audio · ${clip.name}`, `audio:${clip.id}`));
      }
      select.value = slot?.clipKind && slot.clipId ? `${slot.clipKind}:${slot.clipId}` : "";
      select.addEventListener("change", () => {
        const [kind, id] = select.value ? select.value.split(":", 2) : [null, null];
        runCommand("Casilla de Session actualizada", () => platform.setClipSlot(scene.id, track.id, kind, id));
      });
      const launch = document.createElement("button");
      launch.type = "button";
      launch.className = `session-launch${slot?.clipId ? " has-clip" : ""}`;
      launch.textContent = slot?.clipId ? "▶" : "+";
      launch.disabled = Boolean(slot?.clipId);
      launch.title = slot?.clipId
        ? "El lanzamiento de clips requiere conectar el planificador cuantizado de Session"
        : "Asigna un clip existente a esta casilla";
      if (!slot?.clipId) {
        launch.addEventListener("click", () => select.focus());
      }
      cell.append(launch, select);
      grid.append(cell);
    }
  }
  if (!(snapshot.scenes ?? []).length) {
    const empty = document.createElement("div");
    empty.className = "surface-empty session-empty-state";
    empty.textContent = "Aún no hay escenas. Añade una escena y asigna clips existentes a las casillas.";
    grid.append(empty);
  }
  elements.sessionView.append(grid);
}

function renderMixerSurface(tracks) {
  elements.mixerView.replaceChildren();
  const heading = document.createElement("div");
  heading.className = "surface-heading";
  heading.innerHTML = "<strong>MEZCLADOR</strong><span>Controles de pista aplicados a la reproducción</span>";
  elements.mixerView.append(heading);
  if (tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "surface-empty";
    empty.textContent = "El proyecto todavía no tiene canales.";
    elements.mixerView.append(empty);
  }
  const channels = document.createElement("div");
  channels.className = "mixer-channel-list";
  const mixerTracks = tracks.some((track) => track.role === "master")
    ? tracks
    : [...tracks, {
      id: "__master__",
      name: "Master",
      kind: "audio",
      role: "master",
      color: "#d5a36f",
      outputChannels: 2,
      active: true,
      mute: false,
      solo: false,
      gainDb: 0,
      pan: 0,
      virtualMaster: true,
    }];
  for (const track of mixerTracks) {
    const channel = document.createElement("article");
    channel.className = "mixer-channel";
    channel.dataset.trackId = track.id;
    channel.style.setProperty("--track-color", track.color);
    const title = document.createElement("strong");
    title.textContent = track.name;
    const role = document.createElement("span");
    role.className = "mixer-role";
    role.textContent = track.role === "master"
      ? "MASTER"
      : track.role === "bus" ? "BUS" : track.kind.toUpperCase();
    const routing = document.createElement("small");
    routing.className = "mixer-flow";
    routing.textContent = trackSignalFlow(track, tracks);
    const mix = document.createElement("span");
    mix.className = "mixer-values";
    mix.textContent = track.virtualMaster
      ? "Salida estéreo combinada"
      : track.role === "master"
        ? `${Number(track.gainDb).toFixed(1)} dB${track.mute ? " · Silencio" : ""}${track.active ? "" : " · Inactiva"}`
        : `${Number(track.gainDb).toFixed(1)} dB · Pan ${Number(track.pan).toFixed(2)}${track.mute ? " · Silencio" : ""}${track.solo ? " · Solo" : ""}${track.active ? "" : " · Inactiva"}`;
    if (track.groupName) role.textContent += ` · ${track.groupName}`;
    const selection = createTrackSelectionControl(track);
    const removeButton = createTrackRemovalButton(track);
    const orderControls = createTrackOrderControls(track, tracks);
    const outputControl = createTrackOutputControl(track, tracks);
    const inputControl = createTrackInputControl(track);
    const channelHeading = document.createElement("div");
    channelHeading.className = "mixer-channel-heading";
    if (selection) channelHeading.append(selection);
    channelHeading.append(title);
    if (orderControls) channelHeading.append(orderControls);
    if (removeButton) channelHeading.append(removeButton);
    channel.append(channelHeading, role, routing, mix);
    if (inputControl) channel.append(inputControl);
    if (outputControl) channel.append(outputControl);
    const meter = createTrackMeter(track);
    if (meter) channel.append(meter);
    const controls = createTrackMixerControls(track);
    if (controls) channel.append(controls);
    channels.append(channel);
  }
  elements.mixerView.append(channels);
}

function updateTrackMixer(track, changes, title) {
  const mixer = {
    active: track.active,
    mute: track.mute,
    solo: track.solo,
    gainDb: Number(track.gainDb),
    pan: Number(track.pan),
    ...changes,
  };
  return runCommand(title, () => platform.setTrackMixer(track.id, mixer));
}

function updateTrackOutput(track, outputTrackId) {
  return runCommand("Salida actualizada", () => platform.setTrackOutput(
    track.id,
    outputTrackId || null,
  ));
}

function renderSnapshot(snapshot) {
  lastSnapshot = snapshot;
  setProjectEnabled(true);
  if (snapshot.projectId !== editCursorProjectId) {
    selectedTrackIds = new Set();
    selectedClipId = null;
    trackGroupDraft = "";
    editCursorTick = 0;
    transportPositionTick = 0;
    editCursorProjectId = snapshot.projectId;
  }
  const validTrackIds = new Set(snapshot.tracks.map((track) => track.id));
  selectedTrackIds = new Set([...selectedTrackIds].filter((trackId) => validTrackIds.has(trackId)));
  projectTransportState = snapshot.transportState;
  audioRecording = audioRecording && projectTransportState !== "stopped";
  elements.panic.disabled = !snapshot.audioEngineConnected || !["playing", "paused"].includes(projectTransportState);
  loopRange = snapshot.loopRange ?? null;
  for (const button of [elements.loopPointA, elements.loopPointB, elements.loopRangeClear]) {
    button.disabled = audioRecording;
    if (audioRecording) button.title = "Detén la grabación antes de cambiar el rango A/B";
  }
  if (!loopRange) pendingLoopStartTick = null;
  elements.loopRangeReadout.textContent = loopRange
    ? `A ${formatBarBeat(loopRange.startTick)} · B ${formatBarBeat(loopRange.endTick)}`
    : pendingLoopStartTick === null ? "Sin rango" : `A ${formatBarBeat(pendingLoopStartTick)} · fija B`;
  if (projectTransportState === "stopped") renderTransportPosition(0);
  elements.save.disabled = !snapshot.projectPath;
  elements.save.title = snapshot.projectPath ? "Guardar proyecto" : "Guarda como para elegir una ubicación";
  elements.path.textContent = snapshot.projectPath ?? "Proyecto sin ruta";
  elements.path.title = snapshot.projectPath ?? "";
  const projectLabel = snapshot.projectPath?.split(/[\\/]/).at(-1) ?? snapshot.projectId;
  elements.name.textContent = projectLabel;
  elements.browserName.textContent = projectLabel;
  elements.tempo.textContent = Number(snapshot.tempoBpm).toFixed(1);
  elements.transportTempo.textContent = Number(snapshot.tempoBpm).toFixed(1);
  elements.transport.textContent = snapshot.transportState.toUpperCase();
  document.querySelector(".transport-bar").dataset.state = snapshot.transportState.toLowerCase();
  elements.trackCount.textContent = snapshot.trackCount;
  elements.midiCount.textContent = snapshot.midiClipCount;
  elements.audioCount.textContent = snapshot.audioClipCount;
  elements.revision.textContent = `REV ${snapshot.projectRevision}`;
  elements.projectStatus.textContent = snapshot.projectPath ? "PROYECTO ABIERTO" : "PROYECTO SIN GUARDAR";
  const armedTracks = snapshot.tracks.filter((track) => track.recordArmed && track.inputRoute);
  elements.record.disabled = !snapshot.projectPath || projectTransportState !== "stopped" || armedTracks.length === 0 || Boolean(loopRange);
  elements.record.title = !snapshot.projectPath
    ? "Guarda el proyecto antes de grabar"
    : loopRange
      ? "Desactiva el rango A/B antes de grabar; las tomas por secciones aún no están disponibles"
      : armedTracks.length === 0
      ? "Asigna una entrada y arma una pista de audio"
      : "Grabar en las pistas armadas; detén el transporte para finalizar la toma";
  elements.record.setAttribute("aria-pressed", String(audioRecording));
  elements.record.classList.toggle("is-selected", audioRecording);
  elements.pause.disabled = audioRecording || !snapshot.audioEngineConnected;
  elements.engine.textContent = snapshot.audioEngineConnected
    ? "Core listo · motor de audio conectado"
    : "Core listo · motor de audio aún no conectado";
  elements.undo.disabled = !snapshot.canUndo;
  elements.redo.disabled = !snapshot.canRedo;
  if (!document.querySelector(".track-group-toolbar")) {
    document.querySelector(".editor-heading").append(createTrackGroupToolbar());
  }
  const groupInput = document.querySelector(".track-group-toolbar input");
  if (groupInput && groupInput.value !== trackGroupDraft) groupInput.value = trackGroupDraft;
  syncTrackSelectionUi();
  renderSessionSurface(snapshot);
  renderMixerSurface(snapshot.tracks);
  const previousTrack = elements.importTrack.value;
  elements.importTrack.replaceChildren();
  const audioTracks = snapshot.tracks.filter((track) => track.role === "audio");
  for (const track of audioTracks) {
    const option = document.createElement("option");
    option.value = track.id;
    option.textContent = track.name;
    elements.importTrack.append(option);
  }
  elements.importTrack.disabled = audioTracks.length === 0;
  elements.importAudio.disabled = false;
  if (audioTracks.some((track) => track.id === previousTrack)) elements.importTrack.value = previousTrack;
  elements.importCommit.disabled = !pendingAudioPath || audioTracks.length === 0;

  elements.tracks.replaceChildren();
  elements.lanes.replaceChildren();
  renderTimelineRuler(snapshot.beatsPerBar || 4);
  if (snapshot.tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "El proyecto todavía no tiene pistas.";
    elements.tracks.append(empty);
    const emptyLane = document.createElement("div");
    emptyLane.className = "empty-state timeline-empty";
    emptyLane.textContent = "Sin pistas en el arreglo";
    elements.lanes.append(emptyLane);
    renderClipInspector(snapshot);
    renderEditCursor();
    renderTransportPosition(transportPositionTick);
    renderProjectMedia(snapshot);
    return;
  }

  for (const [trackIndex, track] of snapshot.tracks.entries()) {
    const row = document.createElement("div");
    row.className = "track-row";
    row.dataset.trackId = track.id;
    row.style.setProperty("--track-color", track.color);
    const icon = document.createElement("span");
    icon.className = `track-icon ${track.kind}`;
    icon.textContent = track.role === "master" ? "M" : track.role === "bus" ? "B" : track.kind === "audio" ? "◖" : "♫";
    const label = document.createElement("span");
    label.textContent = track.name;
    const details = document.createElement("span");
    details.className = "track-meta";
    const mixState = [track.mute ? "MUTE" : null, track.solo ? "SOLO" : null, !track.active ? "OFF" : null].filter(Boolean).join(" · ");
    details.textContent = `${track.role === "master" ? "MASTER" : track.role === "bus" ? "BUS" : track.kind === "audio" ? "AUDIO" : "MIDI"}${track.kind === "midi" ? ` · ${track.noteCount} notas` : track.role === "audio" ? ` · ${track.outputChannels} ch` : ""}${track.groupName ? ` · GRUPO ${track.groupName}` : ""}${mixState ? ` · ${mixState}` : ""}`;
    const name = document.createElement("div");
    name.className = "track-name";
    name.append(createTrackSelectionControl(track), icon, label);
    const headingRow = document.createElement("div");
    headingRow.className = "track-row-heading";
    headingRow.append(name);
    const removeButton = createTrackRemovalButton(track);
    const orderControls = createTrackOrderControls(track, snapshot.tracks);
    if (orderControls) headingRow.append(orderControls);
    if (removeButton) headingRow.append(removeButton);
    row.append(headingRow, details);
    const meter = createTrackMeter(track);
    if (meter) row.append(meter);
    const mixerControls = createTrackMixerControls(track, true);
    if (mixerControls) row.append(mixerControls);
    elements.tracks.append(row);

    const lane = document.createElement("div");
    lane.className = "timeline-lane";
    lane.style.setProperty("--track-color", track.color);
    lane.addEventListener("click", (event) => {
      if (event.target.closest(".audio-clip, .midi-clip, button")) return;
      setEditCursorFromX(event.clientX, lane.getBoundingClientRect());
    });
    const clips = snapshot.midiClips.filter((clip) => clip.trackId === track.id);
    for (const clip of clips) {
      const block = document.createElement("div");
      block.className = "midi-clip";
      block.dataset.clipId = clip.id;
      block.dataset.ppq = String(clip.ppq);
      if (selectedClipId === clip.id) block.classList.add("is-inspected");
      block.style.setProperty("--clip-hue", String((trackIndex * 54 + 24) % 360));
      block.title = `${clip.name} · ${clip.noteCount} notas`;
      block.tabIndex = 0;
      block.setAttribute("aria-label", `Seleccionar clip MIDI ${clip.name}, ${clip.noteCount} notas. Arrastra para mover.`);
      bindClipSelection(block, clip.id, snapshot);
      const left = Math.max(0, Number(clip.startBeats) || 0);
      const width = Math.max(0.25, Number(clip.durationBeats) || 0.25);
      block.style.left = `${left / (snapshot.beatsPerBar * 16) * 100}%`;
      block.style.width = `${Math.min(width / (snapshot.beatsPerBar * 16) * 100, 100)}%`;
      bindMidiClipMovement(block, lane, clip, snapshot.beatsPerBar);
      const clipLabel = document.createElement("span");
      clipLabel.className = "clip-label";
      clipLabel.textContent = clip.name;
      block.append(clipLabel);
      const noteLayer = document.createElement("div");
      noteLayer.className = "clip-note-layer";
      for (const note of clip.notes) {
        const noteMark = document.createElement("span");
        noteMark.className = "clip-note";
        const noteLeft = Math.max(0, Number(note.startBeats) || 0);
        const noteWidth = Math.max(0.04, Number(note.durationBeats) || 0.04);
        noteMark.style.left = `${noteLeft / width * 100}%`;
        noteMark.style.width = `${Math.min(noteWidth / width * 100, 100)}%`;
        noteMark.style.bottom = `${4 + Math.max(0, Math.min(1, (note.key - 48) / 36)) * 30}px`;
        noteMark.style.opacity = String(0.5 + Math.max(0, Math.min(127, note.velocity)) / 254);
        noteLayer.append(noteMark);
      }
      block.append(noteLayer);
      lane.append(block);
    }
    const audioClips = (snapshot.audioClips ?? []).filter((clip) => clip.trackId === track.id);
    for (const clip of audioClips) {
      const block = document.createElement("div");
      block.className = "audio-clip";
      block.dataset.clipId = clip.id;
      if (selectedClipId === clip.id) block.classList.add("is-inspected");
      block.tabIndex = 0;
      block.setAttribute("aria-label", `Seleccionar región de audio ${clip.name}`);
      block.title = `${clip.name} · ${clip.sourceName ?? "fuente"} · ${clip.sampleRateHz} Hz · ${clip.channels} canales · ${clip.durationBeats.toFixed(2)} pulsos. Arrastra para mover; usa los bordes para recortar.`;
      bindClipSelection(block, clip.id, snapshot);
      const left = Math.max(0, Number(clip.startBeats) || 0);
      const width = Math.max(0.25, Number(clip.durationBeats) || 0.25);
      block.style.left = `${left / (snapshot.beatsPerBar * 16) * 100}%`;
      block.style.width = `${Math.min(width / (snapshot.beatsPerBar * 16) * 100, 100)}%`;
      const label = document.createElement("span");
      label.className = "clip-label";
      label.textContent = clip.name;
      const preview = document.createElement("button");
      preview.type = "button";
      preview.className = "audio-preview-button";
      preview.setAttribute("aria-label", `Preescuchar ${clip.name}`);
      preview.title = "Preescucha aislada · hasta 30 segundos";
      preview.textContent = "▶";
      preview.addEventListener("click", () => previewAudio(clip.sourceId, preview));
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "audio-region-remove";
      remove.setAttribute("aria-label", `Borrar región ${clip.name}`);
      remove.title = "Quitar región del proyecto; conservar archivo fuente";
      remove.textContent = "×";
      remove.addEventListener("click", async () => {
        await whileBusy([remove], async () => {
          try {
            const snapshot = await platform.editAudioRegion({ action: "remove", clipId: clip.id });
            renderSnapshot(snapshot);
            const playbackNote = projectTransportState === "playing"
              ? "El plan activo se reconstruyó; la fuente de audio se conservó."
              : projectTransportState === "paused"
                ? "La fuente se conservó; el plan actualizado se aplicará al reanudar."
                : "La fuente de audio se conservó.";
            setNotice("Región quitada", playbackNote);
          } catch (error) {
            setNotice("No se pudo quitar la región", String(error));
          }
        });
      });
      const leftHandle = makeAudioTrimHandle("left", clip.name);
      const rightHandle = makeAudioTrimHandle("right", clip.name);
      const wave = document.createElement("div");
      wave.className = "audio-waveform";
      block.append(label, remove, preview, leftHandle, rightHandle, wave);
      bindAudioRegionEditing(block, lane, clip, snapshot.beatsPerBar, snapshot.tempoBpm);
      lane.append(block);
      if (clip.sourceId) loadWaveform(clip.sourceId, clip.sourceDigest, wave);
    }
    if (clips.length === 0 && audioClips.length === 0) {
      const empty = document.createElement("span");
      empty.className = "lane-empty-label";
      empty.textContent = "Sin clips";
      lane.append(empty);
    }
    const cursor = document.createElement("span");
    cursor.className = "edit-cursor";
    cursor.setAttribute("aria-hidden", "true");
    lane.append(cursor);
    const playhead = document.createElement("span");
    playhead.className = "playhead";
    playhead.setAttribute("aria-hidden", "true");
    lane.append(playhead);
    elements.lanes.append(lane);
  }
  renderEditCursor();
  renderTransportPosition(transportPositionTick);
  renderClipInspector(snapshot);
  renderProjectMedia(snapshot);
  syncTrackSelectionUi();
  refreshActionAvailability();
}

function renderClipInspector(snapshot) {
  elements.clipInspector.replaceChildren();
  const midiClip = snapshot.midiClips.find((clip) => clip.id === selectedClipId);
  const audioClip = (snapshot.audioClips ?? []).find((clip) => clip.id === selectedClipId);
  const clip = audioClip ?? midiClip;
  if (!clip) {
    selectedClipId = null;
    const empty = document.createElement("span");
    empty.textContent = "Selecciona una región de audio o un clip MIDI en Arreglo.";
    elements.clipInspector.append(empty);
    return;
  }
  const track = snapshot.tracks.find((item) => item.id === clip.trackId);
  const name = document.createElement("strong");
  name.textContent = clip.name;
  const type = document.createElement("span");
  type.className = "clip-inspector-type";
  type.textContent = audioClip ? "AUDIO" : "MIDI";
  const trackName = document.createElement("span");
  trackName.textContent = `Pista: ${track?.name ?? "desconocida"}`;
  const position = document.createElement("span");
  position.textContent = `Inicio: ${formatBarBeat(Math.round((Number(clip.startBeats) || 0) * 960))}`;
  const length = document.createElement("span");
  length.textContent = audioClip
    ? `Duración: ${audioClip.durationBeats.toFixed(2)} pulsos · ${audioClip.sampleRateHz} Hz · ${audioClip.channels} canales`
    : `Duración: ${midiClip.durationBeats.toFixed(2)} pulsos · ${midiClip.noteCount} notas`;
  elements.clipInspector.append(type, name, trackName, position, length);
  if (audioClip?.sourceName) {
    const source = document.createElement("span");
    source.className = "clip-inspector-source";
    source.textContent = `Fuente: ${audioClip.sourceName}`;
    elements.clipInspector.append(source);
  }
  if (audioClip) {
    const positionInput = makeInspectorNumber("Inicio (pulsos)", audioClip.startBeats, 0, null, 0.25, async (beats) => {
      await runCommand("Posición de región actualizada", () => platform.editAudioRegion({
        action: "move",
        clipId: audioClip.id,
        startTick: Math.round(beats * 480),
      }));
    });
    const gainInput = makeInspectorNumber("Ganancia (dB)", audioClip.gainDb, -120, 24, 0.1, async (gainDb) => {
      await runCommand("Ganancia de región actualizada", () => platform.editAudioRegion({
        action: "gain",
        clipId: audioClip.id,
        gainDb,
      }));
    });
    const durationMs = audioClip.durationSamples / Math.max(1, audioClip.sampleRateHz) * 1000;
    const fadeInMs = audioClip.fadeInSamples / Math.max(1, audioClip.sampleRateHz) * 1000;
    const fadeOutMs = audioClip.fadeOutSamples / Math.max(1, audioClip.sampleRateHz) * 1000;
    const saveFades = async (edge, milliseconds) => {
      const boundedMs = Math.max(0, Math.min(milliseconds, durationMs - (edge === "in" ? fadeOutMs : fadeInMs)));
      await runCommand("Desvanecimiento de región actualizado", () => platform.editAudioRegion({
        action: "fades",
        clipId: audioClip.id,
        fadeInSamples: Math.round((edge === "in" ? boundedMs : fadeInMs) * audioClip.sampleRateHz / 1000),
        fadeOutSamples: Math.round((edge === "out" ? boundedMs : fadeOutMs) * audioClip.sampleRateHz / 1000),
      }));
    };
    elements.clipInspector.append(
      positionInput,
      gainInput,
      makeInspectorNumber("Entrada (ms)", fadeInMs, 0, Math.max(0, durationMs - fadeOutMs), 1, (value) => saveFades("in", value)),
      makeInspectorNumber("Salida (ms)", fadeOutMs, 0, Math.max(0, durationMs - fadeInMs), 1, (value) => saveFades("out", value)),
    );
  }
}

function makeInspectorNumber(caption, value, min, max, step, onChange) {
  const field = document.createElement("label");
  field.className = "clip-inspector-field";
  const name = document.createElement("span");
  name.textContent = caption;
  const input = document.createElement("input");
  input.type = "number";
  input.value = Number(value).toFixed(step < 1 ? 2 : 0);
  input.min = String(min);
  if (max !== null) input.max = String(max);
  input.step = String(step);
  input.addEventListener("change", () => {
    const parsed = Number(input.value);
    if (!Number.isFinite(parsed) || parsed < min || (max !== null && parsed > max)) {
      input.value = Number(value).toFixed(step < 1 ? 2 : 0);
      return;
    }
    onChange(parsed);
  });
  field.append(name, input);
  return field;
}

function renderProjectMedia(snapshot) {
  elements.browserMediaList.replaceChildren();
  const clips = snapshot.audioClips ?? [];
  const query = elements.browserMediaSearch.value.trim().toLocaleLowerCase();
  const channelFilter = elements.browserMediaChannels.value;
  const matching = clips.filter((clip) => {
    const trackName = snapshot.tracks.find((track) => track.id === clip.trackId)?.name ?? "";
    const searchable = `${clip.name} ${clip.sourceName ?? ""} ${trackName} ${clip.sampleRateHz} ${clip.channels}`.toLocaleLowerCase();
    const channelMatch = channelFilter === "all"
      || (channelFilter === "multi" ? clip.channels > 2 : clip.channels === Number(channelFilter));
    return channelMatch && (!query || searchable.includes(query));
  });
  elements.browserMediaSearch.disabled = clips.length === 0;
  elements.browserMediaChannels.disabled = clips.length === 0;
  if (!matching.length) {
    const empty = document.createElement("span");
    empty.className = "browser-media-empty";
    empty.textContent = clips.length ? "No hay audio que coincida con la búsqueda." : "Los medios importados aparecerán aquí.";
    elements.browserMediaList.append(empty);
    return;
  }
  for (const clip of matching) {
    const trackName = snapshot.tracks.find((track) => track.id === clip.trackId)?.name ?? "Pista desconocida";
    const item = document.createElement("div");
    item.className = "browser-media-item";
    const selectButton = document.createElement("button");
    selectButton.type = "button";
    selectButton.className = "browser-media-select";
    selectButton.setAttribute("aria-label", `Mostrar ${clip.name} en Arrangement`);
    const name = document.createElement("strong");
    name.textContent = clip.name;
    const details = document.createElement("small");
    details.textContent = `${clip.sourceName ?? "Fuente"} · ${trackName} · ${clip.sampleRateHz} Hz · ${clip.channels} ch`;
    const preview = document.createElement("button");
    preview.type = "button";
    preview.className = "audio-preview-button";
    preview.textContent = "▶";
    preview.title = clip.sourceId ? "Preescucha aislada · hasta 30 segundos" : "La región no tiene una fuente preescuchable";
    preview.setAttribute("aria-label", `Preescuchar ${clip.name}`);
    preview.disabled = !clip.sourceId || projectTransportState === "playing";
    preview.addEventListener("click", (event) => {
      event.stopPropagation();
      previewAudio(clip.sourceId, preview);
    });
    const select = () => {
      selectedClipId = clip.id;
      selectSurface("arrangement");
      renderClipInspector(snapshot);
      const target = [...elements.lanes.querySelectorAll("[data-clip-id]")]
        .find((block) => block.dataset.clipId === clip.id);
      if (!target) return;
      for (const active of elements.lanes.querySelectorAll(".is-inspected")) active.classList.remove("is-inspected");
      target.classList.add("is-inspected");
      target.scrollIntoView({ block: "nearest", inline: "nearest" });
    };
    selectButton.addEventListener("click", select);
    selectButton.append(name, details);
    item.append(selectButton, preview);
    elements.browserMediaList.append(item);
  }
}

function bindClipSelection(block, clipId, snapshot) {
  const select = () => {
    selectedClipId = clipId;
    for (const selected of elements.lanes.querySelectorAll(".is-inspected")) {
      selected.classList.remove("is-inspected");
    }
    renderClipInspector(snapshot);
    block.classList.add("is-inspected");
  };
  block.addEventListener("click", (event) => {
    if (event.target.closest("button")) return;
    select();
  });
  block.addEventListener("pointerdown", (event) => {
    if (event.target.closest("button")) return;
    select();
  });
  block.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    select();
  });
}

function makeAudioTrimHandle(edge, name) {
  const handle = document.createElement("button");
  handle.type = "button";
  handle.className = `audio-region-handle audio-region-handle-${edge}`;
  handle.dataset.edge = edge;
  handle.setAttribute("aria-label", `Recortar ${edge === "left" ? "inicio" : "final"} de ${name}`);
  handle.title = "Arrastra hacia dentro para recortar sin modificar el archivo";
  return handle;
}

function bindMidiClipMovement(block, lane, clip, beatsPerBar) {
  const timelineBeats = Math.max(1, Number(beatsPerBar) || 4) * 16;
  const originalLeft = Math.max(0, Number(clip.startBeats) || 0);
  const duration = Math.max(0.25, Number(clip.durationBeats) || 0.25);
  const ppq = Math.max(1, Number(clip.ppq) || 480);
  let gesture = null;

  block.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || event.target.closest("button")) return;
    const bounds = lane.getBoundingClientRect();
    if (bounds.width <= 0) return;
    gesture = { pointerId: event.pointerId, startX: event.clientX, laneWidth: bounds.width };
    block.setPointerCapture(event.pointerId);
    block.classList.add("is-editing");
    event.preventDefault();
  });

  block.addEventListener("pointermove", (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const deltaBeats = (event.clientX - gesture.startX) / gesture.laneWidth * timelineBeats;
    const left = Math.max(0, Math.min(timelineBeats - duration, originalLeft + deltaBeats));
    block.style.left = `${left / timelineBeats * 100}%`;
  });

  const finish = async (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const current = gesture;
    gesture = null;
    block.classList.remove("is-editing");
    const restore = () => { block.style.left = `${originalLeft / timelineBeats * 100}%`; };
    if (event.type === "pointercancel" || Math.abs(event.clientX - current.startX) < 2) {
      restore();
      return;
    }
    const rawBeats = Math.max(0, Math.min(timelineBeats - duration,
      originalLeft + (event.clientX - current.startX) / current.laneWidth * timelineBeats));
    const snapValue = elements.gridSnap.value;
    const snapBeats = snapValue === "bar"
      ? Math.max(1, Number(beatsPerBar) || 4)
      : Number(snapValue);
    const snappedBeats = snapBeats > 0 ? Math.round(rawBeats / snapBeats) * snapBeats : rawBeats;
    const startTick = Math.round(Math.max(0, Math.min(timelineBeats - duration, snappedBeats)) * ppq);
    if (startTick === Math.round(originalLeft * ppq)) {
      restore();
      return;
    }
    try {
      renderSnapshot(await platform.moveMidiClip(clip.id, startTick));
      setNotice("Clip MIDI movido", projectTransportState === "playing"
        ? "El plan activo se reconstruyó en un límite de bloque. Puedes deshacer el cambio desde el historial."
        : "La toma y sus eventos permanecen intactos; puedes deshacer el cambio desde el historial.");
    } catch (error) {
      try { renderSnapshot(await platform.projectSnapshot()); } catch { restore(); }
      setNotice("No se pudo mover el clip MIDI", String(error));
    }
  };
  block.addEventListener("pointerup", finish);
  block.addEventListener("pointercancel", finish);
}

function bindAudioRegionEditing(block, lane, clip, beatsPerBar, tempoBpm) {
  const timelineBeats = Math.max(1, Number(beatsPerBar) || 4) * 16;
  const originalLeft = Math.max(0, Number(clip.startBeats) || 0);
  const originalWidth = Math.max(0.25, Number(clip.durationBeats) || 0.25);
  let gesture = null;

  block.addEventListener("pointerdown", (event) => {
    const handle = event.target.closest(".audio-region-handle");
    if (event.target.closest("button") && !handle) return;
    if (event.button !== 0) return;
    const bounds = lane.getBoundingClientRect();
    if (bounds.width <= 0) return;
    gesture = {
      pointerId: event.pointerId,
      edge: handle?.dataset.edge ?? "move",
      startX: event.clientX,
      laneWidth: bounds.width,
    };
    block.setPointerCapture(event.pointerId);
    block.classList.add("is-editing");
    event.preventDefault();
  });

  block.addEventListener("pointermove", (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const delta = event.clientX - gesture.startX;
    const deltaPercent = delta / gesture.laneWidth * 100;
    if (gesture.edge === "move") {
      const originalLeftPercent = originalLeft / timelineBeats * 100;
      const widthPercent = originalWidth / timelineBeats * 100;
      block.style.left = `${Math.max(0, Math.min(100 - widthPercent, originalLeftPercent + deltaPercent))}%`;
    } else if (gesture.edge === "left") {
      const trim = Math.max(0, Math.min(deltaPercent, originalWidth / timelineBeats * 100 - 0.3));
      block.style.left = `${originalLeft / timelineBeats * 100 + trim}%`;
      block.style.width = `${originalWidth / timelineBeats * 100 - trim}%`;
    } else {
      const trim = Math.min(0, Math.max(deltaPercent, -originalWidth / timelineBeats * 100 + 0.3));
      block.style.width = `${originalWidth / timelineBeats * 100 + trim}%`;
    }
  });

  const finish = async (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const current = gesture;
    gesture = null;
    block.classList.remove("is-editing");
    if (event.type === "pointercancel") {
      block.style.left = `${originalLeft / timelineBeats * 100}%`;
      block.style.width = `${originalWidth / timelineBeats * 100}%`;
      return;
    }
    const delta = event.clientX - current.startX;
    if (Math.abs(delta) < 2) {
      block.style.left = `${originalLeft / timelineBeats * 100}%`;
      block.style.width = `${originalWidth / timelineBeats * 100}%`;
      return;
    }
    const deltaBeats = delta / current.laneWidth * timelineBeats;
    const samplesPerBeat = clip.sampleRateHz * 60 / Math.max(1, Number(tempoBpm) || 120);
    const snapValue = elements.gridSnap.value;
    const snapBeats = snapValue === "bar"
      ? Math.max(1, Number(beatsPerBar) || 4)
      : Number(snapValue);
    let edit;
    if (current.edge === "move") {
      const proposedStart = originalLeft + deltaBeats;
      const snappedStart = snapBeats > 0 ? Math.round(proposedStart / snapBeats) * snapBeats : proposedStart;
      edit = {
        action: "move",
        clipId: clip.id,
        startTick: Math.round(Math.max(0, Math.min(timelineBeats - originalWidth, snappedStart)) * 480),
      };
    } else {
      const rawTrimBeats = current.edge === "left" ? Math.max(0, deltaBeats) : Math.max(0, -deltaBeats);
      const trimBeats = snapBeats > 0 ? Math.round(rawTrimBeats / snapBeats) * snapBeats : rawTrimBeats;
      if (trimBeats === 0) {
        block.style.left = `${originalLeft / timelineBeats * 100}%`;
        block.style.width = `${originalWidth / timelineBeats * 100}%`;
        return;
      }
      const trimSamples = Math.max(0, Math.round(trimBeats * samplesPerBeat));
      const boundedSamples = Math.min(trimSamples, Math.max(0, clip.durationSamples - 1));
      if (current.edge === "left") {
        const appliedBeats = boundedSamples / samplesPerBeat;
        edit = {
          action: "trim",
          clipId: clip.id,
          startTick: Math.round((originalLeft + appliedBeats) * 480),
          sourceStartSamples: clip.sourceStartSamples + boundedSamples,
          durationSamples: clip.durationSamples - boundedSamples,
        };
      } else {
        edit = {
          action: "trim",
          clipId: clip.id,
          sourceStartSamples: clip.sourceStartSamples,
          durationSamples: clip.durationSamples - boundedSamples,
        };
      }
    }
    try {
      const snapshot = await platform.editAudioRegion(edit);
      renderSnapshot(snapshot);
      const playbackNote = projectTransportState === "stopped"
        ? "La fuente permanece intacta; puedes deshacer el cambio desde el historial."
        : projectTransportState === "playing"
          ? "La fuente permanece intacta; el plan activo se reconstruyó en un límite de bloque."
          : "La fuente permanece intacta; el plan actualizado se aplicará al reanudar.";
      setNotice("Región actualizada", playbackNote);
    } catch (error) {
      try {
        renderSnapshot(await platform.projectSnapshot());
      } catch {
        // La vista actual queda como referencia si el refresco también falla.
      }
      setNotice("No se pudo editar la región", String(error));
    }
  };
  block.addEventListener("pointerup", finish);
  block.addEventListener("pointercancel", finish);
}

function stopPreview() {
  currentPreview?.stop();
  currentPreview = null;
}

async function previewAudio(sourceId, button) {
  if (!sourceId) return;
  if (projectTransportState === "playing") {
    setNotice("Preescucha no disponible", "Detén o pausa el transporte antes de escuchar un fragmento aislado.");
    return;
  }
  button.disabled = true;
  try {
    stopPreview();
    const encoded = await platform.audioPreview(sourceId);
    await playPreviewBytes(encoded);
    setNotice("Preescucha", "Fragmento aislado de hasta 30 segundos; no mueve el transporte del proyecto.");
  } catch (error) {
    setNotice("No se pudo preescuchar el audio", String(error));
  } finally {
    button.disabled = false;
  }
}

async function playPreviewBytes(encoded) {
  if (!previewContext) previewContext = new AudioContext();
  const binary = atob(encoded);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  const audioBuffer = await previewContext.decodeAudioData(bytes.buffer);
  await previewContext.resume();
  const source = previewContext.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(previewContext.destination);
  currentPreview = source;
  source.addEventListener("ended", () => {
    if (currentPreview === source) currentPreview = null;
  }, { once: true });
  source.start();
}

async function previewPendingAudio() {
  if (!pendingAudioPath) return;
  if (projectTransportState === "playing") {
    setNotice("Preescucha no disponible", "Detén o pausa el transporte antes de escuchar el archivo seleccionado.");
    return;
  }
  elements.importPreview.disabled = true;
  try {
    stopPreview();
    await playPreviewBytes(await platform.audioPreviewFile(pendingAudioPath));
    setNotice("Preescucha del archivo", "Fragmento Opus de hasta 30 segundos. El archivo aún no se ha importado.");
  } catch (error) {
    setNotice("No se pudo preescuchar el archivo", String(error));
  } finally {
    elements.importPreview.disabled = !pendingAudioPath;
  }
}

elements.importAudio.addEventListener("click", async () => {
  try {
    stopPreview();
    const path = await platform.selectAudioFile();
    if (!path) return;
    pendingAudioPath = null;
    elements.importPreview.disabled = true;
    elements.importCommit.disabled = true;
    const metadata = await platform.inspectAudioFile(path);
    pendingAudioPath = path;
    const channelOptions = [];
    if (metadata.channels <= 2) {
      channelOptions.push({ label: metadata.channels === 1 ? "Canal 1 · mono" : "Canales 1–2 · estéreo", value: Array.from({ length: metadata.channels }, (_, index) => index) });
    } else {
      for (let channel = 0; channel < metadata.channels; channel += 1) {
        channelOptions.push({ label: `Canal ${channel + 1} · mono`, value: [channel] });
        if (channel + 1 < metadata.channels) channelOptions.push({ label: `Canales ${channel + 1}–${channel + 2} · estéreo`, value: [channel, channel + 1] });
      }
    }
    elements.importChannels.replaceChildren(...channelOptions.map((option) => {
      const item = document.createElement("option");
      item.value = option.value.join(",");
      item.textContent = option.label;
      return item;
    }));
    elements.importChannels.disabled = false;
    const fileName = path.split(/[\\/]/).at(-1) ?? path;
    elements.importSelected.textContent = `${fileName} · ${metadata.sampleRateHz.toLocaleString()} Hz · ${metadata.channels} ch · ${metadata.durationSeconds.toFixed(2)} s`;
    elements.importPreview.disabled = false;
    elements.importCommit.disabled = !hasProject || !elements.importTrack.value;
    setNotice("Archivo listo para revisar", "Puedes escuchar un fragmento antes de decidir si lo importas.");
  } catch (error) {
    setNotice("No se pudo inspeccionar el archivo", String(error));
  }
});

elements.importPreview.addEventListener("click", previewPendingAudio);
elements.importTrack.addEventListener("change", () => {
  elements.importCommit.disabled = !pendingAudioPath || !elements.importTrack.value;
});
elements.importCommit.addEventListener("click", async () => {
  const trackId = elements.importTrack.value;
  if (!pendingAudioPath || !trackId) return;
  stopPreview();
  await whileBusy([elements.importCommit], async () => {
    try {
      const snapshot = await platform.importAudio({
        path: pendingAudioPath,
        trackId,
        copyIntoProject: elements.importMode.value === "copy",
        sourceChannelSelection: elements.importChannels.value.split(",").map(Number),
        startTick: editCursorTick,
      });
      pendingAudioPath = null;
      elements.importSelected.textContent = "No hay archivo seleccionado";
      elements.importPreview.disabled = true;
      renderSnapshot(snapshot);
      setNotice("Audio importado", "La fuente y región se registraron; el Arreglo muestra su waveform. La reproducción de sesión sigue pendiente.");
    } catch (error) {
      setNotice("No se pudo importar audio", String(error));
    }
  });
});

async function loadWaveform(sourceId, sourceDigest, container) {
  try {
    const cacheKey = sourceDigest ?? sourceId;
    let bins = waveformCache.get(cacheKey);
    if (!bins) {
      bins = platform.audioWaveform(sourceId);
      waveformCache.set(cacheKey, bins);
    }
    bins = await bins;
    const points = bins.map(([min, max], index) => {
      const x = bins.length <= 1 ? 0 : index * 512 / (bins.length - 1);
      return [x, Math.max(0, Math.min(100, 50 * (1 - max))), Math.max(0, Math.min(100, 50 * (1 - min)))];
    });
    const top = points.map(([x, y]) => `${x},${y}`).join(" ");
    const bottom = [...points].reverse().map(([x, , y]) => `${x},${y}`).join(" ");
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 512 100");
    svg.setAttribute("preserveAspectRatio", "none");
    const shape = document.createElementNS("http://www.w3.org/2000/svg", "polygon");
    shape.setAttribute("points", `${top} ${bottom}`);
    svg.append(shape);
    container.replaceChildren(svg);
  } catch (error) {
    waveformCache.delete(sourceDigest ?? sourceId);
    container.title = `No se pudo calcular la forma de onda: ${String(error)}`;
  }
}

function renderTimelineRuler(beatsPerBar) {
  elements.ruler.replaceChildren();
  for (let bar = 1; bar <= 16; bar += 1) {
    const tick = document.createElement("span");
    tick.className = "bar-tick";
    tick.textContent = String(bar);
    elements.ruler.append(tick);
  }
  const cursor = document.createElement("span");
  cursor.className = "edit-cursor";
  cursor.setAttribute("aria-hidden", "true");
  elements.ruler.append(cursor);
  const playhead = document.createElement("span");
  playhead.className = "playhead";
  playhead.setAttribute("aria-hidden", "true");
  elements.ruler.append(playhead);
  elements.ruler.dataset.beatsPerBar = String(beatsPerBar);
  renderEditCursor();
}

function renderTransportPosition(ticks) {
  transportPositionTick = Math.max(0, Number(ticks) || 0);
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const ticksPerBar = beatsPerBar * 960;
  const timelineTicks = ticksPerBar * 16;
  const left = `${Math.max(0, Math.min(100, transportPositionTick / timelineTicks * 100))}%`;
  document.querySelectorAll(".playhead").forEach((playhead) => { playhead.style.left = left; });
  const bar = Math.floor(transportPositionTick / ticksPerBar) + 1;
  const beatPosition = (transportPositionTick % ticksPerBar) / 960;
  const beat = Math.floor(beatPosition) + 1;
  const subdivision = Math.floor((beatPosition % 1) * 4) + 1;
  elements.transportPosition.textContent = `${bar}.${beat}.${subdivision}`;
}

function formatBarBeat(ticks) {
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const ticksPerBar = beatsPerBar * 960;
  const bar = Math.floor(ticks / ticksPerBar) + 1;
  const beat = (ticks % ticksPerBar) / 960 + 1;
  return `${bar}.${beat.toLocaleString("es-CL", { maximumFractionDigits: 2 })}`;
}

function renderEditCursor() {
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const timelineTicks = beatsPerBar * 16 * 480;
  const left = `${Math.max(0, Math.min(100, editCursorTick / timelineTicks * 100))}%`;
  document.querySelectorAll(".edit-cursor").forEach((cursor) => { cursor.style.left = left; });
  const bar = Math.floor(editCursorTick / (beatsPerBar * 480)) + 1;
  const beat = ((editCursorTick % (beatsPerBar * 480)) / 480) + 1;
  elements.editCursorPosition.textContent = `Cursor: compás ${bar} · pulso ${beat.toLocaleString("es-CL", { maximumFractionDigits: 2 })}`;
  elements.importBar.value = String(bar);
}

function setEditCursorFromX(clientX, bounds) {
  if (bounds.width <= 0) return;
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const timelineTicks = beatsPerBar * 16 * 480;
  const fraction = Math.max(0, Math.min(1, (clientX - bounds.left) / bounds.width));
  editCursorTick = Math.round(fraction * timelineTicks / 120) * 120;
  renderEditCursor();
}

async function runCommand(title, operation) {
  try {
    const snapshot = await operation();
    if (!snapshot) return;
    renderSnapshot(snapshot);
    setNotice(title, "Cambios aplicados al estado del proyecto.");
  } catch (error) {
    setNotice("La operación falló", String(error));
  }
}

elements.open.addEventListener("click", async () => {
  await whileBusy([elements.open, elements.browserOpen], async () => { try {
    const snapshot = await platform.openProject();
    if (!snapshot) return;
    renderSnapshot(snapshot);
    setNotice("Proyecto abierto", "El estado se carga a través de ProjectApplication.");
  } catch (error) {
    setNotice("No se pudo abrir el proyecto", String(error));
  } });
});

async function addTrack(kind, button) {
  await whileBusy([button], async () => { try {
    const snapshot = await platform.addTrack(kind);
    renderSnapshot(snapshot);
    const label = kind === "audio" ? "Pista de audio" : kind === "bus" ? "Bus" : "Pista MIDI";
    setNotice(`${label} creada`, "La pista quedó en el proyecto y su creación puede deshacerse desde el historial.");
  } catch (error) {
    setNotice("No se pudo crear la pista", String(error));
  } });
}

elements.addMidiTrack.addEventListener("click", () => addTrack("midi", elements.addMidiTrack));
elements.addAudioTrack.addEventListener("click", () => addTrack("audio", elements.addAudioTrack));
elements.addBusTrack.addEventListener("click", () => addTrack("bus", elements.addBusTrack));
elements.ruler.addEventListener("click", async (event) => {
  setEditCursorFromX(event.clientX, elements.ruler.getBoundingClientRect());
  if (projectTransportState !== "playing") return;
  try {
    const snapshot = await platform.setTransport("seek", editCursorTick * 2);
    renderSnapshot(snapshot);
    renderTransportPosition(await platform.transportPosition());
    setNotice("Transporte reubicado", `Reproducción desde ${elements.transportPosition.textContent}.`);
  } catch (error) {
    setNotice("No se pudo reubicar el transporte", String(error));
  }
});
elements.loopPointA.addEventListener("click", () => {
  pendingLoopStartTick = editCursorTick * 2;
  if (loopRange && pendingLoopStartTick < loopRange.endTick) {
    saveLoopRange(pendingLoopStartTick, loopRange.endTick);
  } else {
    setNotice("Inicio A guardado", "Coloca el cursor después de A y fija el punto B para definir el rango.");
  }
});
elements.loopPointB.addEventListener("click", () => {
  const endTick = editCursorTick * 2;
  const startTick = pendingLoopStartTick ?? loopRange?.startTick;
  if (startTick === undefined || startTick === null || endTick <= startTick) {
    setNotice("Rango no válido", "Fija A antes de B y sitúa el punto B después de A.");
    return;
  }
  saveLoopRange(startTick, endTick);
});
elements.loopRangeClear.addEventListener("click", () => saveLoopRange(null, null));

async function saveLoopRange(startTick, endTick) {
  try {
    const snapshot = await platform.setLoopRange(startTick, endTick);
    renderSnapshot(snapshot);
    pendingLoopStartTick = null;
    setNotice("Rango de repetición guardado", loopRange
      ? `A: ${formatBarBeat(loopRange.startTick)} · B: ${formatBarBeat(loopRange.endTick)}. El rango se aplica al próximo inicio de Play.`
      : "Se quitó el rango guardado del proyecto.");
  } catch (error) {
    setNotice("No se pudo guardar el rango", String(error));
  }
}
elements.importBar.addEventListener("change", () => {
  const bar = Math.max(1, Number(elements.importBar.value) || 1);
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  editCursorTick = Math.round((bar - 1) * beatsPerBar * 480);
  renderEditCursor();
});
elements.browserMediaSearch.addEventListener("input", () => {
  if (lastSnapshot) renderProjectMedia(lastSnapshot);
});
elements.browserMediaChannels.addEventListener("change", () => {
  if (lastSnapshot) renderProjectMedia(lastSnapshot);
});
elements.showArrangement.addEventListener("click", () => selectSurface("arrangement"));
elements.showSession.addEventListener("click", () => selectSurface("session"));
elements.showMixer.addEventListener("click", () => selectSurface("mixer"));
elements.gridSnap.addEventListener("change", () => {
  const label = elements.gridSnap.selectedOptions[0]?.textContent ?? "rejilla";
  setNotice("Ajuste actualizado", `El movimiento y recorte de regiones de audio usarán ${label}.`);
});
elements.railArrangement.addEventListener("click", () => selectSurface("arrangement"));
elements.railSession.addEventListener("click", () => selectSurface("session"));
elements.railSettings.addEventListener("click", () => {
  elements.audioSettings.open = true;
  elements.audioSettings.scrollIntoView({ block: "nearest" });
  elements.audioSettings.querySelector("summary").focus();
});

elements.newProject.addEventListener("click", async () => {
  await whileBusy([elements.newProject], async () => { try {
    const snapshot = await platform.newProject();
    renderSnapshot(snapshot);
    setNotice("Proyecto nuevo", "Sesión vacía lista. Abre un proyecto con clips MIDI para escuchar su reproducción.");
  } catch (error) {
    setNotice("No se pudo crear el proyecto", String(error));
  } });
});

elements.demoProject.addEventListener("click", async () => {
  await whileBusy([elements.demoProject, elements.browserDemo], async () => { try {
    const snapshot = await platform.demoMidiProject();
    renderSnapshot(snapshot);
    setNotice("Demo MIDI lista", "Siete notas están preparadas en la pista. Pulsa Play para oírlas por la salida configurada.");
  } catch (error) {
    setNotice("No se pudo preparar la demo MIDI", String(error));
  } });
});

elements.browserDemo.addEventListener("click", () => elements.demoProject.click());
elements.browserOpen.addEventListener("click", () => elements.open.click());

elements.save.addEventListener("click", () => runCommand("Proyecto guardado", () => platform.saveProject()));
elements.saveAs.addEventListener("click", () => runCommand("Copia del proyecto guardada", () => platform.saveProjectAs()));
elements.play.addEventListener("click", async () => {
  stopPreview();
  await whileBusy([elements.play], async () => { try {
    transportLoopErrorReported = false;
    const startAtCursor = projectTransportState === "stopped";
    const snapshot = await platform.setTransport("play", startAtCursor ? editCursorTick * 2 : null);
    renderSnapshot(snapshot);
    renderTransportPosition(await platform.transportPosition());
    const hasContent = snapshot.midiClipCount > 0 || snapshot.audioClipCount > 0;
    setNotice(hasContent ? "Transporte en Play" : "Sesión vacía", hasContent
      ? startAtCursor
        ? `Reproducción desde el cursor de Arreglo (${elements.transportPosition.textContent}).`
        : `Transporte reanudado en ${elements.transportPosition.textContent}.`
      : "No hay regiones MIDI ni de audio en este proyecto.");
  } catch (error) {
    setNotice("No se pudo iniciar la reproducción", String(error));
  } });
});
elements.record.addEventListener("click", async () => {
  stopPreview();
  await whileBusy([elements.record], async () => {
    try {
      const snapshot = await platform.setTransport("record", editCursorTick * 2);
      audioRecording = true;
      renderSnapshot(snapshot);
      renderTransportPosition(await platform.transportPosition());
      setNotice("Grabación en curso", "Se capturan las pistas armadas. Usa Detener para finalizar y crear las regiones de audio.");
    } catch (error) {
      audioRecording = false;
      setNotice("No se pudo iniciar la grabación", String(error));
    }
  });
});
elements.pause.addEventListener("click", () => runCommand("Transporte pausado", () => platform.setTransport("pause")));
elements.stop.addEventListener("click", async () => {
  const priorAudioCount = Number(elements.audioCount.textContent) || 0;
  try {
    const snapshot = await platform.setTransport("stop");
    audioRecording = false;
    renderSnapshot(snapshot);
    const created = Math.max(0, snapshot.audioClipCount - priorAudioCount);
    setNotice("Transporte detenido", created
      ? `Se añadieron ${created} región${created === 1 ? "" : "es"} de audio grabada${created === 1 ? "" : "s"} al proyecto.`
      : "El transporte se detuvo.");
  } catch (error) {
    const wasRecording = audioRecording;
    audioRecording = false;
    try {
      renderSnapshot(await platform.projectSnapshot());
    } catch (_) {
      // Preserve the original transport error if refreshing the UI also fails.
    }
    setNotice(wasRecording ? "Grabación finalizada con incidencia" : "No se pudo detener el transporte", String(error));
  }
});
elements.panic.addEventListener("click", () => runCommand("Notas MIDI apagadas", () => platform.setTransport("panic")));
elements.metronome.addEventListener("click", async () => {
  const enabled = !metronomeEnabled;
  try {
    const snapshot = await platform.setTransport(enabled ? "metronome-on" : "metronome-off");
    metronomeEnabled = enabled;
    renderSnapshot(snapshot);
    elements.metronome.setAttribute("aria-pressed", String(enabled));
    elements.metronome.title = enabled ? "Desactivar metrónomo" : "Activar metrónomo";
    elements.metronome.classList.toggle("is-selected", enabled);
    setNotice(enabled ? "Metrónomo activado" : "Metrónomo desactivado", "El clic sigue el tempo y la métrica del proyecto.");
  } catch (error) {
    setNotice("No se pudo cambiar el metrónomo", String(error));
  }
});
elements.undo.addEventListener("click", () => runCommand("Undo aplicado", () => platform.historyAction("undo")));
elements.redo.addEventListener("click", () => runCommand("Redo aplicado", () => platform.historyAction("redo")));

setInterval(async () => {
  if (!["playing", "paused"].includes(projectTransportState) || transportPositionPollPending) return;
  transportPositionPollPending = true;
  try {
    if (projectTransportState === "playing") {
      renderTransportPosition(await platform.transportPosition());
    }
    updateTrackMeters(await platform.trackMeters());
  } catch (error) {
    if (!transportLoopErrorReported) {
      transportLoopErrorReported = true;
      setNotice("Falló la repetición A/B", String(error));
    }
  } finally {
    transportPositionPollPending = false;
  }
}, 50);

elements.audioProfile.addEventListener("change", () => {
  if (!audioSettings) return;
  audioSettings.activeProfile = elements.audioProfile.value;
  renderAudioProfile();
});

elements.saveAudioSettings.addEventListener("click", async () => {
  if (!audioSettings) return;
  audioSettings.backendDeviceKey = elements.audioOutputDevice.value;
  const profile = selectedAudioProfile();
  profile.devicePeriodFrames = Number(elements.devicePeriod.value);
  profile.playbackSafetyFrames = Number(elements.playbackSafety.value);
  try {
    const view = await platform.saveAudioSettings(audioSettings);
    audioSettings = view.settings;
    elements.audioProfile.value = audioSettings.activeProfile;
    renderAudioProfile(view);
    setNotice("Preferencias de audio guardadas", "El perfil se usará al iniciar el siguiente stream. El cambio no modifica un stream que ya esté ejecutándose.");
  } catch (error) {
    setNotice("No se pudieron guardar los buffers", String(error));
  }
});

elements.zoomIn.addEventListener("click", () => void setUiZoom(uiZoom + UI_ZOOM_STEP));
elements.zoomOut.addEventListener("click", () => void setUiZoom(uiZoom - UI_ZOOM_STEP));
elements.zoomReset.addEventListener("click", () => void setUiZoom(1));
document.addEventListener("keydown", handleUiZoomShortcut);
document.addEventListener("keydown", handleWorkstationShortcut);
document.addEventListener("contextmenu", showContextMenu);
document.addEventListener("pointerdown", (event) => {
  if (!event.target.closest("#action-context-menu")) closeContextMenu();
  if (!event.target.closest(".application-menu-group")) {
    elements.applicationMenu.querySelectorAll(".application-menu-group.is-open").forEach((group) => {
      group.classList.remove("is-open");
      group.querySelector(".application-menu-toggle")?.setAttribute("aria-expanded", "false");
    });
  }
});
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  closeContextMenu();
  elements.applicationMenu.querySelectorAll(".application-menu-group.is-open").forEach((group) => {
    group.classList.remove("is-open");
    group.querySelector(".application-menu-toggle")?.setAttribute("aria-expanded", "false");
  });
});
renderApplicationMenu();

// Este shell inicial sólo resume datos compactos; jamás solicita PCM o buffers
// GPU al core a través del bridge.
setProjectEnabled(false);
elements.undo.disabled = true;
elements.redo.disabled = true;
renderTimelineRuler(4);
initializeUiZoom();
loadAudioSettings();
