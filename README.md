
# Vibe Coder

**Vibe Coder** is a Windows desktop application designed to discreetly capture screen content, extract programming problems from screenshots using AI, and display the solutions in a semi-transparent overlay.

[Research Link](https://docs.google.com/document/d/1EF62WxdENCETVOl7A_AW8fIoFt2Evum5wBA71i1s538/edit?usp=sharing)

---

## 🧩 Overview

Vibe Coder is built for stealth and usability. Its primary function is to:
- **Capture** screenshots
- **Send** them to a proxy server for AI analysis (e.g., GPT-4.1)
- **Display** code solutions and explanations in an always-on-top overlay window

Key stealth features include:
- Hidden from the taskbar
- No focus-stealing
- Attempts to avoid being captured in screen shares or screenshots

---

## ✨ Features

### 🪟 Overlay Window
- Semi-transparent black window (~70% opacity)
- Always-on-top, hidden from the taskbar
- No focus steal when shown
- Attempts to prevent capture using `SetWindowDisplayAffinity`

### 🎛️ Global Hotkeys

| Hotkey             | Action                                                             |
|--------------------|--------------------------------------------------------------------|
| `Ctrl + B`         | Toggle overlay visibility                                          |
| `Ctrl + H`         | Capture screen and preview in overlay                              |
| `Ctrl + Enter`     | Send captured image to AI (must follow `Ctrl + H`)                 |
| `Ctrl + R`         | Reset overlay to welcome state                                     |
| `Ctrl + ArrowKeys` | Move the overlay pixel-by-pixel                                    |
| `Ctrl + +/-`       | Resize overlay window                                              |

### 📡 AI Interaction
- Sends Base64-encoded PNG and prompt to a **proxy server**
- Receives and displays plain text AI response
- Prompt includes request for code solution, explanation, and complexity

### 📱 UI States
- **Welcome**: Initial instructions
- **Captured**: Screenshot preview
- **Sending**: Request in progress
- **Response**: AI-generated code and explanation

---

## ⚙️ Setup & Running

### 🔧 Prerequisites
- [Rust](https://rustup.rs) installed
- A running [proxy server](#proxy-server-requirement)

### 🚀 Running the App
1. Update the proxy URL in `src/main.rs`:

   ```rust
   const PROXY_SERVER_URL: &str = "http://127.0.0.1:3000/process-image";
   ```

2. Compile and run the application:

   ```bash
   cargo build
   ```
   ```bash
   cargo run
   ```
3. Use the hotkeys to interact with the application.

---

## 🧪 Dependencies

| Crate         | Purpose                                      |
|---------------|----------------------------------------------|
| `windows`     | Win32 API integration                        |
| `once_cell`   | Safe one-time initialization                 |
| `reqwest`     | HTTP requests to proxy                       |
| `image`       | Bitmap encoding to PNG                       |
| `base64`      | Encode image as Base64                       |
| `serde`       | JSON serialization of requests               |
| `serde_json`  | JSON handling for HTTP payloads              |

---

## 🌐 Proxy Server Requirement

This app requires a separate backend proxy (e.g., Python Flask) to:
- Securely store the **OpenAI API key**
- Receive image data from Vibe Coder
- Forward requests to OpenAI
- Return AI-generated responses to the app

> A sample Python proxy server is included separately in `python_proxy_server_1`.

---

## 🚧 Limitations & Future Work

- **UI**: GDI-based — lacks syntax highlighting or scrollbars
- **Stealth**: Not hidden from Task Manager
- **Error Handling**: Limited user feedback for network or processing issues
- **Configuration**: Proxy URL is hardcoded — should be externalized
- **Cross-platform**: Windows only

---

## ⚠️ Disclaimer

Vibe Coder is an **experimental** tool intended solely for **educational and personal productivity** purposes. It is designed to explore the technical possibilities of AI-assisted coding workflows, overlay rendering, and Windows UI behavior.

We **do not condone or endorse** the use of this tool for academic dishonesty, cheating on assessments, or violating institutional or workplace policies.

By using this software, you acknowledge that:

- You are solely responsible for how you use it.
- The creators accept **no liability** for misuse or consequences arising from its deployment.
- You should always comply with the rules and guidelines of your educational institution or employer.

**Use responsibly. Learn ethically.**

---
