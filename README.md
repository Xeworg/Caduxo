# Caduxo — Control de Vencimiento de Productos

[English](#english) · [Español](#español)

![Caduxo](docs/assets/caduxo-readme-banner.png)

> 🚧 **Project status:** Caduxo is under active development. Features, interfaces, and data behavior may change between versions. It can be used in production, but keep backups and review changes between releases.
>
> 🚧 **Estado del proyecto:** Caduxo está en desarrollo activo. Las funcionalidades, interfaces y el comportamiento de los datos pueden cambiar entre versiones. Se puede usar en producción, pero mantené copias de seguridad y revisá los cambios entre versiones.

[![Support Caduxo on Ko-fi](https://img.shields.io/badge/Support-Ko--fi-F40?logo=kofi&logoColor=white&style=flat-square)](https://ko-fi.com/xeworgmmm "Support Caduxo development on Ko-fi")

---

## English

### Caduxo — Product Expiration Tracking

---

## Overview

Caduxo is a local-first desktop application for recording product expiration dates. It helps store owners and employees identify which products are expired or about to expire, so they can be used, sold, donated, or discarded before losses occur. Caduxo does not manage sales, payments, accounting, or stock valuation.

### Local, single-device software

Caduxo is not an online service or cloud platform. It is designed to keep expiration records locally on one computer, without requiring an account, internet connection, or cloud synchronization. Multi-store support means you can organize locations within that local installation; it does not provide multi-device collaboration or online synchronization.

---

## Key Features

- **Add products** by SKU, UPC, or barcode scanner (keyboard-wedge mode).
- **Record expiration batches** with quantity, unit, expiration date, store, and alert threshold.
- **Main dashboard** shows expired, same-day, upcoming, and within-alert-window batches at a glance.
- **Calendar view** to browse batches by expiration date.
- **Reports** view, print, or export to PDF/CSV.
- **CSV import** for simple product catalog ingestion.
- **Multi-store support** with optional internal locations (shelf, refrigerator, warehouse, etc.).
- **Daily OS notifications** while the application is running.

---

## Scope

| Included | Excluded in v1 |
|---|---|
| Product catalog with SKU, UPC/barcode, categories | POS, invoicing, payment, electronic billing |
| Expiration batch tracking with per-batch alert thresholds | Cloud sync or multi-user collaboration |
| Barcode-reader-friendly input (keyboard-wedge) | Mobile app |
| Dashboard with urgency sections and filters | Advanced barcode reader driver integration |
| Calendar view | Background notifications after app close |
| Printable reports with PDF export | |
| CSV import / CSV export + manual backup | |
| Local SQLite storage | |

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri v2 |
| Backend | Rust ≥ 1.77.2 |
| Database | SQLite via `sqlx` + `rusqlite` |
| Frontend | Svelte 5 + TypeScript |
| Styles | Tailwind CSS v4 + DaisyUI (multiple themes including `caduxo-light` and built-in themes) |
| Internationalization | `typesafe-i18n` |
| Build tooling | Vite |
| PDF export | `printpdf` (Rust) |
| CSV import | `csv` crate (Rust) |
| Packaging | Tauri v2 bundle targets (`.deb`, `.rpm`, `.AppImage`, `.msi`, NSIS) |

**Verified integrations:** DaisyUI theme switching (including custom `caduxo-light` and built-in themes), plus Tauri frontend plugins for dialogs, notifications, and OS detection. The system tray and close-to-tray behavior use Tauri's built-in tray feature. macOS bundle targets exist but are not a current priority for testing or CI.

---

## Prerequisites

### All platforms

- **Node.js** ≥ 18
- **Rust** ≥ 1.77.2
- **npm** (any recent version)

Install frontend dependencies once:

```bash
npm install
```

### Linux — build dependencies

```bash
# Debian / Ubuntu
sudo apt install \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libssl-dev \
  libsoup-3.0-dev \
  gdk-pixbuf-2.0-dev \
  libcairo2-dev \
  libpango1.0-dev \
  libatk1.0-dev \
  libglib2.0-dev \
  libayatana-appindicator3-dev \
  patchelf
```

For Fedora, RHEL, and Arch equivalents, see [`docs/packaging.md`](docs/packaging.md).

### Windows — build dependencies

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **"Desktop development with C++"** workload. The Windows SDK and MSVC toolchain are required for the Rust target `x86_64-pc-windows-msvc`.

---

## Local Development

```bash
# Install frontend dependencies
npm install

# Start the development server (frontend + Rust backend)
npm run tauri dev
```

The application launches at `http://localhost:1420` (or the port Tauri assigns) with hot-reload for the frontend and Cargo watch for the backend.

---

## Checks

Run type checking before submitting changes:

```bash
# TypeScript / Svelte type checking (no JS test runner is currently configured)
npm run check
```

`npm run check` performs type checking only. A frontend test file (`*.test.ts`) exists in the source tree, but no JavaScript test runner (Vitest, Playwright, etc.) is set up in the project.

Rust checks run automatically as part of the development flow (`cargo check` is invoked by `tauri dev`). To run them manually:

```bash
# In src-tauri/
cargo check
cargo clippy --lib --tests
cargo test --lib
```

---

## Production Build

```bash
# Build optimized frontend + Rust binary + installers
npm run tauri build
```

Output artifacts are placed in `src-tauri/target/release/bundle/`.

### Linux artifacts

| Artifact | Path |
|---|---|
| Unbundled binary | `src-tauri/target/release/caduxo` |
| Debian package | `bundle/deb/Caduxo_0.2.0_amd64.deb` |
| RPM package | `bundle/rpm/Caduxo-0.2.0-1.x86_64.rpm` |
| AppImage | `bundle/appimage/Caduxo_0.2.0_amd64.AppImage` |

> **AppImage note:** requires `linuxdeploy` in `$PATH` at build time. See [`docs/packaging.md`](docs/packaging.md) for setup.

### Windows artifacts

| Artifact | Path |
|---|---|
| Unbundled binary | `src-tauri/target/release/Caduxo.exe` |
| MSI installer | `bundle/msi/Caduxo_0.2.0_x64.msi` |
| NSIS installer | `bundle/nsis/Caduxo_0.2.0_x64-setup.exe` |

---

## Linux Runtime Dependencies

| Package (Debian/Ubuntu) | Purpose |
|---|---|
| `libwebkit2gtk-4.1-dev` | UI layer (Tauri) |
| `libgtk-3-dev` | Interface layout |
| `libssl-dev` | Tauri plugins with HTTPS |
| `libsoup-3.0-dev` | Network requests |
| `gdk-pixbuf-2.0-dev` | Image handling |
| `libcairo2-dev` | PDF rendering |
| `libpango1.0-dev` | Text layout |
| `libatk1.0-dev` | Accessibility |
| `libglib2.0-dev` | Core system library |
| `libayatana-appindicator3-dev` | System tray / app indicator support |

The **AppImage** bundles WebKitGTK and runs portably without system packages.

---

## WebView2 on Windows

Tauri v2 uses the operating system's native WebView for the interface layer.

- WebView2 Evergreen Runtime is **pre-installed on Windows 10 1803+ and Windows 11**.
- **No administrator rights are required** to run Caduxo when WebView2 is present.
- If WebView2 is not installed, install the [Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) before launching, or use the MSI/NSIS installer produced by Tauri — both can install WebView2 during installation.
- Verification: check `%ProgramFiles(x86)%\Microsoft\EdgeWebView\Application\` for `msedgewebview2.exe`, or search for the WebView2 registry entry.

---

## Artifacts

### Data storage

| OS | Path |
|---|---|
| Linux | `~/.local/share/com.caduxo.app/` |
| Windows | `%APPDATA%\com.caduxo.app\` |
| AppImage | `~/.local/share/com.caduxo.app/` (app data); temp path managed by runtime for bundle contents |

### Database

The database file is `caduxo.db` (with optional `.db-wal` and `.db-shm` in WAL mode). Logs are written to `logs/` under the same base directory.

### Manual backup

Copy `caduxo.db` to a safe location, or use the built-in backup/export function in the application.

---

## CI

Caduxo uses GitHub Actions to build and type-check on every push. The workflow runs on two platforms in parallel:

- **ubuntu-22.04** — Linux build and full `tauri build`
- **windows-latest** — Windows build and full `tauri build`

Each job installs Node, Rust, platform-specific build dependencies, runs `npm ci`, runs `npm run check`, and then runs `npm run tauri build`. Artifacts (installers and binaries) are uploaded for both platforms.

Cross-compilation from Linux to Windows is not used as a substitute for Windows validation. Windows packaging is only considered verified after `npm run tauri build` passes on a native Windows runner.

See [`.github/workflows/build.yml`](.github/workflows/build.yml) for the full workflow definition.

---

## Español

### Caduxo — Control de Vencimiento de Productos

## Descripción General

Caduxo es una aplicación de escritorio local-first para registrar las fechas de vencimiento de productos. Ayuda a dueños y empleados de tiendas a identificar qué productos están vencidos o próximos a vencer, de modo que puedan usarse, venderse, donarse o descartarse antes de que se generen pérdidas. Caduxo no gestiona ventas, pagos, contabilidad ni valuación de stock.

### Software local para un solo equipo

Caduxo no es un servicio online ni una plataforma en la nube. Está pensado para llevar el control de vencimientos localmente en una sola computadora, sin cuenta, conexión a internet ni sincronización en la nube. El soporte para varias tiendas permite organizar ubicaciones dentro de esa instalación local; no ofrece colaboración entre dispositivos ni sincronización online.

---

## Funciones Principales

- **Agregar productos** por SKU, UPC o lector de código de barras (modo keyboard-wedge).
- **Registrar lotes de vencimiento** con cantidad, unidad, fecha de vencimiento, tienda y umbral de alerta.
- **Panel principal** muestra los lotes vencidos, del día, próximos a vencer y dentro de la ventana de alerta de un vistazo.
- **Vista de calendario** para navegar los lotes por vencer según el día.
- **Reportes** ver, imprimir o exportar a PDF/CSV.
- **Importación CSV** para ingestión simple del catálogo de productos.
- **Soporte multi-tienda** con ubicaciones internas opcionales (estante, heladera, depósito, etc.).
- **Notificaciones diarias del SO** mientras la aplicación está en ejecución.

---

## Alcance

| Incluido | Excluido en la versión 1 |
|---|---|
| Catálogo de productos con SKU, UPC/código de barras, categorías | POS, facturación, pago, facturación electrónica |
| Seguimiento de lotes de vencimiento con umbrales de alerta por lote | Sincronización en la nube o colaboración multiusuario |
| Entrada amigable para lectores (keyboard-wedge) | Aplicación móvil |
| Panel con secciones de urgencia y filtros | Integración avanzada de drivers de lectores |
| Vista de calendario | Notificaciones en segundo plano luego de cerrar la app |
| Reportes imprimibles con exportación a PDF | |
| Importación CSV / exportación CSV + respaldo manual | |
| Almacenamiento local SQLite | |

---

## Pila Tecnológica

| Capa | Tecnología |
|---|---|
| Shell de escritorio | Tauri v2 |
| Backend | Rust ≥ 1.77.2 |
| Base de datos | SQLite vía `sqlx` + `rusqlite` |
| Frontend | Svelte 5 + TypeScript |
| Estilos | Tailwind CSS v4 + DaisyUI (múltiples temas incluyendo `caduxo-light` y temas integrados) |
| Internacionalización | `typesafe-i18n` |
| Herramienta de build | Vite |
| Exportación PDF | `printpdf` (Rust) |
| Importación CSV | `csv` crate (Rust) |
| Empaquetado | Objetivos de bundle de Tauri v2 (`.deb`, `.rpm`, `.AppImage`, `.msi`, NSIS) |

**Integraciones verificadas:** cambio de temas DaisyUI (incluyendo el tema personalizado `caduxo-light` y temas integrados), además de plugins frontend de Tauri para diálogos, notificaciones y detección del sistema operativo. La bandeja del sistema y el comportamiento de cerrar hacia la bandeja usan la función de bandeja integrada de Tauri. Los targets de bundle para macOS existen, pero no son prioridad actual para pruebas o CI.

---

## Requisitos Previos

### Todas las plataformas

- **Node.js** ≥ 18
- **Rust** ≥ 1.77.2
- **npm** (cualquier versión reciente)

Instalar las dependencias del frontend una vez:

```bash
npm install
```

### Linux — dependencias de compilación

```bash
# Debian / Ubuntu
sudo apt install \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libssl-dev \
  libsoup-3.0-dev \
  gdk-pixbuf-2.0-dev \
  libcairo2-dev \
  libpango1.0-dev \
  libatk1.0-dev \
  libglib2.0-dev \
  libayatana-appindicator3-dev \
  patchelf
```

Para equivalentes en Fedora, RHEL y Arch, ver [`docs/packaging.md`](docs/packaging.md).

### Windows — dependencias de compilación

Instalar [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) con la carga de trabajo **"Desktop development with C++"**. El Windows SDK y el toolchain de MSVC son necesarios para el target Rust `x86_64-pc-windows-msvc`.

---

## Desarrollo Local

```bash
# Instalar dependencias del frontend
npm install

# Iniciar el servidor de desarrollo (frontend + backend Rust)
npm run tauri dev
```

La aplicación se lanza en `http://localhost:1420` (o el puerto que Tauri asigne) con hot-reload para el frontend y Cargo watch para el backend.

---

## Verificaciones

Ejecutar la verificación de tipos antes de enviar cambios:

```bash
# Verificación de tipos TypeScript / Svelte (sin runner de tests JS configurado actualmente)
npm run check
```

`npm run check` realiza solo verificación de tipos. Existe un archivo de test frontend (`*.test.ts`) en el árbol de fuentes, pero no hay un runner de tests JavaScript (Vitest, Playwright, etc.) configurado en el proyecto.

Las verificaciones de Rust se ejecutan automáticamente como parte del flujo de desarrollo (`cargo check` es invocado por `tauri dev`). Para ejecutarlas manualmente:

```bash
# En src-tauri/
cargo check
cargo clippy --lib --tests
cargo test --lib
```

---

## Compilación de Producción

```bash
# Compilar frontend optimizado + binario Rust + instaladores
npm run tauri build
```

Los artefactos de salida se colocan en `src-tauri/target/release/bundle/`.

### Artefactos para Linux

| Artefacto | Ruta |
|---|---|
| Binario sin empaquetar | `src-tauri/target/release/caduxo` |
| Paquete Debian | `bundle/deb/Caduxo_0.2.0_amd64.deb` |
| Paquete RPM | `bundle/rpm/Caduxo-0.2.0-1.x86_64.rpm` |
| AppImage | `bundle/appimage/Caduxo_0.2.0_amd64.AppImage` |

> **Nota sobre AppImage:** requiere `linuxdeploy` en `$PATH` al momento de la compilación. Ver [`docs/packaging.md`](docs/packaging.md) para la configuración.

### Artefactos para Windows

| Artefacto | Ruta |
|---|---|
| Binario sin empaquetar | `src-tauri/target/release/Caduxo.exe` |
| Instalador MSI | `bundle/msi/Caduxo_0.2.0_x64.msi` |
| Instalador NSIS | `bundle/nsis/Caduxo_0.2.0_x64-setup.exe` |

---

## Dependencias de Runtime en Linux

| Paquete (Debian/Ubuntu) | Propósito |
|---|---|
| `libwebkit2gtk-4.1-dev` | Capa UI (Tauri) |
| `libgtk-3-dev` | Layout de la interfaz |
| `libssl-dev` | Plugins Tauri con HTTPS |
| `libsoup-3.0-dev` | Solicitudes de red |
| `gdk-pixbuf-2.0-dev` | Manejo de imágenes |
| `libcairo2-dev` | Renderizado de PDF |
| `libpango1.0-dev` | Layout de texto |
| `libatk1.0-dev` | Accesibilidad |
| `libglib2.0-dev` | Librería central del sistema |
| `libayatana-appindicator3-dev` | Soporte para bandeja del sistema / app indicator |

El **AppImage** incluye WebKitGTK y se ejecuta de forma portable sin paquetes del sistema.

---

## WebView2 en Windows

Tauri v2 utiliza el WebView nativo del sistema operativo para la capa de interfaz.

- WebView2 Evergreen Runtime viene **preinstalado en Windows 10 1803+ y en Windows 11**.
- **No se requieren derechos de administrador** para ejecutar Caduxo cuando WebView2 está presente.
- Si WebView2 no está instalado, instalar el [Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) antes de iniciar, o usar el instalador MSI/NSIS producido por Tauri — ambos pueden instalar WebView2 durante la instalación.
- Verificación: revisar `%ProgramFiles(x86)%\Microsoft\EdgeWebView\Application\` en busca de `msedgewebview2.exe`, o buscar la entrada del registro de WebView2.

---

## Artefactos

### Almacenamiento de datos

| SO | Ruta |
|---|---|
| Linux | `~/.local/share/com.caduxo.app/` |
| Windows | `%APPDATA%\com.caduxo.app\` |
| AppImage | `~/.local/share/com.caduxo.app/` (datos de la app); ruta temporal gestionada por el runtime para el contenido del bundle |

### Base de datos

El archivo de base de datos es `caduxo.db` (con opcionalmente `.db-wal` y `.db-shm` en modo WAL). Los logs se escriben en `logs/` bajo el mismo directorio base.

### Respaldo manual

Copiar `caduxo.db` a una ubicación segura, o usar la función de respaldo/exportación integrada en la aplicación.

---

## CI

Caduxo utiliza GitHub Actions para compilar y verificar tipos en cada push. El workflow se ejecuta en dos plataformas en paralelo:

- **ubuntu-22.04** — Compilación en Linux y `tauri build` completo
- **windows-latest** — Compilación en Windows y `tauri build` completo

Cada job instala Node, Rust, las dependencias de compilación necesarias para la plataforma, ejecuta `npm ci`, ejecuta `npm run check`, y luego ejecuta `npm run tauri build`. Los artefactos (instaladores y binarios) se suben para ambas plataformas.

La compilación cruzada desde Linux a Windows no se utiliza como sustituto de la validación en Windows. El empaquetado para Windows solo se considera verificado después de que `npm run tauri build` pase en un runner nativo de Windows.

Ver [`.github/workflows/build.yml`](.github/workflows/build.yml) para la definición completa del workflow.

---

## Support / Donaciones

If Caduxo is useful to you, consider supporting its development:

[![Support Caduxo on Ko-fi](https://img.shields.io/badge/Support-Ko--fi-F40?logo=kofi&logoColor=white&style=flat-square)](https://ko-fi.com/xeworgmmm "Support Caduxo development on Ko-fi")

Si te resulta útil Caduxo, considerá apoyar su desarrollo:

[![Apoyá Caduxo en Ko-fi](https://img.shields.io/badge/Support-Ko--fi-F40?logo=kofi&logoColor=white&style=flat-square)](https://ko-fi.com/xeworgmmm "Apoyá el desarrollo de Caduxo en Ko-fi")

---

## License and warranty / Licencia y garantía

Caduxo is distributed under the [GNU General Public License v3.0](LICENSE) by **Caduxo Contributors**. Contributions are accepted under the same GPLv3 terms; see [`CONTRIBUTING.md`](CONTRIBUTING.md).

Caduxo is provided **without warranty**. Use it at your own risk. See sections 15 and 16 of the GPLv3 for the warranty disclaimer and limitation of liability.

Caduxo se distribuye bajo la [Licencia Pública General GNU v3](LICENSE) por **Caduxo Contributors**. Las contribuciones se aceptan bajo los mismos términos GPLv3; consultá [`CONTRIBUTING.md`](CONTRIBUTING.md).

Caduxo se entrega **sin ninguna garantía**. Usalo bajo tu propia responsabilidad. Consultá las secciones 15 y 16 de la GPLv3 para la exclusión de garantías y la limitación de responsabilidad.
