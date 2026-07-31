#!/usr/bin/env python3
"""
Voxtera Game Launcher
Downloads updates from GitHub releases and launches the game.
"""

import hashlib
import json
import os
import platform
import socket
import subprocess
import sys
import queue
import threading
import time
from dataclasses import dataclass
from pathlib import Path
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
DEFAULT_INSTALL_DIR = os.path.join(BASE_DIR, "game")


@dataclass(frozen=True)
class PlatformSpec:
    key: str
    archive_prefix: str
    executable_path: tuple[str, ...]
    assets_path: tuple[str, ...]


PLATFORM_SPECS = {
    "Windows": PlatformSpec(
        key="windows-x64",
        archive_prefix="Voxtera-windows-x64-",
        executable_path=("Voxtera.exe",),
        assets_path=("assets",),
    ),
    "Darwin": PlatformSpec(
        key="macos-universal",
        archive_prefix="Voxtera-macos-universal-",
        executable_path=("Voxtera.app", "Contents", "MacOS", "Voxtera"),
        assets_path=("Voxtera.app", "Contents", "Resources", "assets"),
    ),
}


def platform_spec(system_name=None):
    system_name = system_name or platform.system()
    spec = PLATFORM_SPECS.get(system_name)
    if spec is None:
        raise RuntimeError(f"Unsupported platform: {system_name}")
    return spec


def find_platform_archive(release, spec):
    matches = [
        asset
        for asset in release.get("assets", [])
        if asset.get("name", "").startswith(spec.archive_prefix)
        and asset.get("name", "").endswith(".zip")
    ]
    if len(matches) > 1:
        raise RuntimeError(f"Multiple {spec.key} archives found in this release")
    return matches[0] if matches else None


def manifest_sha256_for_platform(manifest, spec, archive_name):
    artifacts = manifest.get("artifacts")
    if isinstance(artifacts, dict):
        artifact = artifacts.get(spec.key)
        if isinstance(artifact, dict) and artifact.get("archive") == archive_name:
            sha256 = artifact.get("sha256")
            return sha256.lower().strip() if isinstance(sha256, str) else None

    # Releases created before platform-specific manifests remain valid on Windows.
    if spec.key == "windows-x64":
        sha256 = manifest.get("zip_sha256")
        return sha256.lower().strip() if isinstance(sha256, str) else None
    return None


def installed_game_path(install_dir, spec):
    return Path(install_dir).joinpath(*spec.executable_path)


def game_launch_environment(install_dir, spec, environ=None):
    environment = dict(os.environ if environ is None else environ)
    assets_dir = Path(install_dir).joinpath(*spec.assets_path)
    if assets_dir.is_dir():
        environment["VELOREN_ASSETS"] = str(assets_dir)
    return environment

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

def _download_file_once(url, dest, progress_cb=None):
    """Single download attempt. Raises on any network/IO error."""
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

def download_file(url, dest, progress_cb=None, status_cb=None):
    """
    Download with retry: up to 3 attempts, 5s pause between attempts.
    Catches URLError, socket.timeout and other transient network errors.
    status_cb(attempt, max_attempts) is called before each attempt so the
    UI can show "Tentativa X/3".
    """
    max_attempts = 3
    last_exc = None
    for attempt in range(1, max_attempts + 1):
        if status_cb:
            status_cb(attempt, max_attempts)
        try:
            _download_file_once(url, dest, progress_cb)
            return  # success
        except (URLError, HTTPError, socket.timeout, ConnectionError, TimeoutError) as e:
            last_exc = e
            # Clean up partial file before retrying
            try:
                if os.path.exists(dest):
                    os.remove(dest)
            except OSError:
                pass
            if attempt < max_attempts:
                time.sleep(5)
            else:
                raise
        except Exception as e:
            # Unexpected error — clean up and re-raise immediately
            try:
                if os.path.exists(dest):
                    os.remove(dest)
            except OSError:
                pass
            raise
    # Should not reach here, but just in case
    if last_exc:
        raise last_exc

def parse_version(v):
    if not v:
        return (0,)
    v = v.lstrip("v")
    try:
        return tuple(int(x) for x in v.split("."))
    except ValueError:
        return (v,)

# ── Manifest / Integrity ───────────────────────────────────────────────────────

def compute_sha256(path, progress_cb=None, chunk_size=65536):
    """Compute SHA-256 of a file, optionally reporting bytes processed."""
    h = hashlib.sha256()
    processed = 0
    with open(path, "rb") as f:
        while True:
            chunk = f.read(chunk_size)
            if not chunk:
                break
            h.update(chunk)
            processed += len(chunk)
            if progress_cb:
                progress_cb(processed)
    return h.hexdigest()

