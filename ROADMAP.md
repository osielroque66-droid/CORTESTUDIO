# Hoja de ruta

Estado: ✅ existe y está probado · 🟡 existe pero incompleto o sin verificar
aquí · ❌ no existe. "Concat" = ya venía en la base 0.2.11; "nuevo" =
añadido en este proyecto.

## Fase 0 — Auditoría ✅ (completada)

- Repositorio clonado e inspeccionado (18 crates, ~170 000 líneas de Rust,
  ~40 000 de Slint, 132 paquetes de efectos, 26 idiomas).
- Compilado en Linux con la receta de CI (FFmpeg 8.1, ONNX Runtime 1.28).
- **Línea base: 1 219 pruebas del motor pasan, 0 fallan, 18 ignoradas**
  (las ignoradas piden modelos descargados o GPU).
- Arquitectura, dependencias y licencias documentadas en ARCHITECTURE.md.

### Inventario contra lo solicitado

| Área | Estado | Detalle |
|---|---|---|
| App de escritorio nativa, sin nube, sin cuenta, sin marca de agua | ✅ Concat | Rust + Slint; red solo a petición |
| Interfaz oscura, paneles acoplables/redimensionables, español | ✅ Concat + nuevo | `es.json` completado (quedaban 59 textos en inglés) |
| Multipista: mover, recortar, dividir, duplicar, copiar/pegar, ripple, imán, bloquear/ocultar/silenciar pista, altura de pista, zoom | ✅ Concat | Cubierto por pruebas de `concat-project` |
| Selección múltiple que se mueve en bloque, enlazar audio/video, separar audio | ✅ Concat | Grupos persistentes (agrupar/desagrupar) ❌ |
| Renombrar pistas | ❌ | Las pistas no tienen nombre en el modelo |
| Atajos configurables, deshacer/rehacer | ✅ Concat | Ajustes › Atajos |
| Keyframes (posición, escala, rotación, opacidad, volumen, parámetros de efectos y máscaras), curvas Bézier, dopesheet | ✅ Concat | |
| Velocidad constante y curvas, congelar, invertir, voltear, rotar, recorte de encuadre, estabilizar | ✅ Concat | |
| Formatos 9:16, 16:9, 1:1, 4:5, 3:4 y personalizados | ✅ Concat | |
| Capas, PiP, modos de fusión, máscaras (círculo, rectángulo, lineal, estrella, corazón), chroma key, luma key | ✅ Concat | |
| Eliminación de fondo (personas, objetos, pincel con seguimiento) | ✅ Concat | Modelos opcionales |
| Seguimiento de movimiento para anclar capas | ❌ | |
| 132 efectos / filtros / transiciones como paquetes | ✅ Concat | Arquitectura de plugins ya existente |
| Color: exposición, contraste, saturación, balance, curvas, ruedas, LUT `.cube` | ✅ Concat | HSL por rango ❌ |
| Audio: ondas, volumen, fundidos, normalizar, reducir ruido, mejorar voz, eco, reverb, robot, teléfono, tono, grabación de voz en off, separar voz/música | ✅ Concat | Ecualizador paramétrico y compresor con controles ❌ |
| Texto a voz local + clonación | 🟡 Concat | Chatterbox; falta verificar voces en español |
| Subtítulos automáticos (Whisper local, VAD, elegir idioma/modelo) | ✅ Concat | Modelos ofrecidos: tiny, base, small |
| **Tiempos por palabra** | ✅ nuevo | `concat-speech/src/words.rs` |
| **Resaltar la palabra que se pronuncia / karaoke** | ✅ nuevo | Probado exportando un MP4 real |
| **Subtítulos por palabra, de 2, de 3–5, por oración** | ✅ nuevo | `concat-project/src/captions.rs` |
| Exportar SRT · importar SRT | ✅ Concat | |
| **Exportar VTT, ASS (con karaoke), TXT** | ✅ nuevo | |
| Panel de transcripción editable (clic → salta el cabezal) | ❌ | Fase 2b |
| Editar el tiempo de cada palabra a mano | ❌ | El modelo ya lo soporta; falta interfaz |
| Plantillas de estilo de subtítulos | 🟡 Concat | Presets de texto; falta guardar el resaltado en la plantilla |
| Modelos Whisper grandes (medium, large-v3-turbo) | ❌ | Ver "Pendientes con bloqueo" |
| Eliminación de silencios con revisión | ❌ | Fase 4; el VAD ya está integrado |
| Detección de muletillas | ❌ | Fase 4; depende de tiempos por palabra (ya hechos) |
| Detección de escenas, reencuadre automático, momentos destacados, búsqueda por transcripción | ❌ | Fase 4 |
| Exportación H.264 / HEVC / AV1, hasta 4K60, 10 bits, bitrate/calidad, progreso, cancelar | ✅ Concat | MP4; ProRes 4444/VP9 con alfa |
| Contenedor MOV para H.264/HEVC, cola de exportación, preajustes por red social | ❌ | Fase 5 |
| Guardar, autoguardado, recuperación, relocalizar medios, versiones de formato | ✅ Concat | |
| Proxies, miniaturas, caché de fotogramas y ondas | ✅ Concat | |
| Indicadores de CPU / memoria / GPU en vivo | 🟡 Concat | Hay información del sistema y medidores de audio; falta monitor de recursos |
| Administrador de modelos (descarga, tamaño, progreso, SHA-256, borrar) | ✅ Concat | Ajustes › Modelos |
| Instalador de Windows (.exe Inno Setup y .msi) | 🟡 Concat | Scripts listos; no se ha generado aquí |
| Stickers | ❌ | La pestaña dice "Pronto" en Concat |
| Importar proyectos de CapCut | ✅ Concat | Bonus |

