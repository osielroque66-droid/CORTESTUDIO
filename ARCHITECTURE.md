# Arquitectura

Resultado de la auditoría (Fase 0) del repositorio Concat 0.2.11
(commit `a7cda105`, 1 525 commits, licencia AGPL-3.0-or-later) más los
módulos añadidos en este proyecto. La referencia de bajo nivel del motor
original sigue siendo [`src/README.md`](src/README.md).

## Tecnologías

| Capa | Tecnología | Notas |
|---|---|---|
| Lenguaje | Rust 1.93.1 (edición 2024) | Un solo workspace de Cargo en `src/` |
| Interfaz | Slint 1.18 (renderer Skia) | Nativa, sin navegador; parche local en `src/patches/i-slint-renderer-skia` |
| Multimedia | FFmpeg 8.1 **enlazado** (libav\*) vía `ffmpeg-the-third` | No se lanza ningún proceso `ffmpeg`; build LGPL |
| Composición | wgpu 30 (Direct3D 12 en Windows) + ruta CPU de respaldo | Efectos en WGSL; comparte dispositivo con la ventana |
| Voz → texto | whisper.cpp vía `whisper-rs` 0.16 + Silero VAD 6.2 embebido | En proceso, cancelable |
| Texto → voz | Chatterbox Turbo (ONNX Runtime) + clonación local | Kokoro/sherpa-onnx fueron retirados aguas arriba |
| Visión | ONNX Runtime 1.28 (`ort`), DirectML en Windows | Recorte de fondo, pincel, mejora, estabilización |
| Proyecto | `concat.json` (JSON versionado) en una carpeta por proyecto | Lector tolerante; autoguardado |
| Automatización | API JSON-RPC / gRPC y servidor MCP integrados | Apagados por defecto; solo `127.0.0.1` con token |

## Mapa de crates → módulos solicitados

| Módulo pedido | Crate(s) | Qué contiene |
|---|---|---|
| 1. Interfaz gráfica | `concat` (`ui/*.slint`, `src/panes`, `src/studio`) | Paneles acoplables, inspector, biblioteca, diálogos, 26 idiomas |
| 2. Motor de edición | `concat-project` (`commands/`, `editor.rs`) | Cada edición es un `Command`; historial deshacer/rehacer |
| 3. Modelo de proyecto | `concat-project` (`model.rs`, `doc.rs`, `unread.rs`) | Documento, versiones, lectura tolerante |
| 4. Línea de tiempo | `concat-core` (`timeline.rs`, `time.rs`), `concat-project` (`placement.rs`) | Pistas, clips, tiempo racional exacto |
| 5. Reproductor / previsualización | `concat-host` (`playback.rs`, `preview.rs`, `proxy.rs`) | Reproducción, proxies, miniaturas, caché |
| 6. Procesamiento multimedia | `concat-media` | Decodificar, codificar, aceleración por hardware, ondas |
| 7. **Motor de subtítulos** | `concat-speech` (`transcribe.rs`, **`words.rs`**), `concat-project` (**`captions.rs`**, **`karaoke.rs`**, `srt.rs`), `concat-text`, `concat-host` (`titles.rs`) | Ver sección propia abajo |
| 8. Motor de audio | `concat-media` (`audio/`), `concat-speech` (`isolate.rs`, `vocals.rs`), paquetes de audio | Mezcla, fundidos, cadenas FFmpeg, separación de voz |
| 9. Motor de efectos | `concat-effects` (132 paquetes), `concat-render` | Catálogo, shaders WGSL, transiciones, máscaras |
| 10. Inteligencia artificial | `concat-speech`, `concat-vision` | Whisper, TTS, recorte, mejora, estabilización |
| 11. Exportación | `concat-export`, `concat-host` (`export.rs`) | Aplanado, bucle de render, H.264/HEVC/AV1/ProRes 4444/VP9 |
| 12. Archivos, caché y modelos | `concat-host` (`dirs.rs`, `models.rs`, `projects.rs`, `memory.rs`, `jobs.rs`) | Descargas con SHA-256, carpetas, trabajos en segundo plano |

Flechas de dependencia (un solo sentido):

```
concat ─► concat-speech ─► concat-host ─► {concat-export, concat-project, concat-media} ─► concat-core
                                          concat-export ─► concat-render ─► concat-core
                                          concat-export ─► concat-effects ─► concat-project
concat-cli / concat-mcp ─► concat-server ─► concat-api ─► concat-host
```

