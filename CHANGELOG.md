# Registro de cambios

Cambios de este proyecto sobre Concat 0.2.11 (`a7cda105`).

## Etapa 1 — Auditoría y subtítulos por palabra

### Añadido

- **Tiempos por palabra en la transcripción.** Whisper corre con marcas de
  tiempo por token; `concat-speech/src/words.rs` las une en palabras
  (acentos, "ñ" y signos incluidos). `Segment.words`.
- **Subtítulos que resaltan la palabra que se pronuncia.**
  `TextStyle.karaoke` (`concat-project/src/karaoke.rs`): un tiempo por
  palabra, modo *palabra activa* o *relleno karaoke*, y el estilo de la
  palabra dicha. Funciona igual en vista previa y exportación.
- **Agrupación de subtítulos** (`concat-project/src/captions.rs`): frases
  como se dicen, palabra por palabra, dos palabras, de tres a cinco, oraciones
  completas; corta en pausas y en fin de oración.
- **Exportar subtítulos en VTT, ASS (con etiquetas de karaoke) y TXT**,
  además del SRT existente. Selector de formato en el panel.
- **Panel de subtítulos:** filas "Líneas", "Resaltado" y color; botón
  "Aplicar a los subtítulos de la línea de tiempo" (un solo paso de deshacer).
  Las elecciones se recuerdan entre sesiones.
- Los subtítulos importados de SRT y los escritos a mano también pueden
  llevar resaltado (tiempo repartido por longitud de palabra).
- Documentos: CLAUDE.md, ARCHITECTURE.md, ROADMAP.md, TESTING.md.

### Cambiado

- `concat_core::timeline::Clip` y `ExportClip` ganan `part_of`: un clip
  puede ser un tramo de otro más largo, y sus keyframes, animaciones,
  máscaras y efectos se miden sobre el largo completo.
- El pintor de títulos (`concat-host/src/titles.rs`) produce una imagen por
  tramo de resaltado. La caché de títulos en disco pasa de 400 a 2 000
  imágenes, porque un subtítulo con resaltado ocupa varias.
- El botón "Exportar SRT…" pasa a ser "Exportar subtítulos…" con selector
  de formato (la clave `captions.exportSrt` se retiró de los idiomas).
- Español: traducidas las cadenas que seguían en inglés. Los otros 24
  idiomas muestran en inglés los textos nuevos.
- "Usar este estilo para títulos nuevos" ya no arrastra los tiempos de
  palabras del título de origen.

### Dependencias añadidas

Ninguna.

### Compatibilidad

Los proyectos anteriores abren sin cambios. Un proyecto con subtítulos
resaltados abierto en Concat 0.2.11 original muestra los subtítulos sin
resaltado.