## Fase 1 — Editor principal ✅ (ya cubierto por Concat)

Lo pedido para esta fase ya existía y pasa sus pruebas. Trabajo propio:
completar el español. Pendiente menor: renombrar pistas.

## Fase 2 — Subtítulos (en curso)

Hecho (2a):
- Tiempos por palabra desde Whisper.
- Modelo `Karaoke` en el título; resaltado "palabra activa" y "relleno
  karaoke" en vista previa y exportación; sobrevive a dividir, recortar,
  guardar y reabrir.
- Agrupación: frases, palabra por palabra, dos palabras, 3–5, oraciones.
- 7 colores de resaltado; "Aplicar a los subtítulos de la línea de tiempo".
- Exportación SRT / VTT / ASS / TXT.

Siguiente (2b):
1. Verificar con audio real en español y un modelo descargado (requiere tu
   autorización para bajar el modelo; ver TESTING.md).
2. Panel de transcripción: lista de segmentos y palabras, clic para saltar,
   corregir texto, dividir/combinar segmentos, editar tiempos por palabra.
3. Controles de resaltado en el inspector del clip (color libre, negritas,
   escala de la palabra activa) y guardarlos en plantillas.
4. Animaciones por palabra (aparecer/rebote) y modo "revelar".
5. Generar subtítulos solo de una región seleccionada.
6. Evaluar alineación DTW de whisper.cpp para tiempos más finos.

## Fase 3 — Edición avanzada (mayormente cubierta por Concat)

Pendiente: HSL por rango, seguimiento de movimiento, máscaras a mano alzada,
partículas.

## Fase 4 — Audio e IA

1. Eliminación de silencios: reutilizar `speech_spans` (Silero), panel de
   revisión con aceptar/rechazar por corte, aplicar con ripple respetando
   clips enlazados.
2. Muletillas ("este", "eh", "o sea", "pues"): lista editable, sobre los
   tiempos por palabra.
3. Ecualizador y compresor con controles.
4. Detección de escenas, reencuadre automático, búsqueda por transcripción.
5. Voces TTS en español verificadas.

## Fase 5 — Optimización

- Karaoke: pintar solo el bloque del texto (no el fotograma completo) o
  resolver el resaltado en el shader, para videos largos.
- Monitor de CPU/memoria/GPU; cola de exportación; preajustes de redes; MOV.
- Pruebas con videos de más de 30 minutos.

## Fase 6 — Aplicación final

Instalador de Windows con nombre propio, asistente de primera ejecución,
manual de usuario en español, validación sin conexión en una PC real.

## Pendientes con bloqueo

- **Modelos Whisper medium / large-v3-turbo.** Cada modelo necesita su
  SHA-256 en la tabla; el entorno de desarrollo no tiene salida a
  huggingface.co para obtenerlos. Se resuelve desde una máquina con acceso.
- **Prueba en Windows.** Todo lo de aquí se compiló y probó en Linux; falta
  compilar y recorrer el flujo de aceptación en Windows.
