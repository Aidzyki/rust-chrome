# 🦀 Rust Chrome — Material You Edition

![Rust](https://img.shields.io/badge/Language-Rust-orange?logo=rust)
![Platform](https://img.shields.io/badge/Platform-Linux%20(Wayland%20%26%20X11)-blue?logo=linux)
![Arch](https://img.shields.io/badge/Tested%20on-Arch%20Linux-1793D1?logo=arch-linux)
![UI](https://img.shields.io/badge/Design-Material%20You%20(MD3)-A8C7FA?logo=materialdesign)

Легковесный, быстрый и современный веб-браузер на **Rust**, выполненный в дизайне **Material You (Material Design 3 / Chrome Refresh)**. Создан с использованием нативных контейнеров `GTK3`, оконного менеджера `Tao` и движка `Wry` (WebKitGTK).

---

## ✨ Особенности

* 🎨 **Дизайн Material You (MD3)** — плавающий «пилюльный» адресный бар (Omnibox), скруглённые вкладки в стиле современного Chrome и плавные анимации.
* 🌈 **Живая смена цветовых тем** — динамическое изменение оттенка интерфейса (Hue) и переключение между тёмной и светлой темой в один клик.
* 🗂 **Двухслойная нативная архитектура** — панель управления и каждая вкладка работают в изолированных нативных WebView внутри GTK-контейнеров:
  * Открываются **любые сайты** (Google, YouTube, GitHub и др.), которые обычно блокируют встраивание через `iframe`.
  * Фоновые вкладки не выгружаются при переключении (сохраняется скролл, введённый текст и воспроизведение видео).
* 🏠 **Встроенная стартовая страница** — кастомная домашняя страница `material://home` с поиском и быстрыми ссылками, синхронизированная с выбранной темой.
* 🐧 **Дружелюбен к Wayland и X11** — отлично работает на современных композиторах (Hyprland, Sway, KDE Plasma, GNOME).

---

## 📦 Зависимости для сборки

Перед сборкой убедитесь, что у вас установлены системные библиотеки `GTK3` и `WebKit2GTK`:

**Arch Linux / Manjaro:**
```bash
sudo pacman -S webkit2gtk-4.1 gtk3 base-devel
```

**Ubuntu / Debian / Linux Mint:**
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev build-essential
```

**Fedora:**
```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel
```

---

## 🚀 Сборка и запуск

1. **Клонируйте репозиторий:**
   ```bash
   git clone https://github.com/Aidzyki/rust_chrome.git
   cd rust_chrome
   ```

2. **Запустите в режиме разработки:**
   ```bash
   cargo run
   ```
   *(Примечание для пользователей Wayland / NVIDIA: если окно открывается пустым, запускайте с переменной `WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo run`).*

3. **Соберите оптимизированную Release-версию и установите в систему:**
   ```bash
   cargo build --release
   sudo cp target/release/rust_chrome /usr/local/bin/rust-chrome
   ```

4. **Добавьте ярлык в меню приложений (опционально):**
   ```bash
   mkdir -p ~/.local/share/applications
   cat << 'EOF' > ~/.local/share/applications/rust-chrome.desktop
   [Desktop Entry]
   Version=1.0
   Name=Rust Chrome
   Comment=Браузер на Rust в стиле Material You
   Exec=env WEBKIT_DISABLE_DMABUF_RENDERER=1 /usr/local/bin/rust-chrome
   Icon=web-browser
   Terminal=false
   Type=Application
   Categories=Network;WebBrowser;
   StartupWMClass=rust_chrome
   EOF
   ```

---

## 🛠 Технологический стек

* **[Rust](https://www.rust-lang.org/)** — основной язык программирования.
* **[Tao](https://github.com/tauri-apps/tao)** — кроссплатформенное создание окон и цикл событий.
* **[Wry](https://github.com/tauri-apps/wry)** — биндинги к системному WebView (WebKitGTK на Linux).
* **[GTK-rs](https://gtk-rs.org/)** — управление нативными контейнерами вкладок и слоями окна.

---

## 🤖 Благодарности (Credits)

Проект разработан при помощи искусственного интеллекта **Google Gemini Argon 4** (модель **Gemini** от Google).