def fetch_manifest(manifest_url):
    """Download and parse the manifest JSON for a release."""
    data = api_get(manifest_url)
    return data

def find_manifest_url(release):
    """
    Locate the manifest asset URL in a GitHub release.
    Looks for an asset named manifest-v{version}.json, falls back to any
    asset starting with 'manifest-' and ending with '.json'.
    """
    tag = release.get("tag_name", "").lstrip("v")
    expected = f"manifest-v{tag}.json"
    for asset in release.get("assets", []):
        if asset["name"] == expected:
            return asset["browser_download_url"]
    # Fallback: any manifest-*.json asset
    for asset in release.get("assets", []):
        name = asset["name"]
        if name.startswith("manifest-") and name.endswith(".json"):
            return asset["browser_download_url"]
    return None

# ── Main Application ───────────────────────────────────────────────────────────

class VoxteraLauncher(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("Voxtera")
        self.geometry("520x700")
        self.resizable(False, False)
        self.configure(bg=BG_DARK)

        self.cfg = load_config()
        self.platform = platform_spec()
        self.latest_version = None
        self.download_url = None
        self.download_asset = None
        self.manifest_url = None
        self._downloading = False
        self._ui_queue = queue.Queue()
        self._tk_alive = True
        self._pump_stop = threading.Event()

        self._build_ui()
        self._start_queue_pump_thread()
        self._check_updates_thread()

    def _safe_after(self, func):
        """Schedule a UI update from any thread.

        Tk widgets are not thread-safe; on macOS PyInstaller bundles the
        worker thread often invokes ``self.after(0, ...)`` while the main
        loop is not in the right apartment, which raises
        ``RuntimeError: main thread is not in main loop``.
        We catch that and fall back to a thread-safe ``queue.Queue``
        drained by ``_pump_dispatch`` from a dedicated pump thread.
        """
        if not self._tk_alive:
            return
        try:
            self.after(0, func)
            return
        except RuntimeError:
            self._ui_queue.put(func)

    def _pump_dispatch(self):
        """Drain the queue from the pump thread.

        Each callback is dispatched via ``self.after(0, ...)`` so that
        the actual widget mutation runs on the main thread (Tk's event
        loop). If ``self.after`` itself is broken (e.g. mainloop not
        yet running), the callback is re-enqueued for the next cycle.
        """
        if not self._tk_alive:
            return
        batch = []
        while True:
            try:
                batch.append(self._ui_queue.get_nowait())
            except queue.Empty:
                break
        for func in batch:
            try:
                self.after(0, func)
            except RuntimeError:
                # mainloop not ready yet – re-enqueue for later
                self._ui_queue.put(func)
                break  # stop processing; retry next pump cycle

    def _start_queue_pump_thread(self):
        """Run the queue pump on a daemon thread.

        Unlike the previous ``self.after``-based pump, this does NOT
        depend on ``self.after`` for the pump loop itself. The pump
        thread runs unconditionally and dispatches to Tk via
        ``self.after(0, ...)`` when possible, re-enqueuing on failure.
        """
        def _loop():
            while not self._pump_stop.is_set():
                try:
                    self._pump_dispatch()
                except Exception:
                    pass
                self._pump_stop.wait(0.05)  # 50 ms interval

        t = threading.Thread(target=_loop, daemon=True, name="voxtera-pump")
        t.start()

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
                img = tk.PhotoImage(file=logo_path)
                scale = max(1, (max(img.width(), img.height()) + 249) // 250)
                self._logo_img = img.subsample(scale, scale)
                tk.Label(main, image=self._logo_img, bg=BG_DARK).pack(pady=(10, 5))
            except tk.TclError:
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
        self.update_btn.pack(pady=(0, 8))
        self.update_btn.config(state="disabled")

        self.repair_btn = tk.Button(main, text="✦  REPARAR", bg=ACCENT,
                                     fg=TEXT_PRIMARY, activebackground=ACCENT_HOVER,
                                     command=self._repair, **btn_style)
        self.repair_btn.pack(pady=(0, 15))
        self.repair_btn.config(state="disabled")

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
        """Check if the platform-specific game executable exists."""
        return installed_game_path(self.cfg["install_dir"], self.platform).is_file()

    # ── Update Check ───────────────────────────────────────────────────────────

    def _check_updates_thread(self):
        threading.Thread(target=self._do_check_updates, daemon=True).start()

    def _do_check_updates(self):
        try:
            # First check if game is actually installed
            if not self._is_installed():
                self.cfg["installed_version"] = None
                save_config(self.cfg)
                self._safe_after(lambda: self._set_status("Jogo não instalado", ACCENT))
                self._safe_after(lambda: self.local_ver_label.config(text=""))
                self._safe_after(lambda: self.play_btn.config(state="disabled"))
            else:
                local_ver = self.cfg.get("installed_version")
                if local_ver:
                    self._safe_after(lambda: self.local_ver_label.config(
                        text=f"Instalado: {local_ver}"))
                    self._safe_after(lambda: self.play_btn.config(state="normal"))

            releases = api_get(GITHUB_API)
            if not releases:
                if self._is_installed():
                    self._safe_after(lambda: self._set_status(
                        "✓ Instalado (sem verificação de atualização)", GREEN))
                else:
                    self._safe_after(lambda: self._set_status(
                        "Nenhum release encontrado", ACCENT))
                return

            release = releases[0]
            self.latest_version = release["tag_name"]

            self.download_asset = find_platform_archive(release, self.platform)
            self.download_url = (
                self.download_asset["browser_download_url"]
                if self.download_asset is not None
                else None
            )
            self.manifest_url = find_manifest_url(release)

            local_ver = self.cfg.get("installed_version")
            if self.download_url:
                if self._is_installed() and local_ver and parse_version(local_ver) >= parse_version(self.latest_version):
                    self._safe_after(lambda: self._set_status(
                        f"✓ Atualizado ({self.latest_version})", GREEN))
                    self._safe_after(lambda: self.play_btn.config(state="normal"))
                    self._safe_after(lambda: self.repair_btn.config(state="normal"))
                else:
                    self._safe_after(lambda: self._set_status(
                        f"Nova versão: {self.latest_version}", ACCENT))
                    self._safe_after(lambda: self.update_btn.config(state="normal"))
                    if self._is_installed():
                        self._safe_after(lambda: self.play_btn.config(state="normal"))
                        self._safe_after(lambda: self.repair_btn.config(state="normal"))
            else:
                self._safe_after(lambda: self._set_status(
                    f"Sem pacote para {self.platform.key}", ACCENT))

        except Exception as e:
            self._safe_after(lambda: self._set_status(f"Erro: {str(e)[:50]}", ACCENT))

    def _set_status(self, text, color=TEXT_SECONDARY):
        self.version_label.config(text=text, fg=color)

    # ── Download ───────────────────────────────────────────────────────────────

    def _update(self):
        if self._downloading or not self.download_url:
            return
        self._start_download(force=False)

    def _repair(self):
        """Force reinstall of the current/latest version (same version)."""
        if self._downloading or not self.download_url:
            return
        if not messagebox.askyesno(
                "Reparar instalação",
                "Isto vai reinstalar a versão atual, sobrescrevendo os arquivos do jogo.\n\nContinuar?"):
            return
        self._start_download(force=True)

    def _start_download(self, force=False):
        if self._downloading or not self.download_url:
            return
        self._downloading = True
        verb = "REINSTALANDO" if force else "BAIXANDO"
        self.update_btn.config(state="disabled", text=verb if not force else "⟳  ATUALIZAR")
        self.repair_btn.config(state="disabled", text="REINSTALANDO...")
        self.play_btn.config(state="disabled")
        threading.Thread(target=self._do_install, args=(force,), daemon=True).start()

    def _do_install(self, force=False):
        """Shared install/update logic. If force=True, reinstall same version."""
        try:
            install_dir = self.cfg["install_dir"]
            os.makedirs(install_dir, exist_ok=True)
            zip_path = os.path.join(install_dir, "voxtera_update.zip")

            target_version = self.latest_version

            def progress(downloaded, total):
                if total > 0:
                    pct = (downloaded / total) * 100
                    mb = downloaded / (1024 * 1024)
                    total_mb = total / (1024 * 1024)
                    self._safe_after(lambda: self.progress.config(value=pct))
                    self._safe_after(lambda: self.progress_label.config(
                        text=f"{mb:.1f} / {total_mb:.1f} MB ({pct:.0f}%)"))

            def download_status(attempt, max_attempts):
                self._safe_after(lambda: self._set_status(
                    f"Baixando... (Tentativa {attempt}/{max_attempts})", TEXT_SECONDARY))
                self._safe_after(lambda: self.progress_label.config(
                    text=f"Tentativa {attempt}/{max_attempts}"))

            self._safe_after(lambda: self.progress.config(mode="determinate", value=0))
            self._safe_after(lambda: self.progress_label.config(text=""))
            self._safe_after(lambda: self._set_status("Baixando...", TEXT_SECONDARY))
            download_file(self.download_url, zip_path, progress, status_cb=download_status)

            # ── SHA-256 verification via manifest ────────────────────────────────
            expected_sha = None
            if self.manifest_url:
                self._safe_after(lambda: self._set_status("Verificando integridade...", TEXT_SECONDARY))
                self._safe_after(lambda: self.progress.config(mode="indeterminate"))
                self._safe_after(lambda: self.progress.start(15))
                try:
                    manifest = fetch_manifest(self.manifest_url)
                    expected_sha = manifest_sha256_for_platform(
                        manifest,
                        self.platform,
                        self.download_asset["name"],
                    )
                except Exception as me:
                    self._safe_after(lambda: self.progress.stop())
                    self._safe_after(lambda: self.progress.config(mode="determinate", value=0))
                    raise RuntimeError(f"Falha ao obter manifest: {str(me)[:80]}")

                if not expected_sha:
                    self._safe_after(lambda: self.progress.stop())
                    self._safe_after(lambda: self.progress.config(mode="determinate", value=0))
                    raise RuntimeError(
                        f"Manifest sem SHA-256 para {self.platform.key}")

                self._safe_after(lambda: self.progress.stop())
                self._safe_after(lambda: self.progress.config(mode="determinate"))
                self._safe_after(lambda: self._set_status(
                    "Calculando SHA-256...", TEXT_SECONDARY))

                zip_size = os.path.getsize(zip_path)
                def hash_progress(processed):
                    if zip_size > 0:
                        pct = (processed / zip_size) * 100
                        self._safe_after(lambda p=pct: self.progress.config(value=p))

                actual_sha = compute_sha256(zip_path, hash_progress)

                if actual_sha.lower() != expected_sha:
                    try:
                        os.remove(zip_path)
                    except OSError:
                        pass
                    self._safe_after(lambda: self.progress.config(value=0))
                    raise RuntimeError(
                        f"SHA-256 não confere!\n"
                        f"  Esperado: {expected_sha[:16]}...\n"
                        f"  Obtido:   {actual_sha[:16]}...\n"
                        f"O arquivo pode estar corrompido.")
                self._safe_after(lambda: self._set_status(
                    "✓ Integridade verificada", GREEN))
            else:
                # No manifest found — proceed but warn
                self._safe_after(lambda: self._set_status(
                    "⚠ Sem manifest (integridade não verificada)", ACCENT))

            # ── Extract ──────────────────────────────────────────────────────────
            self._safe_after(lambda: self._set_status("Extraindo...", TEXT_SECONDARY))
            self._safe_after(lambda: self.progress.config(mode="indeterminate"))
            self._safe_after(lambda: self.progress.start(15))

            with zipfile.ZipFile(zip_path, "r") as zf:
                zf.extractall(install_dir)
            os.remove(zip_path)

            self.cfg["installed_version"] = target_version
            save_config(self.cfg)

            self._safe_after(lambda: self.progress.stop())
            self._safe_after(lambda: self.progress.config(mode="determinate", value=100))
            self._safe_after(lambda: self.progress_label.config(text=""))
            self._safe_after(lambda: self._set_status(
                f"✓ Instalado ({target_version})", GREEN))
            self._safe_after(lambda: self.local_ver_label.config(
                text=f"Instalado: {target_version}"))
            self._safe_after(lambda: self.play_btn.config(state="normal"))
            self._safe_after(lambda: self.update_btn.config(text="⟳  ATUALIZAR", state="disabled"))
            self._safe_after(lambda: self.repair_btn.config(text="✦  REPARAR", state="normal"))

        except Exception as e:
            err_msg = str(e)[:120]
            self._safe_after(lambda: self._set_status(f"Erro: {err_msg}", ACCENT))
            self._safe_after(lambda: messagebox.showerror("Erro", str(e)))
            self._safe_after(lambda: self.update_btn.config(text="⟳  ATUALIZAR", state="normal"))
            self._safe_after(lambda: self.repair_btn.config(text="✦  REPARAR", state="normal"))
            try:
                self._safe_after(lambda: self.progress.stop())
                self._safe_after(lambda: self.progress.config(mode="determinate", value=0))
                self._safe_after(lambda: self.progress_label.config(text=""))
            except Exception:
                pass
        finally:
            self._downloading = False

    # ── Actions ────────────────────────────────────────────────────────────────

    def _play(self):
        game_path = installed_game_path(self.cfg["install_dir"], self.platform)
        if game_path.is_file():
            subprocess.Popen(
                [str(game_path)],
                cwd=self.cfg["install_dir"],
                env=game_launch_environment(self.cfg["install_dir"], self.platform),
            )
            self.destroy()
        else:
            messagebox.showerror(
                "Erro",
                f"{game_path.name} não encontrado.\nBaixe o jogo primeiro.",
            )
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


    def destroy(self):
        # Signal worker threads to stop calling self.after.
        self._tk_alive = False
        try:
            super().destroy()
        except Exception:
            pass

# ── Entry Point ────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app = VoxteraLauncher()
    app.mainloop()
