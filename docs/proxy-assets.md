# MediaSource y proxies

El proyecto conserva siempre el archivo original y trata el proxy como una
representación derivada y reemplazable. `MediaSource` registra la ruta original,
su firma de procedencia y un `ProxyAsset` opcional con perfil y firma de la
fuente que lo generó.

## Resolución

`ProxyPolicy` define la intención del consumidor:

- `Original`: exige que exista el original.
- `Proxy`: exige un proxy existente cuya firma coincida con la fuente.
- `Auto`: usa el proxy válido y vuelve al original cuando todavía no hay proxy.

`MediaSource::proxy_cache_state()` expone el estado observable de la caché:
`Missing`, `Building`, `Ready` o `Stale`. Los jobs crean un marcador
`.building` con `create_new`, por lo que dos procesos no pueden generar el
mismo proxy simultáneamente.

`ProxyCacheManager` calcula rutas deterministas usando nombre sanitizado,
perfil y prefijo SHA-256. `ensure_audio_proxy()` reutiliza entradas `Ready` y
sólo genera una nueva entrada para estados `Missing` o `Stale`; el modelo no se
actualiza hasta que el archivo fue validado y publicado.

El manifiesto `proxy-manifest.json` se escribe con el mismo patrón temporal y
rename atómico. `hydrate_source()` permite reconstruir el `ProxyAsset` después
de reiniciar el DAW y vuelve a clasificarlo como `Ready` o `Stale` según la
fuente actual.

Las pistas de audio contienen ahora un `media_source` opcional dentro del
modelo `Track`. `attach_media_source()` sólo permite asociarlo a pistas de
audio; las pistas MIDI permanecen libres de rutas y metadatos de medios.

`AudioClip` representa una región no destructiva: su inicio se expresa en
ticks musicales y su recorte de fuente en samples. Esto permite cambiar el
tempo del proyecto sin perder la precisión del material grabado.

El binario `estudio-daw-project` es un adaptador de laboratorio, no la
arquitectura de la UI. Las operaciones de asociación, caché y edición viven en
Rust y la futura interfaz las invocará mediante un `Application/Command API`.

El flujo CLI completo conserva cada etapa en un archivo nuevo:

```bash
cargo run -q -p estudio-daw-project-model --bin estudio-daw-project -- \
  attach-media proyecto.json track-audio toma.wav proyecto-con-audio.json

cargo run -q -p estudio-daw-project-model --bin estudio-daw-project -- \
  proxy-track proyecto-con-audio.json track-audio .cache/proxies proyecto-final.json
```

La resolución sólo selecciona una ruta; no copia, modifica ni re-encodea ningún
archivo. Si el original cambió, la firma del proxy deja de coincidir y el
proxy se considera obsoleto. La firma inicial es una comprobación barata de
tamaño y fecha de modificación; el job de generación añadirá hash de contenido
para proyectos que necesiten invalidación fuerte.

## Frontera futura

`generate_proxy()` ya implementa la primera versión del `ProxyJob`: calcula
SHA-256, copia a un temporal, valida que la fuente no haya cambiado, verifica
la salida y publica un nuevo `ProxyAsset` mediante rename atómico. El backend
actual es un materializador de identidad para probar la frontera; los
transcoders de audio reducirán tamaño y sample rate detrás del mismo contrato.

Para audio, `proxy-audio` usa el perfil `audio-opus-preview-v1`, transcodifica
con ffmpeg a Opus/Ogg, valida con ffprobe que la salida sea estéreo a 48 kHz y
publica el archivo sólo después de esa validación:

```bash
cargo run -q -p estudio-daw-project-model --bin estudio-daw-project -- \
  proxy-audio /ruta/a/mi-cancion.wav cache/mi-cancion-preview.ogg
```

El editor podrá seguir usando la representación anterior mientras el job
trabaja, y el render final resolverá explícitamente `Original`.
