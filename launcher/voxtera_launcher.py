#!/usr/bin/env python3
"""
Voxtera Game Launcher
Downloads updates from GitHub releases and launches the game.
"""

import json
import os
import subprocess
import sys
import threading
import tkinter as tk
from tkinter import ttk, messagebox, filedialog
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError
import zipfile

# ── Path helpers ───────────────────────────────────────────────────────────────

def get_base_dir():
    if getattr(sys, 'frozen', False):
        return os.path.dirname(sys.executable)
    return os.path.dirname(os.path.abspath(__file__))

BASE_DIR = get_base_dir()

# ── Constants ──────────────────────────────────────────────────────────────────
GITHUB_REPO = "Stoltemberg/voxtera"
GITHUB_API = f"https://api.github.com/repos/{GITHUB_REPO}/releases"
CONFIG_FILE = os.path.join(BASE_DIR, "voxtera_config.json")
GAME_EXE = "Voxtera.exe"
DEFAULT_INSTALL_DIR = os.path.join(BASE_DIR, "game")

# ── Theme ──────────────────────────────────────────────────────────────────────
BG_DARK = "#0d1117"
BG_MEDIUM = "#161b22"
BG_LIGHT = "#21262d"
BG_CARD = "#1c2128"
ACCENT = "#e94560"
ACCENT_HOVER = "#c81e45"
GREEN = "#3fb950"
GREEN_DARK = "#238636"
TEXT_PRIMARY = "#e6edf3"
TEXT_SECONDARY = "#8b949e"
TEXT_DIM = "#484f58"
BORDER = "#30363d"

# ── Config ─────────────────────────────────────────────────────────────────────

def load_config():
    defaults = {
        "install_dir": DEFAULT_INSTALL_DIR,
        "installed_version": None,
    }
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE, "r") as f:
                content = f.read()
            try:
                saved = json.loads(content)
            except (json.JSONDecodeError, ValueError):
                try:
                    os.remove(CONFIG_FILE)
                except:
                    pass
                return defaults
            for key in defaults:
                if key not in saved:
                    saved[key] = defaults[key]
            return saved
        except Exception:
            try:
                os.remove(CONFIG_FILE)
            except:
                pass
    return defaults

def save_config(cfg):
    with open(CONFIG_FILE, "w") as f:
        json.dump(cfg, f, indent=2)

# ── Network ────────────────────────────────────────────────────────────────────

def api_get(url, timeout=30):
    req = Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "VoxteraLauncher"})
    with urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode())

def download_file(url, dest, progress_cb=None):
    req = Request(url, headers={"User-Agent": "VoxteraLauncher"})
    with urlopen(req, timeout=120) as resp:
        total = int(resp.headers.get("Content-Length", 0))
        downloaded = 0
        chunk_size = 65536
        with open(dest, "wb") as f:
            while True:
                chunk = resp.read(chunk_size)
                if not chunk:
                    break
                f.write(chunk)
                downloaded += len(chunk)
                if progress_cb:
                    progress_cb(downloaded, total)

def parse_version(v):
    if not v:
        return (0,)
    v = v.lstrip("v")
    try:
        return tuple(int(x) for x in v.split("."))
    except ValueError:
        return (v,)

# ── Main Application ───────────────────────────────────────────────────────────

