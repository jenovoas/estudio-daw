const platform = window.estudioPlatform;

const elements = {
  open: document.querySelector("#open-project"),
  importAudio: document.querySelector("#browser-import-audio"),
  importPreview: document.querySelector("#audio-import-preview"),
  importCommit: document.querySelector("#audio-import-commit"),
  importSelected: document.querySelector("#audio-import-selected"),
  importTrack: document.querySelector("#audio-import-track"),
  importBar: document.querySelector("#audio-import-bar"),
  editCursorPosition: document.querySelector("#edit-cursor-position"),
  importMode: document.querySelector("#audio-import-mode"),
  importChannels: document.querySelector("#audio-import-channels"),
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
  browserMediaChannels: document.querySelector("#browser-media-channels"),
  browserMediaTools: document.querySelector("#browser-media-tools"),
  browserMediaList: document.querySelector("#browser-media-list"),
  browserMidiList: document.querySelector("#browser-midi-list"),
  browserMidiContent: document.querySelector("#browser-midi-content"),
  browserInstrumentList: document.querySelector("#browser-instrument-list"),
  browserInstrumentContent: document.querySelector("#browser-instrument-content"),
  browserAudioContent: document.querySelector("#browser-audio-content"),
  browserLibraryEmpty: document.querySelector("#browser-library-empty"),
  browserContentSearch: document.querySelector("#browser-content-search"),
  deviceInspector: document.querySelector("#device-inspector"),
  detailClipTab: document.querySelector("#detail-clip-tab"),
  detailDeviceTab: document.querySelector("#detail-device-tab"),
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
  overview: document.querySelector("#arrangement-overview"),
  arrangementView: document.querySelector(".arrangement-scroll"),
  sessionView: document.querySelector("#session-view"),
  mixerView: document.querySelector("#mixer-view"),
  workstation: document.querySelector(".workstation"),
  toggleBrowser: document.querySelector("#toggle-browser"),
  toggleClipDetail: document.querySelector("#toggle-clip-detail"),
  lowerPanelTitle: document.querySelector("#lower-panel-title"),
  returnToArrangement: document.querySelector("#return-to-arrangement"),
  arrangementLegend: document.querySelector("#arrangement-legend"),
  showArrangement: document.querySelector("#show-arrangement"),
  showSession: document.querySelector("#show-session"),
  showMixer: document.querySelector("#show-mixer"),
  gridSnap: document.querySelector("#grid-snap"),
  railSettings: document.querySelector("#rail-settings"),
  audioSettings: document.querySelector("#audio-settings"),
  clipInspector: document.querySelector("#clip-inspector"),
  editor: document.querySelector(".editor"),
  revision: document.querySelector("#revision"),
  projectStatus: document.querySelector("#project-status"),
  engine: document.querySelector("#engine-status"),
  noticeTitle: document.querySelector("#notice-title"),
  noticeText: document.querySelector("#notice-text"),
  notice: document.querySelector(".status-message"),
  noticeIcon: document.querySelector(".status-icon"),
  connectionDot: document.querySelector(".connection-dot"),
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
let audioReturnDevices = [];
let midiOutputDevices = [];
let audioRecording = false;
let projectTransportState = "stopped";
let transportPositionTick = 0;
let sessionLaunches = [];
let sessionOverrideActive = false;
let transportPositionPollPending = false;
let transportLoopErrorReported = false;
let metronomeEnabled = false;
let loopRange = null;
let pendingLoopStartTick = null;
let selectedTrackIds = new Set();
let trackGroupDraft = "";
let pendingAudioPath = null;
let editCursorTick = 0;
let arrangementVisibleBars = 16;
let arrangementStartBar = 1;
let arrangementTotalBars = 16;
let editCursorProjectId = null;
let selectedClipId = null;
let selectedDetailTab = "clip";
let browserVisible = true;
let clipDetailVisible = true;
let mixerPanelVisible = false;
let selectedMidiNote = null;
const waveformCache = new Map();
const vst3EditorOpenByTrack = new Map();
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
    await refreshInstrumentPorts();
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
  const tone = /no se pudo|fall[oó]|inv[aá]lid|error/i.test(title)
    ? "error"
    : /incidencia|no disponible|pendiente/i.test(title)
      ? "warning"
      : /actualizad[oa]|guardad[oa]|cread[oa]|importad[oa]|lista/i.test(title)
        ? "success"
        : "info";
  elements.notice.dataset.tone = tone;
  elements.noticeIcon.textContent = ({ error: "!", warning: "⚠", success: "✓", info: "i" })[tone];
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

function updateWorkspaceLayout() {
  const sessionActive = elements.showSession.classList.contains("is-selected");
  elements.workstation.classList.toggle("browser-collapsed", !browserVisible);
  elements.editor.classList.toggle("clip-detail-hidden", !clipDetailVisible);
  elements.editor.classList.toggle("mixer-panel-open", mixerPanelVisible && !sessionActive);
  elements.mixerView.hidden = !mixerPanelVisible || sessionActive;
  elements.showMixer.disabled = sessionActive;
  elements.showMixer.title = sessionActive
    ? "La mezcla está alineada bajo las pistas en Session"
    : "Mostrar u ocultar el mezclador (Ctrl+3)";
  elements.clipInspector.hidden = selectedDetailTab !== "clip";
  elements.deviceInspector.hidden = selectedDetailTab !== "device";
  elements.lowerPanelTitle.textContent = selectedDetailTab === "device" ? "DISPOSITIVO" : "DETALLE DE CLIP";
  elements.detailClipTab.classList.toggle("is-selected", selectedDetailTab === "clip");
  elements.detailDeviceTab.classList.toggle("is-selected", selectedDetailTab === "device");
  elements.detailClipTab.setAttribute("aria-pressed", String(selectedDetailTab === "clip"));
  elements.detailDeviceTab.setAttribute("aria-pressed", String(selectedDetailTab === "device"));
  elements.showMixer.setAttribute("aria-pressed", String(mixerPanelVisible));
  elements.toggleBrowser.setAttribute("aria-pressed", String(browserVisible));
  elements.toggleBrowser.title = browserVisible
    ? "Ocultar navegador (Ctrl+Alt+B)"
    : "Mostrar navegador (Ctrl+Alt+B)";
  const clipViewVisible = clipDetailVisible;
  elements.toggleClipDetail.setAttribute("aria-pressed", String(clipViewVisible));
  elements.toggleClipDetail.title = clipViewVisible
    ? "Ocultar detalle del clip (Mayús+Tab)"
    : "Mostrar detalle del clip (Mayús+Tab)";
}
updateWorkspaceLayout();

function handleCreativeWorkspaceShortcut(event) {
  if (event.repeat || event.metaKey) return;
  if (event.target instanceof Element && event.target.closest("input, textarea, select, [contenteditable='true']")) return;
  if (event.ctrlKey && event.altKey && event.key.toLowerCase() === "b") {
    event.preventDefault();
    browserVisible = !browserVisible;
    updateWorkspaceLayout();
    return;
  }
  if (event.ctrlKey || event.altKey) return;
  if (event.key === "Tab" && event.shiftKey) {
    event.preventDefault();
    if (mixerPanelVisible) mixerPanelVisible = false;
    clipDetailVisible = !clipDetailVisible;
    updateWorkspaceLayout();
  } else if (event.key === "Tab") {
    event.preventDefault();
    selectSurface(elements.showSession.classList.contains("is-selected") ? "arrangement" : "session");
  }
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
  };
  elements.arrangementView.hidden = surface !== "arrangement";
  elements.sessionView.hidden = surface !== "session";
  elements.arrangementLegend.hidden = surface !== "arrangement";
  for (const [name, button] of Object.entries(selected)) {
    const active = name === surface;
    button.classList.toggle("is-selected", active);
    button.setAttribute("aria-selected", String(active));
  }
  updateWorkspaceLayout();
}