## Cómo viaja un fotograma

1. La interfaz emite un `Command`; `concat-project::Editor` lo aplica al
   documento y registra el paso de deshacer.
2. `concat-export::flatten` convierte el documento en una lista plana de
   `ExportClip` (segundos `f64`).
3. Los títulos y formas no los conoce el compositor: `concat-host::titles`
   los pinta con `concat-text` a una imagen transparente (PNG en caché, o en
   memoria durante un arrastre) y los reincorpora como clips de imagen.
4. `concat-export::resolve` cuantiza a la rejilla de fotogramas y construye
   la `Timeline` del motor (`Rational`).
5. `concat-render` compone cada instante en GPU (o CPU) aplicando
   transformaciones, keyframes, máscaras, efectos y transiciones.
6. El mismo camino sirve al monitor, a la reproducción y al exportador: lo
   que se ve es lo que se exporta.

## Motor de subtítulos (módulo prioritario)

```
audio del clip ─► AudioDecoder (16 kHz mono)
               ─► Silero VAD: dónde hay voz ─► "carrete" solo con voz
               ─► whisper.cpp (idioma fijo o detectado sobre la voz, tiempos por token)
               ─► words.rs: tokens ─► palabras con inicio/fin (une bytes UTF-8 partidos: ñ, á…)
               ─► Segment { start, end, text, words[] }
panel Subtítulos ─► captions::group(…, Grouping) ─► Caption { texto, palabras[] }
               ─► AddTextClip con TextStyle.karaoke = { words, mode, style }
pintor de títulos ─► TextStyle::karaoke_states(): tramos en que el título se ve igual
               ─► una imagen por tramo, cada una un clip `part_of` del título completo
```

Decisiones de diseño:

- **Los tiempos pertenecen a las palabras por orden, no por caracteres.**
  Corregir una palabra mal transcrita no desincroniza nada. Si cambia el
  número de palabras, el mismo intervalo se reparte por longitud
  (`Karaoke::fitted`).
- **Los tiempos viven en el reloj de origen del clip** (como el metraje):
  recortar el inicio o dividir un subtítulo mueve `source_start`, no los
  tiempos; cada mitad sigue iluminando sus palabras en el instante correcto.
- **El resaltado se resuelve al pintar, no en el shader.** Un subtítulo con
  karaoke se dibuja como una imagen por palabra activa. Para que keyframes,
  animaciones de entrada/salida, máscaras y efectos sigan corriendo a lo
  largo del subtítulo completo y no de cada palabra, cada tramo lleva
  `part_of = (inicio, duración)` del título entero
  (`concat_core::timeline::Clip::part_of`, usado por `fraction_at` y
  `pose_at`). Costo: más imágenes en caché por subtítulo (ver ROADMAP,
  Fase 5).
- **Compatibilidad:** `TextStyle.karaoke` es opcional y no se escribe cuando
  está vacío; una versión anterior abre el subtítulo como título normal.
- **Formatos:** SRT (lectura/escritura, `srt.rs`); VTT, ASS (con etiquetas
  `\k` de karaoke) y TXT (`captions.rs`).

## Formato de proyecto

Carpeta por proyecto con `concat.json` (`DOCUMENT_VERSION` en
`concat-project/src/doc.rs`). Un documento de versión más nueva se rechaza;
campos desconocidos se conservan (`extra`) y los ilegibles se reportan
(`unread.rs`) en lugar de abortar. Los medios se referencian por ruta y
nunca se modifican; `relink` relocaliza los faltantes.

## Datos en disco (Windows)

- Configuración: `%APPDATA%\app.concat.editor\` (preferencias, atajos, idiomas extra).
- Datos: modelos (`whisper-models\`, voces, visión), caché de títulos,
  proxies, registros (`logs\concat-<fecha>.log`).
- Todo local; nada se sube.

## Dependencias y licencias (resumen de la auditoría)

- Propias del repo: AGPL-3.0-or-later. Contribuciones entran como MIT-0.
- Slint: GPL-3.0 (opción libre). FFmpeg: LGPL (build BtbN `lgpl-shared`).
- whisper.cpp, Silero VAD, Spleeter, ONNX Runtime: MIT. Chatterbox: MIT.
- Robust Video Matting (pesos del recorte de personas): GPL-3.0, se descarga
  a petición.
- Lista completa y avisos: `THIRD_PARTY_NOTICES.md`.
- **Dependencias añadidas por este proyecto hasta ahora: ninguna.**
