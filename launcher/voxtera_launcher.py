#!/usr/bin/env python3
"""
Voxtera Game Launcher
Downloads updates from GitHub releases and launches the game.
GitHub repo: https://github.com/Stoltemberg/voxtera
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
import io
import shutil

# ── Constants ──────────────────────────────────────────────────────────────────
GITHUB_REPO = "Stoltemberg/voxtera"
GITHUB_API = f"https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
CONFIG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "voxtera_config.json")
GAME_EXE = "Voxtera.exe"
DEFAULT_INSTALL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "game")

# ── Config helpers ─────────────────────────────────────────────────────────────

def load_config():
    if os.path.exists(CONFIG_FILE):
        with open(CONFIG_FILE, "r") as f:
            return json.load(f)
    return {"install_dir": DEFAULT_INSTALL_DIR, "installed_version": None}

def save_config(cfg):
    with open(CONFIG_FILE, "w") as f:
        json.dump(cfg, f, indent=2)

# ── Network helpers ────────────────────────────────────────────────────────────

def api_get(url, timeout=30):
    """GET a JSON API endpoint. Returns parsed JSON or raises."""
    req = Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "VoxteraLauncher"})
    with urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode())

def download_file(url, dest, progress_cb=None):
    """Download a file, calling progress_cb(bytes_read, total) periodically."""
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

# ── Version comparison ─────────────────────────────────────────────────────────

def parse_version(v):
    """Strip leading 'v' and split into int tuple for comparison."""
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
        self.title("Voxtera Launcher")
        self.geometry("520x420")
        self.resizable(False, False)
        self.configure(bg="#1a1a2e")

        self.cfg = load_config()
        self.latest_release = None
        self.latest_version = None
        self.download_url = None
        self._downloading = False

        self._build_ui()
        self._check_updates_thread()

    # ── UI ─────────────────────────────────────────────────────────────────────

    def _build_ui(self):
        bg = "#1a1a2e"
        fg = "#e0e0e0"
        accent = "#e94560"

        # Logo / title
        tk.Label(self, text="VOXTERA", font=("Consolas", 36, "bold"),
                 bg=bg, fg=accent).pack(pady=(30, 5))
        tk.Label(self, text="Voxel RPG", font=("Consolas", 12),
                 bg=bg, fg=fg).pack(pady=(0, 20))

        # Version info
        self.version_label = tk.Label(self, text="Checking for updates…",
                                       font=("Consolas", 10), bg=bg, fg="#aaa")
        self.version_label.pack(pady=(0, 5))

        self.local_ver_label = tk.Label(self, text="", font=("Consolas", 9),
                                         bg=bg, fg="#777")
        self.local_ver_label.pack(pady=(0, 10))

        # Progress bar
        self.progress = ttk.Progressbar(self, length=400, mode="determinate")
        self.progress.pack(pady=5)
        self.progress["value"] = 0

        self.progress_label = tk.Label(self, text="", font=("Consolas", 9),
                                        bg=bg, fg="#aaa")
        self.progress_label.pack(pady=(0, 10))

        # Buttons frame
        btn_frame = tk.Frame(self, bg=bg)
        btn_frame.pack(pady=10)

        style_btn = {"font": ("Consolas", 12, "bold"), "width": 14, "height": 2,
                     "bd": 0, "cursor": "hand2"}

        self.play_btn = tk.Button(btn_frame, text="▶  PLAY", bg="#0f3460",
                                   fg=fg, activebackground="#16213e",
                                   command=self._play, **style_btn)
        self.play_btn.grid(row=0, column=0, padx=10)

        self.update_btn = tk.Button(btn_frame, text="⟳  UPDATE", bg=accent,
                                     fg="#fff", activebackground="#c81e45",
                                     command=self._update, **style_btn)
        self.update_btn.grid(row=0, column=1, padx=10)

        self.update_btn.config(state="disabled")

        # Install dir label
        dir_frame = tk.Frame(self, bg=bg)
        dir_frame.pack(pady=(15, 0), fill="x", padx=30)
        tk.Label(dir_frame, text="Install:", font=("Consolas", 8),
                 bg=bg, fg="#555").pack(side="left")
        self.dir_label = tk.Label(dir_frame, text=self.cfg["install_dir"],
                                   font=("Consolas", 8), bg=bg, fg="#555",
                                   anchor="w")
        self.dir_label.pack(side="left", fill="x", expand=True)
        tk.Button(dir_frame, text="📁", font=("Consolas", 8), bg=bg, fg="#888",
                  bd=0, command=self._choose_dir).pack(side="right")

    def _choose_dir(self):
        d = filedialog.askdirectory(initialdir=self.cfg["install_dir"])
        if d:
            self.cfg["install_dir"] = d
            save_config(self.cfg)
            self.dir_label.config(text=d)

    def _set_status(self, text):
        self.version_label.config(text=text)

    def _set_progress(self, value, maximum=100):
        self.progress["value"] = value
        self.progress["maximum"] = maximum

    def _set_progress_text(self, text):
        self.progress_label.config(text=text)

    # ── Update check (background) ─────────────────────────────────────────────

    def _check_updates_thread(self):
        t = threading.Thread(target=self._check_updates_worker, daemon=True)
        t.start()

    def _check_updates_worker(self):
        try:
            release = api_get(GITHUB_API)
            self.latest_release = release
            self.latest_version = release.get("tag_name", "unknown")

            # Find ZIP asset
            for asset in release.get("assets", []):
                name = asset["name"].lower()
                if name.endswith(".zip"):
                    self.download_url = asset["browser_download_url"]
                    break

            self.after(0, self._on_check_done)
        except Exception as e:
            self.after(0, lambda: self._set_status(f"Update check failed: {e}"))
            self.after(0, lambda: self._update_local_ver())

    def _on_check_done(self):
        self._update_local_ver()
        if self._has_update():
            self._set_status(f"Update available: {self.latest_version}")
            self.update_btn.config(state="normal")
        else:
            self._set_status("Up to date ✓")
            self.update_btn.config(state="disabled")

    def _update_local_ver(self):
        local = self.cfg.get("installed_version")
        if local:
            self.local_ver_label.config(text=f"Installed: {local}")
        else:
            self.local_ver_label.config(text="Not installed")

    def _has_update(self):
        if not self.cfg.get("installed_version"):
            return True
        return parse_version(self.latest_version) > parse_version(self.cfg["installed_version"])

    # ── Download / Update ─────────────────────────────────────────────────────

    def _update(self):
        if self._downloading:
            return
        if not self.download_url:
            messagebox.showerror("Error", "No download URL found.")
            return
        self._downloading = True
        self.update_btn.config(state="disabled", text="Updating…")
        self.play_btn.config(state="disabled")
        t = threading.Thread(target=self._download_worker, daemon=True)
        t.start()

    def _download_worker(self):
        try:
            install_dir = self.cfg["install_dir"]
            os.makedirs(install_dir, exist_ok=True)

            def on_progress(read, total):
                if total > 0:
                    pct = read / total * 100
                    mb_read = read / (1024 * 1024)
                    mb_total = total / (1024 * 1024)
                    self.after(0, self._set_progress, pct)
                    self.after(0, self._set_progress_text,
                               f"{mb_read:.1f} / {mb_total:.1f} MB")

            # Download ZIP to temp file
            zip_path = os.path.join(install_dir, "_update.zip")
            self.after(0, self._set_progress_text, "Downloading…")
            download_file(self.download_url, zip_path, on_progress)

            # Extract
            self.after(0, self._set_progress_text, "Extracting…")
            with zipfile.ZipFile(zip_path, "r") as zf:
                # Detect top-level folder in ZIP (e.g. "voxtera-0.1.0/...")
                top_levels = set()
                for name in zf.namelist():
                    parts = name.split("/")
                    if parts[0]:
                        top_levels.add(parts[0])

                if len(top_levels) == 1 and not any(
                    name == list(top_levels)[0] + "/" + GAME_EXE for name in zf.namelist()
                ):
                    # Strip top-level dir if it's just a wrapper
                    strip_prefix = list(top_levels)[0] + "/"
                    for member in zf.namelist():
                        if member.startswith(strip_prefix):
                            rel = member[len(strip_prefix):]
                            if not rel:
                                continue
                            target = os.path.join(install_dir, rel)
                            if member.endswith("/"):
                                os.makedirs(target, exist_ok=True)
                            else:
                                os.makedirs(os.path.dirname(target), exist_ok=True)
                                with zf.open(member) as src, open(target, "wb") as dst:
                                    shutil.copyfileobj(src, dst)
                else:
                    zf.extractall(install_dir)

            # Cleanup temp zip
            os.remove(zip_path)

            # Save version
            self.cfg["installed_version"] = self.latest_version
            save_config(self.cfg)

            self.after(0, self._on_update_done)
        except Exception as e:
            self.after(0, lambda: messagebox.showerror("Update Error", str(e)))
            self.after(0, self._on_update_done)

    def _on_update_done(self):
        self._downloading = False
        self._set_progress(100)
        self._set_progress_text("Done ✓")
        self._update_local_ver()
        self.play_btn.config(state="normal")
        if self._has_update():
            self.update_btn.config(state="normal", text="⟳  UPDATE")
        else:
            self.update_btn.config(state="disabled", text="⟳  UPDATE")

    # ── Play ───────────────────────────────────────────────────────────────────

    def _play(self):
        exe = os.path.join(self.cfg["install_dir"], GAME_EXE)
        if not os.path.exists(exe):
            messagebox.showerror("Game Not Found",
                                 f"{GAME_EXE} not found in:\n{self.cfg['install_dir']}\n\n"
                                 "Click UPDATE to download the game first.")
            return
        try:
            subprocess.Popen([exe], cwd=self.cfg["install_dir"])
            self.destroy()
        except Exception as e:
            messagebox.showerror("Launch Error", str(e))

# ── Entry point ────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app = VoxteraLauncher()
    app.mainloop()
