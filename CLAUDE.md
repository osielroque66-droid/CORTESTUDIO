# CLAUDE.md — instrucciones permanentes del proyecto

Editor de video de escritorio para Windows, 100 % local, construido sobre
[Concat](https://github.com/jub0t/Concat) (Rust + Slint + FFmpeg + wgpu +
whisper.cpp). Objetivo: cubrir el flujo de trabajo de un editor tipo CapCut
para TikTok / Reels / Shorts, con prioridad en **subtítulos automáticos en
español con resaltado palabra por palabra**.

Documentos hermanos: [ARCHITECTURE.md](ARCHITECTURE.md),
[ROADMAP.md](ROADMAP.md), [CHANGELOG.md](CHANGELOG.md),
[TESTING.md](TESTING.md). Mantenlos al día en cada etapa.

## Reglas que no se negocian

1. **Se trabaja sobre Concat, no se reescribe.** Antes de crear algo, busca
   si ya existe (el repo tiene ~170 000 líneas de Rust y 40 000 de Slint).
2. **Nada de simulaciones ni botones decorativos.** Un control solo entra a
   la interfaz cuando su función está implementada y probada.
3. **Una función no está "terminada" sin prueba que lo demuestre.** Las
   pruebas de extremo a extremo exportan un MP4 real y leen sus pixeles
   (`src/crates/concat-host/tests/export.rs`); úsalas como modelo.
4. **100 % local.** Sin telemetría, sin APIs en la nube, sin cuentas. La red
   solo se toca cuando el usuario lo pide (descargar un modelo, buscar
   actualización). Ver `CODE_SIGNING.md` › *What Concat sends over the network*.
5. **No descargar modelos sin autorización del usuario**, ni en la app ni
   durante el desarrollo.
6. **No romper lo existente.** La línea base de pruebas debe seguir en verde
   (ver TESTING.md). Los proyectos guardados por versiones anteriores deben
   seguir abriendo: todo campo nuevo del documento lleva `#[serde(default)]`
   y se omite al escribir cuando está vacío.
7. **Licencias.** El proyecto es AGPL-3.0-or-later. Código de terceros solo
   con licencia permisiva (MIT, BSD, Apache-2.0, zlib, ISC); regístralo en
   `THIRD_PARTY_NOTICES.md`. No copiar recursos propietarios de CapCut.
8. **No eliminar código ni archivos sin razón documentada** en CHANGELOG.md.

## Dónde está cada cosa

- Raíz de git: este directorio. **Raíz de Cargo: `src/`** — todo comando
  `cargo` se ejecuta desde `src/`.
- Crates en `src/crates/` (mapa completo en ARCHITECTURE.md). Las flechas de
  dependencia van en un solo sentido:
  `concat → concat-speech → concat-host → {export, project, media} → core`.
- Interfaz: `src/crates/concat/ui/*.slint`; su lógica en
  `src/crates/concat/src/panes/*.rs` y `src/crates/concat/src/studio/`.
- Textos de la interfaz: `src/crates/concat/locales/<idioma>.json`.
  `en.json` es el inventario; **`es.json` es el idioma objetivo (español de
  México)**. Tras tocar textos: `python3 scripts/locales.py` y luego
  `python3 scripts/locales.py --check`. Los demás idiomas reciben el texto
  en inglés como respaldo.
- Efectos/filtros/transiciones: una carpeta por paquete en
  `src/crates/concat-effects/packages/concat.<nombre>/` (manifiesto TOML +
  WGSL o cadena FFmpeg). Se descubren solos al compilar; no requieren Rust.
- Modelos de IA: tabla en `models/manifest.toml` + la tabla del motor
  correspondiente; ambos con SHA-256 (una descarga sin hash se rechaza).

## Convenciones de código (heredadas de Concat; respétalas)

- Cabecera SPDX en cada archivo fuente nuevo:
  `// SPDX-License-Identifier: AGPL-3.0-or-later`.
- Toda edición del proyecto es un `concat_project::Command` (deshacer/rehacer
  y la API JSON-RPC/MCP salen gratis). No mutar el documento por fuera.
- Tiempo del motor en `concat_core::time::Rational`; el documento y la
  interfaz usan `f64` en segundos y se cuantizan a la rejilla de fotogramas
  en `concat-export/src/resolve.rs`.
- Hilos, no `async`. Trabajo pesado en un worker (`spawn_in_project`) que
  reporta a la interfaz; la ventana nunca espera.
- Registro con `log::info!/warn!/error!`, nunca `println!`.
- `missing_docs` es *warning* y CI lo trata como error: todo ítem público
  lleva comentario `///`. Comentarios y nombres de código en inglés (como el
  resto del repo); documentos de proyecto y textos de interfaz en español.
- `cargo fmt` antes de cada commit.

## Compilar y probar (Linux, entorno de desarrollo en la nube)

Requiere Rust 1.93.1 (fijado en `src/rust-toolchain.toml`), FFmpeg ≥ 7 de
desarrollo, ONNX Runtime 1.28, cmake, clang. Receta completa en TESTING.md.

```bash
cd src
cargo test --workspace --exclude concat --exclude concat-android   # motor
cargo check -p concat                                               # ventana + Slint
cargo run --profile quick -p concat                                 # abrir el editor
```

La ventana (`concat`) tarda y consume mucha memoria al compilar (~7 GB): es
normal, no es un cuelgue.

## Windows (plataforma objetivo)

FFmpeg: build `win64-lgpl-shared` de BtbN apuntado con `FFMPEG_DIR`
(`scripts/ci/deps.lock` fija versión y hash). Instaladores: `assets/windows/`
(`concat.iss` para Inno Setup, `concat.wxs` para MSI), generados por
`.github/workflows/build-app.yml`. El nombre y logo "Concat" son marca del
autor original (`TRADEMARK.md`): una distribución propia debe llevar otro
nombre.

## Al terminar cada etapa, reportar

Funciones implementadas · archivos modificados · dependencias añadidas ·
pruebas ejecutadas y resultado · errores pendientes · lo que aún no está
disponible · siguiente etapa. Y actualizar CHANGELOG.md y ROADMAP.md.
