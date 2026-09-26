const platform = window.estudioPlatform;

const elements = {
  open: document.querySelector("#open-project"),
  save: document.querySelector("#save-project"),
  saveAs: document.querySelector("#save-project-as"),
  path: document.querySelector("#project-path"),
  name: document.querySelector("#project-name"),
  tempo: document.querySelector("#tempo"),
  transport: document.querySelector("#transport-state"),
  tracks: document.querySelector("#track-list"),
  trackCount: document.querySelector("#track-count"),
  midiCount: document.querySelector("#midi-count"),
  audioCount: document.querySelector("#audio-count"),
  revision: document.querySelector("#revision"),
  engine: document.querySelector("#engine-status"),
  noticeTitle: document.querySelector("#notice-title"),
  noticeText: document.querySelector("#notice-text"),
  play: document.querySelector("#play"),
  pause: document.querySelector("#pause"),
  stop: document.querySelector("#stop"),
  undo: document.querySelector("#undo"),
  redo: document.querySelector("#redo"),
};

let hasProject = false;

function setNotice(title, text) {
  elements.noticeTitle.textContent = title;
  elements.noticeText.textContent = text;
}

function setProjectEnabled(enabled) {
  hasProject = enabled;
  for (const button of [elements.save, elements.saveAs, elements.play, elements.pause, elements.stop]) {
    button.disabled = !enabled;
  }
}

function renderSnapshot(snapshot) {
  setProjectEnabled(true);
  elements.path.textContent = snapshot.projectPath ?? "Proyecto sin ruta";
  elements.path.title = snapshot.projectPath ?? "";
  elements.name.textContent = snapshot.projectId;
  elements.tempo.textContent = Number(snapshot.tempoBpm).toFixed(1);
  elements.transport.textContent = snapshot.transportState.toUpperCase();
  elements.trackCount.textContent = snapshot.trackCount;
  elements.midiCount.textContent = snapshot.midiClipCount;
  elements.audioCount.textContent = snapshot.audioClipCount;
  elements.revision.textContent = `REV ${snapshot.projectRevision}`;
  elements.engine.textContent = snapshot.audioEngineConnected
    ? "Core listo · motor de audio conectado"
    : "Core listo · motor de audio aún no conectado";
  elements.undo.disabled = !snapshot.canUndo;
  elements.redo.disabled = !snapshot.canRedo;

  elements.tracks.replaceChildren();
  if (snapshot.tracks.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.textContent = "El proyecto todavía no tiene pistas.";
    elements.tracks.append(empty);
    return;
  }

  for (const track of snapshot.tracks) {
    const row = document.createElement("div");
    row.className = "track-row";
    const name = document.createElement("div");
    name.className = "track-name";
    const icon = document.createElement("span");
    icon.className = `track-icon ${track.kind}`;
    icon.textContent = track.kind === "audio" ? "◖" : "♫";
    const label = document.createElement("span");
    label.textContent = track.name;
    name.append(icon, label);
    const kind = document.createElement("span");
    kind.className = "track-kind";
    kind.textContent = track.kind === "audio" ? "Audio" : "MIDI";
    const notes = document.createElement("span");
    notes.className = "track-notes";
    notes.textContent = track.noteCount;
    const state = document.createElement("span");
    state.className = "track-badge";
    state.textContent = "Preparada";
    row.append(name, kind, notes, state);
    elements.tracks.append(row);
  }
}

async function runCommand(title, operation) {
  try {
    renderSnapshot(await operation());
    setNotice(title, "Cambios aplicados al estado del proyecto.");
  } catch (error) {
    setNotice("La operación falló", String(error));
  }
}

elements.open.addEventListener("click", async () => {
  try {
    const snapshot = await platform.openProject();
    if (!snapshot) return;
    renderSnapshot(snapshot);
    setNotice("Proyecto abierto", "El estado se carga a través de ProjectApplication.");
  } catch (error) {
    setNotice("No se pudo abrir el proyecto", String(error));
  }
});

elements.save.addEventListener("click", () => runCommand("Proyecto guardado", () => platform.saveProject()));
elements.saveAs.addEventListener("click", () => runCommand("Copia del proyecto guardada", () => platform.saveProjectAs()));
elements.play.addEventListener("click", () => runCommand("Transporte en Play", () => platform.setTransport("play")));
elements.pause.addEventListener("click", () => runCommand("Transporte pausado", () => platform.setTransport("pause")));
elements.stop.addEventListener("click", () => runCommand("Transporte detenido", () => platform.setTransport("stop")));
elements.undo.addEventListener("click", () => runCommand("Undo aplicado", () => platform.historyAction("undo")));
elements.redo.addEventListener("click", () => runCommand("Redo aplicado", () => platform.historyAction("redo")));

// Este shell inicial sólo resume datos compactos; jamás solicita PCM o buffers
// GPU al core a través del bridge.
setProjectEnabled(false);
elements.undo.disabled = true;
elements.redo.disabled = true;