function updateReturnToArrangementButton() {
  elements.returnToArrangement.hidden = !sessionOverrideActive;
  elements.returnToArrangement.disabled = projectTransportState !== "playing";
  elements.returnToArrangement.title = projectTransportState === "playing"
    ? "Quitar las sustituciones de Session y continuar Arrangement desde la posición actual"
    : "Reanuda Play para volver al Arreglo sin reiniciar el transporte";
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
  if (track.instrument?.backend === "standalone") {
    const midi = track.instrument.midi_output ? "MIDI → Analog Lab" : "MIDI sin destino";
    const audio = track.instrument.audio_input ? "retorno → pista" : "retorno sin asignar";
    return `${midi} · ${audio} → ganancia/pan → medidor → ${destination}`;
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

function vst3PluginLabel(path) {
  const file = String(path || "").split(/[\\/]/).filter(Boolean).pop() || "VST3";
  return file.replace(/\.vst3$/i, "") || "VST3";
}

function createTrackInstrumentControl(track) {
  if (track.virtualMaster || track.kind !== "midi") return null;
  const currentVst3 = track.instrument?.backend === "vst3" ? track.instrument : null;
  const chooseVst3 = async () => {
    const path = await platform.selectVst3Plugin();
    if (!path) return;
    try {
      const report = await platform.inspectVst3Plugin(path);
      const info = report.info;
      if (!info.has_midi_input || info.audio_outputs < 1) {
        setNotice("El plugin no es un instrumento MIDI", `${info.name} debe recibir MIDI y ofrecer audio para asignarse a una pista.`);
        return;
      }
      const snapshot = await platform.setTrackInstrument(track.id, {
        backend: "vst3",
        plugin: {
          format: "vst3",
          path,
          uniqueId: info.uid,
          bridge: path.toLowerCase().includes("/yabridge/") ? "yabridge" : null,
        },
        state: null,
      });
      setNotice("Instrumento asignado", `${info.name} · ${info.vendor} · MIDI y audio listos para revisar.`);
      renderSnapshot(snapshot);
    } catch (error) {
      setNotice("No se pudo leer el VST3", String(error));
    }
  };
  if (currentVst3) {
    const panel = document.createElement("div");
    panel.className = "track-instrument-control is-device";
    panel.style.setProperty("--track-color", track.color);
    const name = document.createElement("strong");
    name.className = "device-name";
    name.textContent = vst3PluginLabel(currentVst3.plugin?.path);
    name.title = currentVst3.plugin?.path ?? "";
    const show = document.createElement("button");
    show.type = "button";
    show.className = "mixer-toggle";
    const editorOpen = Boolean(vst3EditorOpenByTrack.get(track.id));
    const engineReady = ["playing", "paused"].includes(projectTransportState);
    show.textContent = editorOpen ? "OCULTAR" : "CONTROLES";
    show.classList.toggle("is-selected", editorOpen);
    show.disabled = !engineReady;
    show.title = engineReady
      ? (editorOpen ? `Ocultar los controles de ${name.textContent}` : `Abrir los controles de ${name.textContent}`)
      : "Dale a Play para cargar el instrumento y abrir sus controles";
    show.setAttribute("aria-pressed", String(editorOpen));
    show.setAttribute("aria-label", `${editorOpen ? "Ocultar" : "Mostrar"} controles de ${name.textContent} en ${track.name}`);
    show.addEventListener("click", async () => {
      const open = !vst3EditorOpenByTrack.get(track.id);
      show.disabled = true;
      try {
        await platform.setVst3Editor(track.id, open);
        vst3EditorOpenByTrack.set(track.id, open);
        setNotice(
          open ? "Controles abiertos" : "Controles ocultos",
          open
            ? `${name.textContent} muestra su ventana nativa. Ciérrala aquí o en la ventana del plugin.`
            : `${name.textContent} sigue en la pista; sólo se ocultó su ventana.`,
        );
        renderSnapshot(lastSnapshot);
      } catch (error) {
        vst3EditorOpenByTrack.set(track.id, false);
        setNotice("No se pudieron abrir los controles", String(error));
        renderSnapshot(lastSnapshot);
      }
    });
    const choose = document.createElement("button");
    choose.type = "button";
    choose.textContent = "Cambiar";
    choose.disabled = projectTransportState !== "stopped";
    choose.addEventListener("click", chooseVst3);
    const revokeTrust = document.createElement("button");
    revokeTrust.type = "button";
    revokeTrust.textContent = "Revocar confianza local";
    revokeTrust.disabled = projectTransportState !== "stopped";
    revokeTrust.addEventListener("click", async () => {
      try {
        await platform.revokeExternalCode(currentVst3.plugin.path);
        setNotice("Confianza revocada", "Este plugin volverá a pedir aprobación antes de inspeccionarse o cargarse.");
      } catch (error) {
        setNotice("No se pudo revocar la confianza", String(error));
      }
    });
    const help = document.createElement("small");
    const bridge = currentVst3.plugin?.bridge ? `${currentVst3.plugin.bridge} · ` : "";
    help.textContent = engineReady
      ? `${bridge}El instrumento está en esta pista. CONTROLES abre su ventana real; la primera vez puede tardar unos segundos.`
      : `${bridge}Play carga el instrumento. Después CONTROLES abre su ventana.`;
    panel.append(name, show, choose, revokeTrust, help);
    return panel;
  }
  const current = track.instrument?.backend === "standalone" ? track.instrument : null;
  const panel = document.createElement("details");
  panel.className = "track-instrument-control";
  const summary = document.createElement("summary");
  summary.textContent = current ? "Instrumento externo · Analog Lab" : "Elegir instrumento";
  panel.append(summary);
  const chooseVst3Button = document.createElement("button");
  chooseVst3Button.type = "button";
  chooseVst3Button.textContent = "Elegir instrumento VST3";
  chooseVst3Button.disabled = projectTransportState !== "stopped";
  chooseVst3Button.addEventListener("click", chooseVst3);
  panel.append(chooseVst3Button);

  const application = document.createElement("input");
  application.type = "text";
  application.readOnly = true;
  application.placeholder = "Elige el ejecutable de Analog Lab (.exe)";
  application.value = current?.application_path ?? "";
  application.setAttribute("aria-label", `Aplicación de instrumento para ${track.name}`);
  const choose = document.createElement("button");
  choose.type = "button";
  choose.textContent = "Elegir Analog Lab";
  choose.disabled = projectTransportState !== "stopped";
  choose.addEventListener("click", async () => {
    const path = await platform.selectStandaloneInstrument();
    if (path) {
      application.value = path;
      void refreshLocalWinePrefix(path);
      updateSaveAvailability();
    }
  });

  const prefix = document.createElement("input");
  prefix.type = "text";
  prefix.readOnly = true;
  prefix.placeholder = "Prefijo Wine local (vacío = predeterminado)";
  prefix.setAttribute("aria-label", `Prefijo Wine local de ${track.name}`);
  const refreshLocalWinePrefix = async (path) => {
    prefix.value = path ? await platform.getLocalWinePrefix(path) ?? "" : "";
  };
  if (application.value) void refreshLocalWinePrefix(application.value);
  const choosePrefix = document.createElement("button");
  choosePrefix.type = "button";
  choosePrefix.textContent = "Elegir prefijo local";
  choosePrefix.disabled = projectTransportState !== "stopped" || !application.value;
  choosePrefix.addEventListener("click", async () => {
    if (!application.value) return;
    const path = await platform.selectWinePrefix();
    if (!path) return;
    try {
      prefix.value = await platform.setLocalWinePrefix(application.value, path) ?? "";
      setNotice("Prefijo Wine local guardado", "Este ajuste queda en la configuración de usuario y no dentro del proyecto.");
    } catch (error) {
      setNotice("No se pudo guardar el prefijo Wine", String(error));
    }
  });
  const clearPrefix = document.createElement("button");
  clearPrefix.type = "button";
  clearPrefix.textContent = "Usar prefijo predeterminado";
  clearPrefix.disabled = projectTransportState !== "stopped" || !application.value;
  clearPrefix.addEventListener("click", async () => {
    if (!application.value) return;
    try {
      prefix.value = await platform.setLocalWinePrefix(application.value, null) ?? "";
      setNotice("Prefijo Wine local quitado", "Wine usará su prefijo predeterminado local.");
    } catch (error) {
      setNotice("No se pudo quitar el prefijo Wine", String(error));
    }
  });
  const open = document.createElement("button");
  open.type = "button";
  open.textContent = "Abrir y buscar puertos";
  open.disabled = projectTransportState !== "stopped";
  open.addEventListener("click", async () => {
    if (!application.value) return;
    try {
      await platform.openStandaloneInstrument({
        trackId: track.id,
        applicationPath: application.value,
      });
      setNotice("Analog Lab abierto", "Buscando sus puertos MIDI y de audio en PipeWire…");
      await new Promise((resolve) => setTimeout(resolve, 1500));
      await refreshInstrumentPorts();
      renderSnapshot(lastSnapshot);
    } catch (error) {
      setNotice("No se pudo abrir Analog Lab", String(error));
    }
  });
  const revokeTrust = document.createElement("button");
  revokeTrust.type = "button";
  revokeTrust.textContent = "Revocar confianza local";
  revokeTrust.disabled = projectTransportState !== "stopped" || !current?.application_path;
  revokeTrust.addEventListener("click", async () => {
    try {
      await platform.revokeExternalCode(current.application_path);
      setNotice("Confianza revocada", "Esta aplicación volverá a pedir aprobación antes de abrirse.");
    } catch (error) {
      setNotice("No se pudo revocar la confianza", String(error));
    }
  });

  const midi = document.createElement("select");
  midi.setAttribute("aria-label", `Entrada MIDI de Analog Lab para ${track.name}`);
  midi.append(new Option("Entrada MIDI de Analog Lab", ""));
  for (const port of midiOutputDevices) midi.append(new Option(port.label, port.key));
  if (current?.midi_output?.portName && !midiOutputDevices.some((port) => port.key === current.midi_output.portName)) {
    midi.append(new Option(`No disponible · ${current.midi_output.portName}`, current.midi_output.portName));
  }
  midi.value = current?.midi_output?.portName ?? "";

  const audio = document.createElement("select");
  audio.setAttribute("aria-label", `Retorno de audio de Analog Lab para ${track.name}`);
  audio.append(new Option("Retorno de audio a la pista", ""));
  for (const device of audioReturnDevices) {
    audio.append(new Option(device.description ? `${device.description} · ${device.name}` : device.name, device.key));
  }
  if (current?.audio_input?.nodeKey && !audioReturnDevices.some((device) => device.key === current.audio_input.nodeKey)) {
    audio.append(new Option(`No disponible · ${current.audio_input.nodeKey}`, current.audio_input.nodeKey));
  }
  audio.value = current?.audio_input?.nodeKey ?? "";

  const save = document.createElement("button");
  save.type = "button";
  save.textContent = "Usar en esta pista";
  const updateSaveAvailability = () => {
    save.disabled = projectTransportState !== "stopped" || !application.value || !midi.value || !audio.value;
    choosePrefix.disabled = projectTransportState !== "stopped" || !application.value;
    clearPrefix.disabled = projectTransportState !== "stopped" || !application.value;
  };
  midi.addEventListener("change", updateSaveAvailability);
  audio.addEventListener("change", updateSaveAvailability);
  updateSaveAvailability();
  save.addEventListener("click", () => {
    if (!application.value || !midi.value || !audio.value) {
      setNotice("Falta completar el ruteo", "Elige Analog Lab, su entrada MIDI y su retorno de audio a esta pista.");
      return;
    }
    const midiOutput = midi.value
      ? { deviceKey: midi.value, portName: midi.value, channel: null }
      : null;
    const audioInput = audio.value
      ? { nodeKey: audio.value, channels: [0, 1] }
      : null;
    const instrument = {
      backend: "standalone",
      application_path: application.value,
      wine_prefix: current?.wine_prefix ?? null,
      midi_output: midiOutput,
      audio_input: audioInput,
    };
    void runCommand("Analog Lab asignado a la pista", () => platform.setTrackInstrument(track.id, instrument));
  });

  const help = document.createElement("small");
  const missingRoutes = [
    !current?.midi_output && "entrada MIDI",
    !current?.audio_input && "retorno de audio",
  ].filter(Boolean);
  help.textContent = current
    ? (missingRoutes.length
      ? `Falta elegir ${missingRoutes.join(" y ")}.`
      : "Se abrirá al reproducir. MIDI sale por el puerto elegido; el retorno pasa por esta pista y el mezclador.")
    : "Abre la app para descubrir sus puertos. El retorno se suma a esta pista y no usa el monitor general.";
  panel.append(application, choose, prefix, choosePrefix, clearPrefix, open, revokeTrust, midi, audio, save, help);
  return panel;
}

async function refreshInstrumentPorts() {
  const [midi, audio] = await Promise.all([
    platform.midiOutputDevices().catch(() => []),
    platform.audioReturnDevices().catch(() => []),
  ]);
  midiOutputDevices = midi;
  audioReturnDevices = audio;
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

function createTrackIdentityControl(track) {
  if (track.virtualMaster) return null;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "track-identity-button";
  button.textContent = track.marker || "◉";
  button.style.setProperty("--track-color", track.color);
  button.title = ["Editar color e identidad de pista", track.annotation].filter(Boolean).join(" · ");
  button.setAttribute("aria-label", `Editar color, marca y nota de ${track.name}`);
  button.addEventListener("click", () => editTrackIdentity(track));
  return button;
}

function editTrackIdentity(track) {
  const dialog = document.createElement("dialog");
  dialog.className = "track-identity-dialog";
  const form = document.createElement("form");
  form.method = "dialog";
  const heading = document.createElement("h2");
  heading.textContent = `Identidad de ${track.name}`;
  const colorLabel = document.createElement("label");
  colorLabel.textContent = "Color del instrumento";
  const color = document.createElement("input");
  color.type = "color";
  color.value = /^#[\da-f]{6}$/i.test(track.color) ? track.color : "#58a6b8";
  colorLabel.append(color);
  const markerLabel = document.createElement("label");
  markerLabel.textContent = "Marca para reconocerlo";
  const marker = document.createElement("select");
  for (const [value, label] of [["", "Sin marca"], ["🎹", "Teclas / piano"], ["🥁", "Batería / percusión"], ["🎸", "Guitarra / bajo"], ["🎤", "Voz"], ["🎻", "Cuerdas"], ["🎺", "Vientos"], ["🎛️", "Sintetizador"], ["♪", "Instrumento MIDI"], ["◖", "Audio"]]) {
    marker.append(new Option(label, value));
  }
  marker.value = track.marker ?? "";
  markerLabel.append(marker);
  const noteLabel = document.createElement("label");
  noteLabel.textContent = "Nota de pista";
  const annotation = document.createElement("textarea");
  annotation.maxLength = 256;
  annotation.rows = 3;
  annotation.placeholder = "Por ejemplo: bajo principal, toma 2, entra en el estribillo…";
  annotation.value = track.annotation ?? "";
  noteLabel.append(annotation);
  const actions = document.createElement("div");
  actions.className = "track-identity-actions";
  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.textContent = "Cancelar";
  cancel.addEventListener("click", () => dialog.close());
  const save = document.createElement("button");
  save.type = "submit";
  save.className = "button-accent";
  save.textContent = "Guardar";
  actions.append(cancel, save);
  form.append(heading, colorLabel, markerLabel, noteLabel, actions);
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    save.disabled = true;
    const saved = await runCommand("Identidad de pista actualizada", () => platform.setTrackIdentity(track.id, {
      color: color.value,
      marker: marker.value,
      annotation: annotation.value,
    }));
    if (saved) dialog.close();
    else save.disabled = false;
  });
  dialog.append(form);
  dialog.addEventListener("close", () => dialog.remove(), { once: true });
  document.body.append(dialog);
  dialog.showModal();
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

function sessionQuantizeTicks() {
  const snap = elements.gridSnap.value;
  const beatsPerBar = Number(lastSnapshot?.beatsPerBar) || 4;
  if (snap === "0") return 0;
  if (snap === "bar") return Math.round(beatsPerBar * 960);
  return Math.max(0, Math.round(Number(snap) * 960));
}

function sessionQuantizeLabel(override = "global") {
  if (override !== "global") {
    const labels = {
      immediate: "ahora",
      sixteenth: "en la siguiente semicorchea",
      eighth: "en la siguiente corchea",
      quarter: "en la siguiente negra",
      bar: "en el siguiente compás",
    };
    return labels[override] ?? "la rejilla de la casilla";
  }
  if (elements.gridSnap.value === "0") return "ahora";
  return `en la rejilla global (${elements.gridSnap.selectedOptions[0]?.textContent ?? "compás"})`;
}

async function launchSessionSlot(sceneId, trackId, respectClipQuantization = true) {
  try {
    const launch = await platform.launchSessionSlot(sceneId, trackId, sessionQuantizeTicks(), respectClipQuantization);
    const launchQuantization = (lastSnapshot?.clipSlots ?? []).find(
      (slot) => slot.sceneId === sceneId && slot.trackId === trackId,
    )?.launchQuantization ?? "global";
    const launchMode = (lastSnapshot?.clipSlots ?? []).find(
      (slot) => slot.sceneId === sceneId && slot.trackId === trackId,
    )?.launchMode ?? "loop";
    sessionOverrideActive = true;
    sessionLaunches = sessionLaunches.filter((item) => item.trackId !== trackId).concat(launch);
    setNotice("Clip en cola", launch.state === "queued"
      ? `Sonará ${sessionQuantizeLabel(launchQuantization)}.`
      : launchMode === "one_shot" ? "El clip está reproduciendo una pasada." : "El clip está repitiéndose hasta detenerlo.");
    if (lastSnapshot) renderSnapshot(lastSnapshot);
  } catch (error) {
    setNotice("No se pudo lanzar el clip", String(error));
  }
}

async function launchSessionScene(sceneId, snapshot) {
  const slots = (snapshot.clipSlots ?? []).filter((slot) => slot.sceneId === sceneId && slot.clipId && (slot.clipKind === "midi" || slot.clipKind === "audio"));
  if (!slots.length) {
    setNotice("Escena vacía", "Asigna clips MIDI o de audio antes de lanzar la escena.");
    return;
  }
  try {
    const launches = await platform.launchSessionScene(sceneId, sessionQuantizeTicks());
    sessionOverrideActive = true;
    const trackIds = new Set(launches.map((launch) => launch.trackId));
    sessionLaunches = sessionLaunches.filter((item) => !trackIds.has(item.trackId)).concat(launches);
    setNotice("Escena en cola", `Los clips se lanzan juntos según la rejilla global (${sessionQuantizeLabel()}) y respetan su modo; las casillas vacías conservan su clip actual.`);
    renderSnapshot(snapshot);
  } catch (error) {
    setNotice("No se pudo lanzar la escena", String(error));
  }
}

async function stopSessionTrack(trackId) {
  try {
    await platform.stopSessionTrack(trackId);
    sessionLaunches = sessionLaunches.filter((item) => item.trackId !== trackId);
    setNotice("Pista detenida", "Las notas de esa pista se apagaron.");
    if (lastSnapshot) renderSnapshot(lastSnapshot);
  } catch (error) {
    setNotice("No se pudo detener la pista", String(error));
  }
}

async function returnToArrangement() {
  try {
    await platform.returnToArrangement();
    sessionLaunches = [];
    sessionOverrideActive = false;
    updateReturnToArrangementButton();
    selectSurface("arrangement");
    setNotice("Arreglo reanudado", "Session se detuvo en sus pistas; Play continúa desde la posición actual.");
  } catch (error) {
    setNotice("No se pudo volver a Arreglo", String(error));
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
  hint.textContent = "Lanza MIDI y audio a la rejilla; el clip se repite hasta ■";
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
  corner.textContent = "PISTAS";
  corner.style.gridColumn = String(tracks.length + 1);
  corner.style.gridRow = "1";
  grid.append(corner);
  for (const [trackIndex, track] of tracks.entries()) {
    const header = document.createElement("div");
    header.className = "session-track-header";
    header.dataset.trackId = track.id;
    header.style.gridColumn = String(trackIndex + 1);
    header.style.gridRow = "1";
    header.style.setProperty("--track-color", track.color);
    header.append(createTrackSelectionControl(track));
    const identity = createTrackIdentityControl(track);
    if (identity) header.append(identity);
    const name = document.createElement("strong");
    name.textContent = track.name;
    const type = document.createElement("small");
    type.textContent = track.kind.toUpperCase();
    const meter = createTrackMeter(track);
    header.append(name, type);
    if (meter) header.append(meter);
    const removeButton = createTrackRemovalButton(track);
    const orderControls = createTrackOrderControls(track, snapshot.tracks);
    if (orderControls) header.append(orderControls);
    if (removeButton) header.append(removeButton);
    grid.append(header);
  }
  for (const [sceneIndex, scene] of (snapshot.scenes ?? []).entries()) {
    const sceneHeader = document.createElement("div");
    sceneHeader.className = "session-scene-header";
    sceneHeader.style.gridColumn = String(tracks.length + 1);
    sceneHeader.style.gridRow = String(sceneIndex + 2);
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
    const sceneHasClips = tracks.some((track) => (snapshot.clipSlots ?? []).some((slot) => slot.sceneId === scene.id && slot.trackId === track.id && slot.clipId));
    const engineReady = ["playing", "paused"].includes(projectTransportState);
    launchScene.disabled = !sceneHasClips || !engineReady;
    launchScene.title = !sceneHasClips
      ? "Asigna clips MIDI o de audio a esta escena"
      : engineReady
        ? `Lanzar ${scene.name} ${sessionQuantizeLabel()}`
        : "Dale a Play para lanzar la escena";
    launchScene.setAttribute("aria-label", launchScene.title);
    launchScene.addEventListener("click", () => launchSessionScene(scene.id, snapshot));
    sceneHeader.append(launchScene, sceneName, moveUp, moveDown, remove);
    grid.append(sceneHeader);
    for (const [trackIndex, track] of tracks.entries()) {
      const cell = document.createElement("div");
      cell.className = "session-cell";
      cell.style.gridColumn = String(trackIndex + 1);
      cell.style.gridRow = String(sceneIndex + 2);
      cell.style.setProperty("--track-color", track.color);
      if (track.annotation) cell.title = track.annotation;
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
      const launchQuantization = document.createElement("select");
      launchQuantization.setAttribute("aria-label", `Cuantización de lanzamiento de ${track.name} en ${scene.name}`);
      launchQuantization.title = "Cuantización de lanzamiento propia de esta casilla";
      for (const [value, label] of [
        ["global", "Rejilla global"],
        ["immediate", "Ahora"],
        ["sixteenth", "1/16"],
        ["eighth", "1/8"],
        ["quarter", "Negra"],
        ["bar", "Compás"],
      ]) launchQuantization.append(new Option(label, value));
      launchQuantization.value = slot?.launchQuantization ?? "global";
      launchQuantization.disabled = !slot?.clipId;
      launchQuantization.addEventListener("change", () => runCommand(
        "Cuantización de Session actualizada",
        () => platform.setSessionLaunchQuantization(scene.id, track.id, launchQuantization.value),
      ));
      const launchMode = document.createElement("select");
      launchMode.setAttribute("aria-label", `Modo de lanzamiento de ${track.name} en ${scene.name}`);
      launchMode.title = "Se aplica al próximo lanzamiento de esta casilla";
      launchMode.append(new Option("Repetir", "loop"), new Option("Una pasada", "one_shot"));
      launchMode.value = slot?.launchMode ?? "loop";
      launchMode.disabled = !slot?.clipId;
      launchMode.addEventListener("change", () => runCommand(
        "Modo de lanzamiento de Session actualizado",
        () => platform.setSessionLaunchMode(scene.id, track.id, launchMode.value),
      ));
      const launch = document.createElement("button");
      launch.type = "button";
      const playing = sessionLaunches.find((item) => item.trackId === track.id && item.clipId === slot?.clipId);
      launch.className = `session-launch${slot?.clipId ? " has-clip" : ""}${playing?.state === "queued" ? " is-queued" : ""}${playing?.state === "playing" ? " is-playing" : ""}`;
      const engineReady = ["playing", "paused"].includes(projectTransportState);
      if (!slot?.clipId) {
        launch.textContent = "+";
        launch.disabled = true;
        launch.title = "Asigna un clip existente a esta casilla";
        launch.addEventListener("click", () => select.focus());
      } else if (playing?.state === "playing") {
        launch.textContent = "■";
        launch.disabled = !engineReady;
        launch.title = engineReady ? `Detener ${slot.clipId} en ${track.name}` : "Dale a Play para controlar Session";
        launch.addEventListener("click", () => stopSessionTrack(track.id));
      } else {
        launch.textContent = playing?.state === "queued" ? "○" : "▶";
        launch.disabled = !engineReady;
        launch.title = engineReady
          ? `Lanzar clip ${sessionQuantizeLabel(slot?.launchQuantization)} · ${track.name}`
          : "Dale a Play para lanzar el clip";
        launch.addEventListener("click", () => launchSessionSlot(scene.id, track.id));
      }
      cell.append(launch, select, launchQuantization, launchMode);
      grid.append(cell);
    }
  }
  if (!(snapshot.scenes ?? []).length) {
    const empty = document.createElement("div");
    empty.className = "surface-empty session-empty-state";
    empty.textContent = "Aún no hay escenas. Añade una escena y asigna clips existentes a las casillas.";
    empty.style.gridColumn = `1 / span ${tracks.length + 1}`;
    empty.style.gridRow = "2";
    grid.append(empty);
  }
  const mixerRow = Math.max(1, (snapshot.scenes ?? []).length) + 2;
  const mixerLabel = document.createElement("div");
  mixerLabel.className = "session-mixer-label";
  mixerLabel.textContent = "MEZCLA";
  mixerLabel.style.gridColumn = String(tracks.length + 1);
  mixerLabel.style.gridRow = String(mixerRow);
  grid.append(mixerLabel);
  for (const [trackIndex, track] of tracks.entries()) {
    const mixerCell = document.createElement("div");
    mixerCell.className = "session-mixer-cell";
    mixerCell.dataset.trackId = track.id;
    mixerCell.style.setProperty("--track-color", track.color);
    mixerCell.style.gridColumn = String(trackIndex + 1);
    mixerCell.style.gridRow = String(mixerRow);
    const meter = createTrackMeter(track);
    const controls = createTrackMixerControls(track, true);
    if (meter) mixerCell.append(meter);
    if (controls) mixerCell.append(controls);
    grid.append(mixerCell);
  }
  const mixExtras = document.createElement("div");
  mixExtras.className = "session-mixer-extras";
  for (const track of snapshot.tracks.filter((item) => ["bus", "return", "master"].includes(item.role))) {
    const channel = document.createElement("div");
    channel.className = "session-mixer-extra";
    channel.style.setProperty("--track-color", track.color);
    const label = document.createElement("strong");
    label.textContent = track.name;
    channel.append(label);
    const meter = createTrackMeter(track);
    const controls = createTrackMixerControls(track, true);
    if (meter) channel.append(meter);
    if (controls) channel.append(controls);
    mixExtras.append(channel);
  }
  if (mixExtras.childElementCount) {
    mixExtras.style.gridColumn = String(tracks.length + 1);
    mixExtras.style.gridRow = String(mixerRow + 1);
    grid.append(mixExtras);
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
    const identity = createTrackIdentityControl(track);
    if (identity) channelHeading.append(identity);
    channelHeading.append(title);
    if (orderControls) channelHeading.append(orderControls);
    if (removeButton) channelHeading.append(removeButton);
    channel.append(channelHeading, role, routing, mix);
    const instrumentControl = createTrackInstrumentControl(track);
    if (instrumentControl) channel.append(instrumentControl);
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

function renderDeviceInspector(snapshot) {
  elements.deviceInspector.replaceChildren();
  const selectedClip = snapshot.midiClips.find((clip) => clip.id === selectedClipId)
    ?? (snapshot.audioClips ?? []).find((clip) => clip.id === selectedClipId);
  const focusedTrack = [...selectedTrackIds]
    .map((id) => snapshot.tracks.find((track) => track.id === id))
    .find((track) => track && track.kind === "midi")
    ?? snapshot.tracks.find((track) => track.id === selectedClip?.trackId)
    ?? snapshot.tracks.find((track) => track.kind === "midi" && track.role !== "master");
  if (!focusedTrack) {
    const empty = document.createElement("p");
    empty.className = "surface-empty";
    empty.textContent = "Selecciona una pista MIDI para ver su instrumento asignado.";
    elements.deviceInspector.append(empty);
    return;
  }
  const heading = document.createElement("strong");
  heading.textContent = focusedTrack.name;
  const description = document.createElement("span");
  description.textContent = "Instrumento de pista";
  elements.deviceInspector.append(heading, description);
  const control = createTrackInstrumentControl(focusedTrack);
  if (control) elements.deviceInspector.append(control);
  else {
    const empty = document.createElement("span");
    empty.textContent = "No hay instrumento asignado.";
    elements.deviceInspector.append(empty);
  }
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
    selectedMidiNote = null;
    trackGroupDraft = "";
    editCursorTick = 0;
    transportPositionTick = 0;
    arrangementVisibleBars = 16;
    arrangementStartBar = 1;
    arrangementTotalBars = 16;
    editCursorProjectId = snapshot.projectId;
    sessionOverrideActive = false;
  }
  const validTrackIds = new Set(snapshot.tracks.map((track) => track.id));
  selectedTrackIds = new Set([...selectedTrackIds].filter((trackId) => validTrackIds.has(trackId)));
  projectTransportState = snapshot.transportState;
  audioRecording = audioRecording && projectTransportState !== "stopped";
  if (projectTransportState === "stopped") {
    vst3EditorOpenByTrack.clear();
    sessionLaunches = [];
    sessionOverrideActive = false;
  }
  updateReturnToArrangementButton();
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
  elements.transportTempo.textContent = Number(snapshot.tempoBpm).toFixed(1);
  elements.transport.textContent = snapshot.transportState.toUpperCase();
  document.querySelector(".transport-bar").dataset.state = snapshot.transportState.toLowerCase();
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
  elements.connectionDot.classList.toggle("is-connected", snapshot.audioEngineConnected);
  elements.undo.disabled = !snapshot.canUndo;
  elements.redo.disabled = !snapshot.canRedo;
  if (!document.querySelector(".track-group-toolbar")) {
    const toolbar = createTrackGroupToolbar();
    document.querySelector(".surface-toolbar").prepend(toolbar);
  }
  const groupInput = document.querySelector(".track-group-toolbar input");
  if (groupInput && groupInput.value !== trackGroupDraft) groupInput.value = trackGroupDraft;
  syncTrackSelectionUi();
  renderSessionSurface(snapshot);
  renderMixerSurface(snapshot.tracks);
  renderDeviceInspector(snapshot);
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
  const beatsPerBar = snapshot.beatsPerBar || 4;
  const clipEnds = [
    ...snapshot.midiClips.map((clip) => (Number(clip.startBeats) || 0) + (Number(clip.durationBeats) || 0)),
    ...(snapshot.audioClips ?? []).map((clip) => (Number(clip.startBeats) || 0) + (Number(clip.durationBeats) || 0)),
  ];
  arrangementTotalBars = Math.max(16, Math.ceil(Math.max(0, ...clipEnds) / beatsPerBar) + 1);
  arrangementStartBar = Math.min(arrangementStartBar, Math.max(1, arrangementTotalBars - arrangementVisibleBars + 1));
  elements.ruler.style.setProperty("--visible-bars", String(arrangementVisibleBars));
  elements.ruler.style.setProperty("--bar-width", `${100 / arrangementVisibleBars}%`);
  elements.lanes.style.setProperty("--visible-bars", String(arrangementVisibleBars));
  elements.lanes.style.setProperty("--bar-width", `${100 / arrangementVisibleBars}%`);
  renderTimelineRuler(beatsPerBar);
  renderArrangementOverview(snapshot, beatsPerBar);
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

  for (const track of snapshot.tracks) {
    const row = document.createElement("div");
    row.className = "track-row";
    row.dataset.trackId = track.id;
    row.style.setProperty("--track-color", track.color);
    const icon = document.createElement("span");
    icon.className = `track-icon ${track.kind}`;
    icon.textContent = track.marker || (track.role === "master" ? "M" : track.role === "bus" ? "B" : track.kind === "audio" ? "◖" : "♫");
    icon.title = track.annotation || track.name;
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
    const identity = createTrackIdentityControl(track);
    if (identity) headingRow.append(identity);
    headingRow.append(name);
    const removeButton = createTrackRemovalButton(track);
    const orderControls = createTrackOrderControls(track, snapshot.tracks);
    if (orderControls) headingRow.append(orderControls);
    if (removeButton) headingRow.append(removeButton);
    row.append(headingRow, details);
    const instrumentControl = createTrackInstrumentControl(track);
    if (instrumentControl) row.append(instrumentControl);
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
      block.style.setProperty("--track-color", track.color);
      block.title = `${clip.name} · ${clip.noteCount} notas`;
      block.tabIndex = 0;
      block.setAttribute("aria-label", `Seleccionar clip MIDI ${clip.name}, ${clip.noteCount} notas. Arrastra para mover.`);
      bindClipSelection(block, clip.id, snapshot);
      const left = Math.max(0, Number(clip.startBeats) || 0);
      const width = Math.max(0.25, Number(clip.durationBeats) || 0.25);
      const viewportBeats = (snapshot.beatsPerBar || 4) * arrangementVisibleBars;
      const offsetBeats = (snapshot.beatsPerBar || 4) * (arrangementStartBar - 1);
      block.style.left = `${(left - offsetBeats) / viewportBeats * 100}%`;
      block.style.width = `${Math.min(width / viewportBeats * 100, 100)}%`;
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
      block.style.setProperty("--track-color", track.color);
      block.dataset.clipId = clip.id;
      if (selectedClipId === clip.id) block.classList.add("is-inspected");
      block.tabIndex = 0;
      block.setAttribute("aria-label", `Seleccionar región de audio ${clip.name}`);
      block.title = `${clip.name} · ${clip.sourceName ?? "fuente"} · ${clip.sampleRateHz} Hz · ${clip.channels} canales · ${clip.durationBeats.toFixed(2)} pulsos. Arrastra para mover; usa los bordes para recortar.`;
      bindClipSelection(block, clip.id, snapshot);
      const left = Math.max(0, Number(clip.startBeats) || 0);
      const width = Math.max(0.25, Number(clip.durationBeats) || 0.25);
      const viewportBeats = (snapshot.beatsPerBar || 4) * arrangementVisibleBars;
      const offsetBeats = (snapshot.beatsPerBar || 4) * (arrangementStartBar - 1);
      block.style.left = `${(left - offsetBeats) / viewportBeats * 100}%`;
      block.style.width = `${Math.min(width / viewportBeats * 100, 100)}%`;
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
  elements.editor.classList.toggle("has-midi-editor", Boolean(midiClip));
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
  const title = document.createElement("div");
  title.className = "clip-view-title";
  const name = document.createElement("strong");
  name.textContent = clip.name;
  const titleType = document.createElement("span");
  titleType.textContent = audioClip ? "VISTA DE CLIP · AUDIO" : "VISTA DE CLIP · MIDI";
  title.append(name, titleType);
  const body = document.createElement("div");
  body.className = `clip-view-body${audioClip ? " audio" : ""}`;
  const properties = document.createElement("aside");
  properties.className = "clip-properties";
  elements.clipInspector.append(title, body);
  body.append(properties);
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
  properties.append(type, trackName, position, length);
  if (audioClip?.sourceName) {
    const source = document.createElement("span");
    source.className = "clip-inspector-source";
    source.textContent = `Fuente: ${audioClip.sourceName}`;
    properties.append(source);
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
    properties.append(
      positionInput,
      gainInput,
      makeInspectorNumber("Entrada (ms)", fadeInMs, 0, Math.max(0, durationMs - fadeOutMs), 1, (value) => saveFades("in", value)),
      makeInspectorNumber("Salida (ms)", fadeOutMs, 0, Math.max(0, durationMs - fadeInMs), 1, (value) => saveFades("out", value)),
    );
  } else if (midiClip) {
    const pianoRoll = document.createElement("div");
    pianoRoll.className = "piano-roll";
    if (track?.color) pianoRoll.style.setProperty("--track-color", track.color);
    const toolbar = document.createElement("div");
    toolbar.className = "piano-roll-toolbar";
    const heading = document.createElement("strong");
    heading.textContent = "Editor MIDI";
    const instruction = document.createElement("span");
    instruction.textContent = "Clic en la rejilla para añadir · arrastra una nota para moverla";
    toolbar.append(heading, instruction);
    pianoRoll.append(toolbar);
    const ruler = document.createElement("div");
    ruler.className = "piano-roll-ruler";
    const rulerSpacer = document.createElement("span");
    rulerSpacer.className = "piano-roll-ruler-key";
    ruler.append(rulerSpacer);
    const rulerCells = document.createElement("div");
    rulerCells.className = "piano-roll-ruler-cells";
    rulerCells.style.setProperty("--steps", String(Math.min(256, Math.max(16, Math.ceil(midiClip.durationTicks / Math.max(1, midiClip.ppq / 2) / 16) * 16))));
    const rulerStepCount = Math.min(256, Math.max(16, Math.ceil(midiClip.durationTicks / Math.max(1, midiClip.ppq / 2) / 16) * 16));
    for (let step = 0; step < rulerStepCount; step += 1) {
      const marker = document.createElement("span");
      marker.className = step % Math.max(1, Math.round((snapshot.beatsPerBar || 4) * 2)) === 0 ? "bar-start" : "";
      if (step % Math.max(1, Math.round((snapshot.beatsPerBar || 4) * 2)) === 0) marker.textContent = String(Math.floor(step / Math.max(1, Math.round((snapshot.beatsPerBar || 4) * 2))) + 1);
      rulerCells.append(marker);
    }
    ruler.append(rulerCells);
    pianoRoll.append(ruler);
    const grid = document.createElement("div");
    grid.className = "piano-roll-grid";
    const stepsPerBar = Math.max(1, Math.round((snapshot.beatsPerBar || 4) * 2));
    const steps = Math.min(256, Math.max(16, Math.ceil(midiClip.durationTicks / Math.max(1, midiClip.ppq / 2) / 16) * 16));
    grid.style.setProperty("--steps", String(steps));
    const saveNoteEdit = async (note, changes) => runCommand("Nota MIDI actualizada", async () => {
      const startTick = changes.startTick ?? note.startTick;
      const durationTicks = changes.durationTicks ?? (note.endTick - note.startTick);
      const key = changes.key ?? note.key;
      const velocity = changes.velocity ?? note.velocity;
      const updated = await platform.updateMidiNote({
        clipId: midiClip.id,
        noteOnIndex: note.noteOnIndex,
        noteOffIndex: note.noteOffIndex,
        expectedStartTick: note.startTick,
        expectedEndTick: note.endTick,
        expectedKey: note.key,
        expectedChannel: note.channel,
        startTick, durationTicks, key, velocity,
      });
      const updatedClip = updated.midiClips.find((clipItem) => clipItem.id === midiClip.id);
      const matches = updatedClip?.notes.filter((item) => item.startTick === startTick && item.endTick === startTick + durationTicks && item.key === key && item.velocity === velocity) ?? [];
      const updatedNote = matches.sort((a, b) => Math.abs(a.noteOnIndex - note.noteOnIndex) - Math.abs(b.noteOnIndex - note.noteOnIndex))[0];
      selectedMidiNote = updatedNote ? { clipId: midiClip.id, ...updatedNote } : null;
      return updated;
    });
    for (let key = 83; key >= 48; key--) {
      const lane = document.createElement("div");
      const blackKey = [1, 3, 6, 8, 10].includes(key % 12);
      lane.className = `piano-roll-row${key % 12 === 0 ? " octave" : ""}${blackKey ? " black-key" : ""}`;
      const label = document.createElement("span");
      label.className = "piano-roll-key";
      label.textContent = key % 12 === 0 ? `C${Math.floor(key / 12) - 1}` : "";
      lane.append(label);
      const cells = document.createElement("div");
      cells.className = "piano-roll-cells";
      for (let stepIndex = 0; stepIndex < steps; stepIndex++) {
        const cell = document.createElement("button");
        cell.type = "button";
        cell.className = `piano-roll-cell${stepIndex % stepsPerBar === 0 ? " bar-start" : ""}${stepIndex % 2 === 0 ? " beat-start" : ""}`;
        cell.title = `Añadir nota ${key} en el pulso ${(stepIndex / 2 + 1).toFixed(1)}`;
        cell.addEventListener("click", () => {
          const startTick = Math.round(stepIndex * midiClip.ppq / 2);
          void runCommand("Nota MIDI añadida", () => platform.addMidiNote({
            clipId: midiClip.id, startTick, durationTicks: Math.max(1, Math.round(midiClip.ppq / 2)), key, velocity: 96,
          }));
        });
        cells.append(cell);
      }
      lane.append(cells);
      grid.append(lane);
    }
    for (const note of midiClip.notes) {
      const row = grid.querySelectorAll(".piano-roll-row")[83 - note.key];
      if (!row) continue;
      const block = document.createElement("span");
      const isSelected = selectedMidiNote?.clipId === midiClip.id
        && selectedMidiNote.noteOnIndex === note.noteOnIndex
        && selectedMidiNote.noteOffIndex === note.noteOffIndex;
      block.className = `piano-roll-note${isSelected ? " selected" : ""}`;
      block.style.left = `calc(34px + ${Math.max(0, note.startBeats * 2 / steps) * 100}%)`;
      block.style.width = `max(8px, ${Math.max(0.008, note.durationBeats * 2 / steps) * 100}%)`;
      block.title = `Nota ${note.key}, velocidad ${note.velocity}`;
      const resizeHandle = document.createElement("span");
      resizeHandle.className = "piano-roll-resize-handle";
      block.append(resizeHandle);
      let suppressNoteClick = false;
      const beginNoteDrag = (event, mode) => {
        if (event.button !== 0) return;
        event.stopPropagation();
        const startX = event.clientX;
        const startY = event.clientY;
        const cellsBounds = cells.getBoundingClientRect();
        const tickPerStep = Math.max(1, Math.round(midiClip.ppq / 2));
        const pixelPerStep = Math.max(1, cellsBounds.width / steps);
        let moved = false;
        const move = (moveEvent) => {
          if (Math.abs(moveEvent.clientX - startX) > 3 || Math.abs(moveEvent.clientY - startY) > 3) moved = true;
        };
        const up = (upEvent) => {
          document.removeEventListener("pointermove", move);
          document.removeEventListener("pointerup", up);
          const horizontalSteps = Math.round((upEvent.clientX - startX) / pixelPerStep);
          const semitones = Math.round((startY - upEvent.clientY) / 14);
          if (!moved) return;
          suppressNoteClick = true;
          const noteLength = note.endTick - note.startTick;
          if (mode === "resize") {
            void saveNoteEdit(note, { durationTicks: Math.max(tickPerStep, noteLength + horizontalSteps * tickPerStep) });
          } else {
            void saveNoteEdit(note, {
              startTick: Math.max(0, note.startTick + horizontalSteps * tickPerStep),
              key: Math.max(0, Math.min(127, note.key + semitones)),
            });
          }
        };
        document.addEventListener("pointermove", move);
        document.addEventListener("pointerup", up);
      };
      block.addEventListener("pointerdown", (event) => beginNoteDrag(event, "move"));
      resizeHandle.addEventListener("pointerdown", (event) => beginNoteDrag(event, "resize"));
      block.addEventListener("click", (event) => {
        event.stopPropagation();
        if (suppressNoteClick) { suppressNoteClick = false; return; }
        selectedMidiNote = { clipId: midiClip.id, ...note };
        renderClipInspector(lastSnapshot);
      });
      row.append(block);
    }
    pianoRoll.append(grid);
    const velocityLane = document.createElement("div");
    velocityLane.className = "piano-roll-velocity";
    const velocityLabel = document.createElement("span");
    velocityLabel.textContent = "VEL";
    const velocityEvents = document.createElement("div");
    velocityEvents.className = "piano-roll-velocity-events";
    for (const note of midiClip.notes) {
      const stem = document.createElement("button");
      stem.type = "button";
      stem.className = `velocity-stem${selectedMidiNote?.noteOnIndex === note.noteOnIndex ? " selected" : ""}`;
      stem.style.left = `${Math.max(0, note.startTick / Math.max(1, midiClip.durationTicks) * 100)}%`;
      stem.style.height = `${Math.max(8, note.velocity / 127 * 100)}%`;
      stem.title = `Velocidad ${note.velocity} · clic para seleccionar nota`;
      stem.setAttribute("aria-label", `Seleccionar nota, velocidad ${note.velocity}`);
      stem.addEventListener("click", () => { selectedMidiNote = { clipId: midiClip.id, ...note }; renderClipInspector(lastSnapshot); });
      velocityEvents.append(stem);
    }
    velocityLane.append(velocityLabel, velocityEvents);
    pianoRoll.append(velocityLane);
    const selected = midiClip.notes.find((note) => selectedMidiNote?.clipId === midiClip.id
      && selectedMidiNote.noteOnIndex === note.noteOnIndex
      && selectedMidiNote.noteOffIndex === note.noteOffIndex
      && selectedMidiNote.startTick === note.startTick
      && selectedMidiNote.key === note.key);
    if (selected) {
      const editor = document.createElement("div");
      editor.className = "piano-roll-note-editor";
      const start = makeInspectorNumber("Inicio (ticks)", selected.startTick, 0, null, 1, () => {});
      const duration = makeInspectorNumber("Duración (ticks)", selected.endTick - selected.startTick, 1, null, 1, () => {});
      const key = makeInspectorNumber("Tono MIDI", selected.key, 0, 127, 1, () => {});
      const velocity = makeInspectorNumber("Velocidad", selected.velocity, 1, 127, 1, () => {});
      const apply = document.createElement("button");
      apply.type = "button";
      apply.className = "button button-accent";
      apply.textContent = "Aplicar nota";
      apply.addEventListener("click", async () => {
        const values = [start, duration, key, velocity].map((field) => Number(field.querySelector("input").value));
        if (values.some((value) => !Number.isInteger(value)) || values[0] < 0 || values[1] < 1 || values[2] > 127 || values[3] < 1 || values[3] > 127) return;
        const [startTick, durationTicks, nextKey, nextVelocity] = values;
        await runCommand("Nota MIDI actualizada", async () => {
          const command = {
            clipId: midiClip.id,
            noteOnIndex: selected.noteOnIndex,
            noteOffIndex: selected.noteOffIndex,
            expectedStartTick: selected.startTick,
            expectedEndTick: selected.endTick,
            expectedKey: selected.key,
            expectedChannel: selected.channel,
            startTick, durationTicks, key: nextKey, velocity: nextVelocity,
          };
          const updated = await platform.updateMidiNote(command);
          const updatedClip = updated.midiClips.find((clipItem) => clipItem.id === midiClip.id);
          const updatedNote = updatedClip?.notes.find((item) => item.startTick === startTick && item.endTick === startTick + durationTicks && item.key === nextKey && item.velocity === nextVelocity);
          selectedMidiNote = updatedNote ? { clipId: midiClip.id, ...updatedNote } : null;
          return updated;
        });
      });
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "button";
      remove.textContent = "Eliminar nota";
      remove.addEventListener("click", () => {
        selectedMidiNote = null;
        void runCommand("Nota MIDI eliminada", () => platform.removeMidiNote({
          clipId: midiClip.id,
          noteOnIndex: selected.noteOnIndex,
          noteOffIndex: selected.noteOffIndex,
          expectedStartTick: selected.startTick,
          expectedEndTick: selected.endTick,
          expectedKey: selected.key,
          expectedChannel: selected.channel,
        }));
      });
      editor.append(start, duration, key, velocity, apply, remove);
      pianoRoll.append(editor);
    }
    body.append(pianoRoll);
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
  elements.browserMidiList.replaceChildren();
  elements.browserInstrumentList.replaceChildren();
  const contentQuery = elements.browserContentSearch.value.trim().toLocaleLowerCase();
  const midiTracks = snapshot.tracks.filter((track) => track.kind === "midi" && track.role !== "master");
  elements.browserMidiContent.hidden = snapshot.midiClips.length === 0;
  const midiMatches = snapshot.midiClips.filter((clip) => {
    const track = midiTracks.find((item) => item.id === clip.trackId);
    return `${clip.name} ${track?.name ?? ""}`.toLocaleLowerCase().includes(contentQuery);
  });
  for (const clip of midiMatches) {
    const track = midiTracks.find((item) => item.id === clip.trackId);
    const button = document.createElement("button");
    button.type = "button";
    button.className = "browser-content-item";
    button.textContent = `${clip.name} · ${track?.name ?? "Pista desconocida"}`;
    button.title = `${clip.name} · ${clip.noteCount} notas; seleccionar y enfocar en Arreglo`;
    button.addEventListener("click", () => {
      selectedClipId = clip.id;
      selectedDetailTab = "clip";
      selectSurface("arrangement");
      renderClipInspector(snapshot);
      updateWorkspaceLayout();
      const target = elements.lanes.querySelector(`[data-clip-id="${CSS.escape(clip.id)}"]`);
      target?.classList.add("is-inspected");
      target?.scrollIntoView({ block: "nearest", inline: "nearest" });
    });
    elements.browserMidiList.append(button);
  }
  if (!midiMatches.length) {
    const empty = document.createElement("span");
    empty.className = "browser-media-empty";
    empty.textContent = snapshot.midiClips.length ? "No hay clips MIDI que coincidan." : "Sin clips MIDI.";
    elements.browserMidiList.append(empty);
  }
  const instrumentTracks = midiTracks.filter((track) => track.instrument);
  elements.browserInstrumentContent.hidden = instrumentTracks.length === 0;
  const instrumentMatches = instrumentTracks.filter((item) => `${item.name} ${item.instrument?.plugin?.path ?? item.instrument?.backend ?? ""}`.toLocaleLowerCase().includes(contentQuery));
  for (const track of instrumentMatches) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "browser-content-item";
    const instrument = track.instrument;
    button.textContent = `${track.name} · ${instrument.backend === "vst3" ? vst3PluginLabel(instrument.plugin?.path) : instrument.backend ?? "Instrumento asignado"}`;
    button.title = "Abrir el instrumento asignado a esta pista";
    button.addEventListener("click", () => {
      selectedTrackIds = new Set([track.id]);
      selectedDetailTab = "device";
      syncTrackSelectionUi();
      renderDeviceInspector(snapshot);
      updateWorkspaceLayout();
      selectSurface("arrangement");
      document.querySelector(`.track-row[data-track-id="${CSS.escape(track.id)}"]`)?.scrollIntoView({ block: "nearest" });
    });
    elements.browserInstrumentList.append(button);
  }
  if (!instrumentMatches.length && instrumentTracks.length) {
    const empty = document.createElement("span");
    empty.className = "browser-media-empty";
    empty.textContent = "No hay instrumentos que coincidan con la búsqueda.";
    elements.browserInstrumentList.append(empty);
  }
  const clips = snapshot.audioClips ?? [];
  elements.browserAudioContent.hidden = clips.length === 0;
  elements.browserLibraryEmpty.hidden = snapshot.midiClips.length + instrumentTracks.length + clips.length > 0;
  elements.browserMediaTools.hidden = clips.length === 0;
  const query = contentQuery;
  const channelFilter = elements.browserMediaChannels.value;
  const matching = clips.filter((clip) => {
    const trackName = snapshot.tracks.find((track) => track.id === clip.trackId)?.name ?? "";
    const searchable = `${clip.name} ${clip.sourceName ?? ""} ${trackName} ${clip.sampleRateHz} ${clip.channels}`.toLocaleLowerCase();
    const channelMatch = channelFilter === "all"
      || (channelFilter === "multi" ? clip.channels > 2 : clip.channels === Number(channelFilter));
    return channelMatch && (!query || searchable.includes(query));
  });
  elements.browserMediaChannels.disabled = clips.length === 0;
  if (!matching.length) {
    const empty = document.createElement("span");
    empty.className = "browser-media-empty";
    empty.textContent = clips.length ? "No hay audio que coincida con la búsqueda." : "Sin audio importado.";
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
      selectedDetailTab = "clip";
      selectSurface("arrangement");
      renderClipInspector(snapshot);
      updateWorkspaceLayout();
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
    selectedDetailTab = "clip";
    for (const selected of elements.lanes.querySelectorAll(".is-inspected")) {
      selected.classList.remove("is-inspected");
    }
    renderClipInspector(snapshot);
    updateWorkspaceLayout();
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
  const timelineBeats = Math.max(1, Number(beatsPerBar) || 4) * arrangementVisibleBars;
  const offsetBeats = Math.max(1, Number(beatsPerBar) || 4) * (arrangementStartBar - 1);
  const totalBeats = Math.max(1, Number(beatsPerBar) || 4) * arrangementTotalBars;
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
    const left = Math.max(0, Math.min(totalBeats - duration, originalLeft + deltaBeats));
    block.style.left = `${(left - offsetBeats) / timelineBeats * 100}%`;
  });

  const finish = async (event) => {
    if (!gesture || gesture.pointerId !== event.pointerId) return;
    const current = gesture;
    gesture = null;
    block.classList.remove("is-editing");
    const restore = () => { block.style.left = `${(originalLeft - offsetBeats) / timelineBeats * 100}%`; };
    if (event.type === "pointercancel" || Math.abs(event.clientX - current.startX) < 2) {
      restore();
      return;
    }
    const rawBeats = Math.max(0, Math.min(totalBeats - duration,
      originalLeft + (event.clientX - current.startX) / current.laneWidth * timelineBeats));
    const snapValue = elements.gridSnap.value;
    const snapBeats = snapValue === "bar"
      ? Math.max(1, Number(beatsPerBar) || 4)
      : Number(snapValue);
    const snappedBeats = snapBeats > 0 ? Math.round(rawBeats / snapBeats) * snapBeats : rawBeats;
    const startTick = Math.round(Math.max(0, Math.min(totalBeats - duration, snappedBeats)) * ppq);
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
  const timelineBeats = Math.max(1, Number(beatsPerBar) || 4) * arrangementVisibleBars;
  const offsetBeats = Math.max(1, Number(beatsPerBar) || 4) * (arrangementStartBar - 1);
  const totalBeats = Math.max(1, Number(beatsPerBar) || 4) * arrangementTotalBars;
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
      const originalLeftPercent = (originalLeft - offsetBeats) / timelineBeats * 100;
      const widthPercent = originalWidth / timelineBeats * 100;
      block.style.left = `${Math.max(-widthPercent, Math.min(100, originalLeftPercent + deltaPercent))}%`;
    } else if (gesture.edge === "left") {
      const trim = Math.max(0, Math.min(deltaPercent, originalWidth / timelineBeats * 100 - 0.3));
      block.style.left = `${(originalLeft - offsetBeats) / timelineBeats * 100 + trim}%`;
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
      block.style.left = `${(originalLeft - offsetBeats) / timelineBeats * 100}%`;
      block.style.width = `${originalWidth / timelineBeats * 100}%`;
      return;
    }
    const delta = event.clientX - current.startX;
    if (Math.abs(delta) < 2) {
      block.style.left = `${(originalLeft - offsetBeats) / timelineBeats * 100}%`;
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
        startTick: Math.round(Math.max(0, Math.min(totalBeats - originalWidth, snappedStart)) * 480),
      };
    } else {
      const rawTrimBeats = current.edge === "left" ? Math.max(0, deltaBeats) : Math.max(0, -deltaBeats);
      const trimBeats = snapBeats > 0 ? Math.round(rawTrimBeats / snapBeats) * snapBeats : rawTrimBeats;
      if (trimBeats === 0) {
        block.style.left = `${(originalLeft - offsetBeats) / timelineBeats * 100}%`;
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
      setNotice("Audio importado", "La fuente y región se registraron; el Arreglo muestra su forma de onda y Play reproduce el audio junto con el proyecto.");
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
  const controls = document.createElement("div");
  controls.className = "timeline-ruler-controls";
  const zoomOut = document.createElement("button");
  zoomOut.type = "button";
  zoomOut.textContent = "−";
  zoomOut.title = "Mostrar más compases";
  zoomOut.setAttribute("aria-label", zoomOut.title);
  zoomOut.disabled = arrangementVisibleBars >= arrangementTotalBars;
  zoomOut.addEventListener("click", () => setArrangementZoom(Math.min(arrangementTotalBars, arrangementVisibleBars * 2)));
  const readout = document.createElement("span");
  readout.textContent = `${arrangementVisibleBars} compases`;
  const zoomIn = document.createElement("button");
  zoomIn.type = "button";
  zoomIn.textContent = "+";
  zoomIn.title = "Ampliar compases";
  zoomIn.setAttribute("aria-label", zoomIn.title);
  zoomIn.disabled = arrangementVisibleBars <= 4;
  zoomIn.addEventListener("click", () => setArrangementZoom(Math.max(4, Math.floor(arrangementVisibleBars / 2))));
  controls.append(zoomOut, readout, zoomIn);
  elements.ruler.append(controls);
  for (let offset = 0; offset < arrangementVisibleBars; offset += 1) {
    const tick = document.createElement("span");
    tick.className = "bar-tick";
    tick.textContent = String(arrangementStartBar + offset);
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

function setArrangementZoom(bars) {
  arrangementVisibleBars = Math.max(4, Math.min(arrangementTotalBars, Math.round(bars)));
  arrangementStartBar = Math.min(arrangementStartBar, Math.max(1, arrangementTotalBars - arrangementVisibleBars + 1));
  if (lastSnapshot) renderSnapshot(lastSnapshot);
}

function renderArrangementOverview(snapshot, beatsPerBar) {
  const overview = elements.overview;
  overview.replaceChildren();
  for (const track of snapshot.tracks) {
    const row = document.createElement("div");
    row.className = "overview-track";
    row.style.setProperty("--track-color", track.color);
    const clips = [
      ...snapshot.midiClips.filter((clip) => clip.trackId === track.id).map((clip) => ({ ...clip, kind: "midi" })),
      ...(snapshot.audioClips ?? []).filter((clip) => clip.trackId === track.id).map((clip) => ({ ...clip, kind: "audio" })),
    ];
    for (const clip of clips) {
      const mark = document.createElement("span");
      mark.className = `overview-clip${clip.kind === "audio" ? " audio" : ""}`;
      const startBar = (Number(clip.startBeats) || 0) / beatsPerBar;
      const durationBars = Math.max(0.02, (Number(clip.durationBeats) || 0) / beatsPerBar);
      mark.style.left = `${startBar / arrangementTotalBars * 100}%`;
      mark.style.width = `${Math.max(.35, durationBars / arrangementTotalBars * 100)}%`;
      row.append(mark);
    }
    overview.append(row);
  }
  const viewport = document.createElement("button");
  viewport.type = "button";
  viewport.className = "overview-window";
  viewport.title = "Arrastra para desplazarte por el arreglo; pulsa para centrar la vista";
  viewport.setAttribute("aria-label", viewport.title);
  const updateWindow = () => {
    viewport.style.left = `${(arrangementStartBar - 1) / arrangementTotalBars * 100}%`;
    viewport.style.width = `${Math.min(100, arrangementVisibleBars / arrangementTotalBars * 100)}%`;
  };
  updateWindow();
  let drag = null;
  viewport.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) return;
    drag = { x: event.clientX, start: arrangementStartBar };
    viewport.setPointerCapture(event.pointerId);
    event.preventDefault();
  });
  viewport.addEventListener("pointermove", (event) => {
    if (!drag) return;
    const width = overview.getBoundingClientRect().width;
    const deltaBars = width > 0 ? (event.clientX - drag.x) / width * arrangementTotalBars : 0;
    const maxStart = Math.max(1, arrangementTotalBars - arrangementVisibleBars + 1);
    arrangementStartBar = Math.max(1, Math.min(maxStart, drag.start + Math.round(deltaBars)));
    updateWindow();
  });
  viewport.addEventListener("pointerup", () => { if (drag) { drag = null; renderSnapshot(lastSnapshot); } });
  viewport.addEventListener("pointercancel", () => { drag = null; renderSnapshot(lastSnapshot); });
  overview.append(viewport);
  overview.addEventListener("pointerdown", (event) => {
    if (event.target === viewport || event.button !== 0) return;
    const bounds = overview.getBoundingClientRect();
    const bar = Math.max(1, Math.min(arrangementTotalBars, Math.floor((event.clientX - bounds.left) / bounds.width * arrangementTotalBars) + 1));
    arrangementStartBar = Math.max(1, Math.min(arrangementTotalBars - arrangementVisibleBars + 1, bar - Math.floor(arrangementVisibleBars / 2)));
    renderSnapshot(lastSnapshot);
  });
}

function renderTransportPosition(ticks) {
  transportPositionTick = Math.max(0, Number(ticks) || 0);
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const ticksPerBar = beatsPerBar * 960;
  const timelineTicks = ticksPerBar * arrangementVisibleBars;
  const offsetTicks = ticksPerBar * (arrangementStartBar - 1);
  const left = `${Math.max(0, Math.min(100, (transportPositionTick - offsetTicks) / timelineTicks * 100))}%`;
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
  const timelineTicks = beatsPerBar * arrangementVisibleBars * 480;
  const offsetTicks = beatsPerBar * (arrangementStartBar - 1) * 480;
  const left = `${Math.max(0, Math.min(100, (editCursorTick - offsetTicks) / timelineTicks * 100))}%`;
  document.querySelectorAll(".edit-cursor").forEach((cursor) => { cursor.style.left = left; });
  const bar = Math.floor(editCursorTick / (beatsPerBar * 480)) + 1;
  const beat = ((editCursorTick % (beatsPerBar * 480)) / 480) + 1;
  elements.editCursorPosition.textContent = `Cursor: compás ${bar} · pulso ${beat.toLocaleString("es-CL", { maximumFractionDigits: 2 })}`;
  elements.importBar.value = String(bar);
}

function setEditCursorFromX(clientX, bounds) {
  if (bounds.width <= 0) return;
  const beatsPerBar = Number(elements.ruler.dataset.beatsPerBar) || 4;
  const timelineTicks = beatsPerBar * arrangementVisibleBars * 480;
  const offsetTicks = beatsPerBar * (arrangementStartBar - 1) * 480;
  const fraction = Math.max(0, Math.min(1, (clientX - bounds.left) / bounds.width));
  editCursorTick = offsetTicks + Math.round(fraction * timelineTicks / 120) * 120;
  renderEditCursor();
}

async function runCommand(title, operation) {
  try {
    const snapshot = await operation();
    if (!snapshot) return;
    renderSnapshot(snapshot);
    setNotice(title, "Cambios aplicados al estado del proyecto.");
    return true;
  } catch (error) {
    setNotice("La operación falló", String(error));
    return false;
  }
}

elements.open.addEventListener("click", async () => {
  await whileBusy([elements.open], async () => { try {
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
elements.browserContentSearch.addEventListener("input", () => {
  if (lastSnapshot) renderProjectMedia(lastSnapshot);
});
elements.browserMediaChannels.addEventListener("change", () => {
  if (lastSnapshot) renderProjectMedia(lastSnapshot);
});
for (const tab of document.querySelectorAll(".browser-tab")) {
  tab.addEventListener("click", () => {
    const selectedPanel = tab.getAttribute("aria-controls");
    for (const peer of document.querySelectorAll(".browser-tab")) {
      const selected = peer === tab;
      peer.classList.toggle("is-selected", selected);
      peer.setAttribute("aria-selected", String(selected));
    }
    for (const panel of document.querySelectorAll(".browser-panel")) {
      panel.hidden = panel.id !== selectedPanel;
    }
  });
}
elements.showArrangement.addEventListener("click", () => selectSurface("arrangement"));
elements.showSession.addEventListener("click", () => selectSurface("session"));
elements.showMixer.addEventListener("click", () => {
  mixerPanelVisible = !mixerPanelVisible;
  updateWorkspaceLayout();
});
elements.detailClipTab.addEventListener("click", () => {
  selectedDetailTab = "clip";
  updateWorkspaceLayout();
});
elements.detailDeviceTab.addEventListener("click", () => {
  selectedDetailTab = "device";
  if (lastSnapshot) renderDeviceInspector(lastSnapshot);
  updateWorkspaceLayout();
});
elements.toggleBrowser.addEventListener("click", () => {
  browserVisible = !browserVisible;
  updateWorkspaceLayout();
});
elements.toggleClipDetail.addEventListener("click", () => {
  if (mixerPanelVisible) {
    mixerPanelVisible = false;
    clipDetailVisible = true;
    updateWorkspaceLayout();
    return;
  }
  clipDetailVisible = !clipDetailVisible;
  updateWorkspaceLayout();
});
elements.returnToArrangement.addEventListener("click", returnToArrangement);
elements.gridSnap.addEventListener("change", () => {
  const label = elements.gridSnap.selectedOptions[0]?.textContent ?? "rejilla";
  setNotice("Ajuste actualizado", `El movimiento y recorte de regiones de audio usarán ${label}.`);
});
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
  await whileBusy([elements.demoProject], async () => { try {
    const snapshot = await platform.demoMidiProject();
    renderSnapshot(snapshot);
    setNotice("Demo MIDI lista", "Siete notas están preparadas en la pista. Pulsa Play para oírlas por la salida configurada.");
  } catch (error) {
    setNotice("No se pudo preparar la demo MIDI", String(error));
  } });
});

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
  const priorAudioCount = Number(lastSnapshot?.audioClipCount) || 0;
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
    const launches = await platform.sessionLaunches();
    const changed = JSON.stringify(launches) !== JSON.stringify(sessionLaunches);
    sessionLaunches = launches;
    if (changed) updateReturnToArrangementButton();
    if (changed && lastSnapshot) renderSnapshot(lastSnapshot);
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
document.addEventListener("keydown", handleCreativeWorkspaceShortcut);
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

document.querySelector(".panel-grip").addEventListener("pointerdown", (event) => {
  if (!elements.editor.classList.contains("has-midi-editor")) return;
  event.preventDefault();
  const startY = event.clientY;
  const startHeight = document.querySelector(".lower-panel").getBoundingClientRect().height;
  const editorHeight = elements.editor.getBoundingClientRect().height;
  const minHeight = editorHeight <= 680 ? 190 : 330;
  const maxHeight = Math.max(minHeight, editorHeight - 240);
  const resize = (moveEvent) => {
    const height = Math.max(minHeight, Math.min(maxHeight, startHeight + startY - moveEvent.clientY));
    elements.editor.style.setProperty("--detail-height", `${height}px`);
  };
  const finish = () => {
    document.removeEventListener("pointermove", resize);
    document.removeEventListener("pointerup", finish);
  };
  document.addEventListener("pointermove", resize);
  document.addEventListener("pointerup", finish);
});
document.querySelector(".panel-grip").addEventListener("keydown", (event) => {
  if (!elements.editor.classList.contains("has-midi-editor") || !["ArrowUp", "ArrowDown"].includes(event.key)) return;
  event.preventDefault();
  const current = document.querySelector(".lower-panel").getBoundingClientRect().height;
  const minHeight = elements.editor.clientHeight <= 680 ? 190 : 330;
  const next = current + (event.key === "ArrowUp" ? 24 : -24);
  elements.editor.style.setProperty("--detail-height", `${Math.max(minHeight, Math.min(elements.editor.clientHeight - 240, next))}px`);
});

// Este shell inicial sólo resume datos compactos; jamás solicita PCM o buffers
// GPU al core a través del bridge.
setProjectEnabled(false);
elements.undo.disabled = true;
elements.redo.disabled = true;
renderTimelineRuler(4);
initializeUiZoom();
loadAudioSettings();
