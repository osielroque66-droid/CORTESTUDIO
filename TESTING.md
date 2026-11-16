# Pruebas

## Preparar el entorno (Linux)

```bash
sudo apt-get install -y libfontconfig1-dev libfreetype6-dev libxkbcommon-dev \
  libgl1-mesa-dev libgtk-3-dev libasound2-dev libclang-dev cmake \
  libvulkan1 mesa-vulkan-drivers
```

FFmpeg 8.1 y ONNX Runtime 1.28, fijados por hash en `scripts/ci/deps.lock`:

```bash
scripts/ci/fetch.sh ffmpeg-linux64 ffmpeg.tar.xz
```

```bash
scripts/ci/fetch.sh onnxruntime-linux-x64 onnxruntime.tgz
```

Descomprimir y exportar (ajusta las rutas):

```bash
export FFMPEG_DIR=$PWD/ffmpeg-dev
export ORT_LIB_LOCATION=$PWD/onnxruntime-linux-x64-1.28.0/lib
export ORT_PREFER_DYNAMIC_LINK=1
export LD_LIBRARY_PATH=$FFMPEG_DIR/lib:$ORT_LIB_LOCATION
export CARGO_PROFILE_DEV_DEBUG=0
```

En Windows: build `win64-lgpl-shared` de BtbN en `FFMPEG_DIR` (ver README).

## Comandos

Desde `src/`:

```bash
cargo test --workspace --exclude concat --exclude concat-android --no-fail-fast
```

```bash
cargo check -p concat
```

```bash
cargo fmt --check
```

Desde la raíz:

```bash
python3 scripts/locales.py --check
```

```bash
python3 scripts/models.py --check
```

## Línea base (Concat 0.2.11 sin modificar, Fase 0)

| Suite | Resultado |
|---|---|
| Motor (16 crates) | 1 219 pasan · 0 fallan · 18 ignoradas |

Las ignoradas necesitan un modelo descargado, una GPU real o archivos de
audio externos.

## Pruebas añadidas

| Dónde | Qué demuestra |
|---|---|
| `concat-project/src/karaoke.rs` (10) | Rangos de palabras con acentos y signos; tramos de resaltado sin huecos; pausas largas sin resaltar; modo relleno; clip dividido/recortado conserva sincronía; corregir una palabra conserva tiempos; documento sin karaoke no cambia |
| `concat-project/src/captions.rs` (13) | Agrupación frase / palabra / par / ráfaga / oración sin perder palabras; puntuación española (¿ ¡ …); VTT, TXT y ASS con etiquetas `\k` |
| `concat-speech/src/words.rs` (7) | Tokens → palabras; "ñ" partida en dos tokens se reconstruye; puntuación suelta; tiempos monótonos |
| `concat-host/tests/export.rs` › `a_karaoke_caption_lights_each_word_as_it_is_said` | **Extremo a extremo:** exporta un MP4, lo decodifica y cuenta pixeles: la palabra correcta está en rojo en el segundo correcto; igual tras dividir el clip, en modo relleno, y tras guardar y reabrir |

## Lo que todavía NO está verificado

- Transcripción real de audio en español con los tiempos por palabra
  (necesita un modelo Whisper; no se descargó ninguno sin autorización).
  Cuando haya modelo:

  ```bash
  CONCAT_CAPTION_FILE=voz.wav CONCAT_CAPTION_MODEL=small cargo test -p concat-speech -- --ignored --nocapture
  ```

- La interfaz nueva del panel de subtítulos operada a mano en una ventana.
- Compilación y ejecución en Windows.

## Flujo de aceptación manual (sección 24 del encargo)

1. Abrir la app → Nuevo proyecto 9:16.
2. Importar un video con narración en español y soltarlo en la línea de tiempo.
3. Reproducir, dividir (`Ctrl+B`), borrar con cierre de hueco (`Shift+Supr`).
4. Añadir imagen/video en otra pista.
5. Ajustes › Transcriptor: descargar un modelo. Biblioteca › Texto y
   subtítulos › Subtítulos: Idioma "Español", Líneas "De tres a cinco
   palabras", Resaltado "Palabra que se pronuncia", color → Generar.
6. Corregir una palabra en el inspector; mover/recortar un subtítulo.
7. Añadir música, bajar volumen; añadir título con animación; efecto y transición.
8. Guardar, cerrar, reabrir: todo se conserva.
9. Exportar; comprobar audio, imagen, efectos y palabras resaltadas; sin marca de agua.
10. Desconectar Internet y repetir 5–9.
