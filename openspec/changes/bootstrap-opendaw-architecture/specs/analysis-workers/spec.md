# Workers de análisis e IPC

## Propósito

Ejecutar separación de stems, transcripción, pitch, acordes, estructura, balance espectral y generación de partituras fuera del hilo de audio. Rust conserva la autoridad sobre el proyecto y Python aporta el ecosistema de modelos sin bloquear la reproducción.

## Transporte de audio sin serialización

- `analysis-protocol` nunca serializa audio multicanal grande como JSON, base64 ni mensajes completos.
- Rust publica un `ArtifactRef` con versión, ruta o descriptor de archivo, formato, sample rate, canales, frames, rango de samples, permisos y checksum opcional.
- El worker recibe una referencia a archivo/descriptor, no un puntero de memoria de otro proceso. Para archivos persistentes usa `mmap`/`numpy.memmap`; para datos efímeros puede usar memoria compartida POSIX (`shm_open`) con dueño, tamaño, ciclo de vida y limpieza explícitos.
- Los stems, tomas y proxies se escriben como artefactos versionados y el IPC sólo transmite metadatos y rangos de lectura.
- Los workers devuelven referencias a nuevos artefactos, no buffers de audio embebidos en el mensaje.

## Sistema de proxies

- La generación, regeneración, validación y transcodificación de `ProxyAsset` son jobs de baja prioridad fuera del callback.
- Cada proxy declara hash de fuente, perfil de calidad, encoder, duración, timebase, canales y relación con el `MediaSource` original.
- Los workers pueden analizar el proxy para obtener resultados rápidos, pero el manifiesto conserva la procedencia y permite invalidar resultados cuando cambia la fuente.
- La caché de proxies tiene límites, limpieza segura y estados `ready`, `stale`, `missing` y `failed`; nunca borra originales.

## Ciclo de vida y resiliencia

- Un job tiene estados `queued`, `running`, `progress`, `canceling`, `completed`, `failed`, `canceled` y `oom`.
- El supervisor inicia procesos, limita recursos, recibe heartbeat/progreso y detecta salida anormal. En Unix registra señal, código de salida y diagnóstico de OOM.
- Un fallo de Demucs, Whisper u otro modelo nunca detiene el motor de audio ni invalida la última versión reproducible del proyecto.
- El diagnóstico incluye job id, etapa, modelo, memoria estimada, señal/código, artefactos parciales y recomendación de reintento. Los reintentos pueden reducir batch size, usar CPU o desactivar GPU.
- La cancelación libera memoria compartida y deja artefactos parciales marcados como no finales.

## Manifiesto simbólico de producción

Los workers convierten los resultados acústicos en un manifiesto compacto antes de consultar a una LLM. La LLM recibe estructura musical y métricas interpretables, no audio crudo salvo una operación explícita.

```json
{
  "schema_version": "production-manifest.v1",
  "tempo": 92,
  "key": "E",
  "mode": "natural_minor",
  "time_signature": "4/4",
  "structure": [
    {"section": "verse_1", "bars": "1-8", "chords": ["Em", "C", "G", "D"], "energy_lufs": -18.2},
    {"section": "chorus_1", "bars": "9-16", "chords": ["C", "D", "Em", "Bm"], "energy_lufs": -14.1}
  ],
  "vocal_pitch_analysis": {"average_drift_cents": -14.2, "tendency": "flat_on_long_chorus_notes", "vibrato_rate_hz": 5.2},
  "guitar_spectral_balance": {"mud_region_300hz_db": 3.5, "presence_3khz_db": -2.0},
  "confidence": 0.87,
  "provenance": {"job_ids": ["job-123"], "models": ["model-hash"]}
}
```

El manifiesto puede crecer con letra alineada, melodía, dinámica, stems, afinación, dicción, ruido, atmósferas y relaciones de mezcla. Todo valor relevante lleva unidades, rango temporal y confianza.

## Requisitos verificables

- **Dado** un WAV multicanal grande, **cuando** se crea un job, **entonces** el mensaje contiene referencias/rangos y no audio serializado.
- **Dado** un proxy vigente, **cuando** se solicita un análisis rápido, **entonces** el worker puede leerlo por mmap y conserva su procedencia.
- **Dado** un worker que termina por OOM, **entonces** el supervisor publica `oom`, conserva la reproducción y deja un diagnóstico accionable.
- **Dado** un análisis completado, **entonces** existe un manifiesto versionado con timestamps, confianza y procedencia.
- **Dado** una consulta del profesor IA, **entonces** el payload por defecto es el manifiesto simbólico y la respuesta propuesta se traduce a un changeset con preview/undo.

## ADDED Requirements

### Requirement: intercambio de audio por referencias versionadas
El protocolo entre Rust y los workers MUST transportar referencias a artefactos
y rangos de samples, no audio voluminoso serializado en mensajes.

#### Scenario: análisis de audio multicanal
- WHEN Rust solicita analizar una toma de audio
- THEN el worker recibe una referencia verificable al archivo o memoria compartida y el rango solicitado.

### Requirement: aislamiento de fallos de workers
Un fallo, cancelación u OOM de un worker MUST conservar la reproducción y el
último estado reproducible del proyecto.

#### Scenario: worker termina por OOM
- WHEN el supervisor detecta que el proceso de análisis terminó por presión de memoria
- THEN publica el diagnóstico y el estado del job sin detener el motor de audio.

### Requirement: contexto simbólico para el profesor musical
Las consultas de asesoría MUST usar por defecto resultados simbólicos con
unidades, rangos temporales, confianza y procedencia, no audio crudo.

#### Scenario: consulta de producción
- WHEN el usuario solicita consejo sobre una canción analizada
- THEN el profesor recibe el manifiesto musical y cualquier cambio sugerido requiere preview y aceptación.