class VoxteraLauncher(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("Voxtera")
        self.geometry("520x700")
        self.resizable(False, False)
        self.configure(bg=BG_DARK)

        self.cfg = load_config()
        self.latest_version = None
        self.download_url = None
        self._downloading = False

        self._build_ui()
        self._check_updates_thread()

    # ── UI ─────────────────────────────────────────────────────────────────────

    def _build_ui(self):
        # Main container
        main = tk.Frame(self, bg=BG_DARK)
        main.pack(fill="both", expand=True, padx=30, pady=20)

        # ── Logo ───────────────────────────────────────────────────────────────
        # When frozen (PyInstaller), files are extracted to sys._MEIPASS temp dir
        if getattr(sys, 'frozen', False):
            logo_path = os.path.join(sys._MEIPASS, "voxtera_logo.png")
        else:
            logo_path = os.path.join(BASE_DIR, "voxtera_logo.png")

        if os.path.exists(logo_path):
            try:
                from PIL import Image, ImageTk
                img = Image.open(logo_path)
                img = img.resize((250, int(250 * img.height / img.width)), Image.LANCZOS)
                self._logo_img = ImageTk.PhotoImage(img)
                tk.Label(main, image=self._logo_img, bg=BG_DARK).pack(pady=(10, 5))
            except ImportError:
                tk.Label(main, text="VOXTERA", font=("Consolas", 42, "bold"),
                         bg=BG_DARK, fg=ACCENT).pack(pady=(20, 5))
        else:
            tk.Label(main, text="VOXTERA", font=("Consolas", 42, "bold"),
                     bg=BG_DARK, fg=ACCENT).pack(pady=(20, 5))

        tk.Label(main, text="Voxel RPG", font=("Consolas", 12),
                 bg=BG_DARK, fg=TEXT_SECONDARY).pack(pady=(0, 20))

        # ── Status ─────────────────────────────────────────────────────────────
        status_frame = tk.Frame(main, bg=BG_MEDIUM, highlightbackground=BORDER,
                                highlightthickness=1)
        status_frame.pack(fill="x", pady=(0, 10), ipady=10)

        self.version_label = tk.Label(status_frame, text="Verificando atualizações...",
                                       font=("Consolas", 10), bg=BG_MEDIUM, fg=TEXT_SECONDARY)
        self.version_label.pack(pady=(5, 2))

        self.local_ver_label = tk.Label(status_frame, text="", font=("Consolas", 9),
                                         bg=BG_MEDIUM, fg=TEXT_DIM)
        self.local_ver_label.pack(pady=(0, 5))

        # ── Progress ───────────────────────────────────────────────────────────
        style = ttk.Style()
        style.theme_use("default")
        style.configure("Voxtera.Horizontal.TProgressbar",
                        troughcolor=BG_LIGHT, background=ACCENT,
                        darkcolor=ACCENT, lightcolor=ACCENT,
                        bordercolor=BG_DARK, relief="flat")

        self.progress = ttk.Progressbar(main, mode="determinate",
                                         style="Voxtera.Horizontal.TProgressbar")
        self.progress.pack(fill="x", pady=(0, 3))
        self.progress["value"] = 0

        self.progress_label = tk.Label(main, text="", font=("Consolas", 8),
                                        bg=BG_DARK, fg=TEXT_DIM)
        self.progress_label.pack(anchor="w", pady=(0, 15))

        # ── Buttons ────────────────────────────────────────────────────────────
        btn_style = {"font": ("Consolas", 14, "bold"), "width": 22, "height": 2,
                     "bd": 0, "cursor": "hand2", "relief": "flat"}

        self.play_btn = tk.Button(main, text="▶  JOGAR", bg=GREEN,
                                   fg=TEXT_PRIMARY, activebackground=GREEN_DARK,
                                   command=self._play, **btn_style)
        self.play_btn.pack(pady=(0, 8))

        self.update_btn = tk.Button(main, text="⟳  ATUALIZAR", bg=ACCENT,
                                     fg=TEXT_PRIMARY, activebackground=ACCENT_HOVER,
                                     command=self._update, **btn_style)
        self.update_btn.pack(pady=(0, 15))
        self.update_btn.config(state="disabled")

        # ── Install dir ────────────────────────────────────────────────────────
        dir_frame = tk.Frame(main, bg=BG_CARD, highlightbackground=BORDER,
                             highlightthickness=1)
        dir_frame.pack(fill="x", pady=(0, 10), ipady=5)

        tk.Label(dir_frame, text="Pasta:", font=("Consolas", 9),
                 bg=BG_CARD, fg=TEXT_DIM).pack(side="left", padx=10)

        self.dir_label = tk.Label(dir_frame, text=self.cfg["install_dir"],
                                   font=("Consolas", 8), bg=BG_CARD, fg=TEXT_SECONDARY)
        self.dir_label.pack(side="left", expand=True, fill="x", padx=5)

        tk.Button(dir_frame, text="Alterar", bg=BG_LIGHT, fg=TEXT_PRIMARY,
                  font=("Consolas", 8), bd=0, cursor="hand2",
                  command=self._change_install_dir).pack(side="right", padx=10)

        # ── Footer ─────────────────────────────────────────────────────────────
        tk.Label(main, text="v0.1.0", font=("Consolas", 8),
                 bg=BG_DARK, fg=TEXT_DIM).pack(side="bottom")

    # ── Install Check ─────────────────────────────────────────────────────────

    def _is_installed(self):
        """Check if the game EXE actually exists in the install directory."""
        game_path = os.path.join(self.cfg["install_dir"], GAME_EXE)
        return os.path.isfile(game_path)

    # ── Update Check ───────────────────────────────────────────────────────────

    def _check_updates_thread(self):
        threading.Thread(target=self._do_check_updates, daemon=True).start()

    def _do_check_updates(self):
        try:
            # First check if game is actually installed
            if not self._is_installed():
                self.cfg["installed_version"] = None
                save_config(self.cfg)
                self.after(0, lambda: self._set_status("Jogo não instalado", ACCENT))
                self.after(0, lambda: self.local_ver_label.config(text=""))
                self.after(0, lambda: self.play_btn.config(state="disabled"))
            else:
                local_ver = self.cfg.get("installed_version")
                if local_ver:
                    self.after(0, lambda: self.local_ver_label.config(
                        text=f"Instalado: {local_ver}"))
                    self.after(0, lambda: self.play_btn.config(state="normal"))

            releases = api_get(GITHUB_API)
            if not releases:
                if self._is_installed():
                    self.after(0, lambda: self._set_status(
                        "✓ Instalado (sem verificação de atualização)", GREEN))
                else:
                    self.after(0, lambda: self._set_status(
                        "Nenhum release encontrado", ACCENT))
                return

            release = releases[0]
            self.latest_version = release["tag_name"]

            for asset in release.get("assets", []):
                if asset["name"].endswith(".zip"):
                    self.download_url = asset["browser_download_url"]
                    break

            local_ver = self.cfg.get("installed_version")
            if self.download_url:
                if self._is_installed() and local_ver and parse_version(local_ver) >= parse_version(self.latest_version):
                    self.after(0, lambda: self._set_status(
                        f"✓ Atualizado ({self.latest_version})", GREEN))
                    self.after(0, lambda: self.play_btn.config(state="normal"))
                else:
                    self.after(0, lambda: self._set_status(
                        f"Nova versão: {self.latest_version}", ACCENT))
                    self.after(0, lambda: self.update_btn.config(state="normal"))
                    if self._is_installed():
                        self.after(0, lambda: self.play_btn.config(state="normal"))

        except Exception as e:
            self.after(0, lambda: self._set_status(f"Erro: {str(e)[:50]}", ACCENT))

    def _set_status(self, text, color=TEXT_SECONDARY):
        self.version_label.config(text=text, fg=color)

    # ── Download ───────────────────────────────────────────────────────────────

    def _update(self):
        if self._downloading or not self.download_url:
            return
        self._downloading = True
        self.update_btn.config(state="disabled", text="BAIXANDO...")
        self.play_btn.config(state="disabled")
        threading.Thread(target=self._do_update, daemon=True).start()

    def _do_update(self):
        try:
            install_dir = self.cfg["install_dir"]
            os.makedirs(install_dir, exist_ok=True)
            zip_path = os.path.join(install_dir, "voxtera_update.zip")

            def progress(downloaded, total):
                if total > 0:
                    pct = (downloaded / total) * 100
                    mb = downloaded / (1024 * 1024)
                    total_mb = total / (1024 * 1024)
                    self.after(0, lambda: self.progress.config(value=pct))
                    self.after(0, lambda: self.progress_label.config(
                        text=f"{mb:.1f} / {total_mb:.1f} MB ({pct:.0f}%)"))

            self.after(0, lambda: self._set_status("Baixando...", TEXT_SECONDARY))
            download_file(self.download_url, zip_path, progress)

            self.after(0, lambda: self._set_status("Extraindo...", TEXT_SECONDARY))
            self.after(0, lambda: self.progress.config(mode="indeterminate"))
            self.after(0, lambda: self.progress.start(15))

            with zipfile.ZipFile(zip_path, "r") as zf:
                zf.extractall(install_dir)
            os.remove(zip_path)

            self.cfg["installed_version"] = self.latest_version
            save_config(self.cfg)

            self.after(0, lambda: self.progress.stop())
            self.after(0, lambda: self.progress.config(mode="determinate", value=100))
            self.after(0, lambda: self.progress_label.config(text=""))
            self.after(0, lambda: self._set_status(
                f"✓ Instalado ({self.latest_version})", GREEN))
            self.after(0, lambda: self.local_ver_label.config(
                text=f"Instalado: {self.latest_version}"))
            self.after(0, lambda: self.play_btn.config(state="normal"))
            self.after(0, lambda: self.update_btn.config(text="⟳  ATUALIZAR", state="disabled"))

        except Exception as e:
            self.after(0, lambda: self._set_status(f"Erro: {str(e)[:50]}", ACCENT))
            self.after(0, lambda: self.update_btn.config(text="⟳  ATUALIZAR", state="normal"))
        finally:
            self._downloading = False

    # ── Actions ────────────────────────────────────────────────────────────────

    def _play(self):
        game_path = os.path.join(self.cfg["install_dir"], GAME_EXE)
        if os.path.exists(game_path):
            subprocess.Popen([game_path], cwd=self.cfg["install_dir"])
            self.destroy()
        else:
            messagebox.showerror("Erro", f"{GAME_EXE} não encontrado.\nBaixe o jogo primeiro.")
            d = filedialog.askdirectory(title="Selecione a pasta de instalação")
            if d:
                self.cfg["install_dir"] = d
                save_config(self.cfg)
                self.dir_label.config(text=d)

    def _change_install_dir(self):
        d = filedialog.askdirectory(title="Selecione a pasta de instalação")
        if d:
            self.cfg["install_dir"] = d
            save_config(self.cfg)
            self.dir_label.config(text=d)
            # Check if game exists in new directory
            if self._is_installed():
                self.cfg["installed_version"] = self.cfg.get("installed_version") or "unknown"
                save_config(self.cfg)
                self.play_btn.config(state="normal")
                self._set_status("✓ Jogo encontrado na nova pasta", GREEN)
            else:
                self.cfg["installed_version"] = None
                save_config(self.cfg)
                self.play_btn.config(state="disabled")
                self._set_status("Jogo não instalado", ACCENT)
                self.local_ver_label.config(text="")

# ── Entry Point ────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app = VoxteraLauncher()
    app.mainloop()
