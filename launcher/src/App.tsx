import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  Download,
  Play,
  RefreshCw,
  FolderOpen,
  Wrench,
  ScrollText,
  Settings,
  X,
  AlertCircle,
  CheckCircle2,
  Loader2,
} from 'lucide-react';
import voxteraLogo from '../voxtera_logo.png';

/* ---------- Types mirroring the Rust backend ---------- */

interface LauncherSnapshot {
  phase: 'needs_install' | 'ready' | 'update_available' | 'offline' | 'repair_required' | 'error';
  installed_version: string | null;
  available_version: string | null;
  install_dir: string;
  local_build_valid: boolean;
  operation: string | null;
  last_error: LauncherError | null;
}

interface LauncherError {
  code: string;
  message: string;
}

interface RepairPlan {
  checked_files: number;
  checked_bytes: number;
  invalid_files: string[];
  invalid_bytes: number;
  deletions: string[];
}

interface LauncherSettingsInput {
  install_dir: string;
  bandwidth_limit_kib: number | null;
  start_minimized: boolean;
}

interface ProgressEvent {
  operation: string;
  stage: string;
  message?: string;
  percent?: number;
}

/* ---------- UI helpers ---------- */

const PHASE_LABELS: Record<string, string> = {
  needs_install: 'Aguardando instalação',
  ready: 'Pronto para jogar',
  update_available: 'Atualização disponível',
  offline: 'Modo offline',
  repair_required: 'Reparo necessário',
  error: 'Erro',
};

const PHASE_ICONS: Record<string, typeof Play> = {
  needs_install: Download,
  ready: CheckCircle2,
  update_available: RefreshCw,
  offline: AlertCircle,
  repair_required: Wrench,
  error: AlertCircle,
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function formatPhase(phase: string): string {
  return PHASE_LABELS[phase] ?? phase;
}

/* ---------- Main App ---------- */

export function App() {
  const [snapshot, setSnapshot] = useState<LauncherSnapshot | null>(null);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [repairPlan, setRepairPlan] = useState<RepairPlan | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [installDir, setInstallDir] = useState('');
  const [bandwidthLimit, setBandwidthLimit] = useState('');
  const [startMinimized, setStartMinimized] = useState(false);

  /* Load initial snapshot */
  const refreshSnapshot = useCallback(async () => {
    try {
      const snap = await invoke<LauncherSnapshot>('get_snapshot');
      setSnapshot(snap);
      setInstallDir(snap.install_dir);
      setError(snap.last_error?.message ?? null);
    } catch (e: unknown) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refreshSnapshot();
  }, [refreshSnapshot]);

  /* Listen for progress events */
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    listen<ProgressEvent>('launcher://progress', (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  /* ---------- Command handlers ---------- */

  const handleCheckRelease = async () => {
    setBusy(true);
    setError(null);
    try {
      const snap = await invoke<LauncherSnapshot>('check_release');
      setSnapshot(snap);
    } catch (e: unknown) {
      const msg = typeof e === 'string' ? e : (e as {message?: string})?.message ?? String(e);
      setError(msg);
    } finally {
      setBusy(false);
    }
  };

  const handleInstallOrUpdate = async () => {
    setBusy(true);
    setError(null);
    setProgress(null);
    try {
      const snap = await invoke<LauncherSnapshot>('install_or_update');
      setSnapshot(snap);
    } catch (e: unknown) {
      const msg = typeof e === 'string' ? e : (e as {message?: string})?.message ?? String(e);
      setError(msg);
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  const handleCancel = async () => {
    try {
      await invoke('cancel_operation');
      setBusy(false);
      setProgress(null);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleScanRepair = async () => {
    setBusy(true);
    setError(null);
    try {
      const plan = await invoke<RepairPlan>('scan_repair');
      setRepairPlan(plan);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleRepair = async () => {
    setBusy(true);
    setError(null);
    try {
      const snap = await invoke<LauncherSnapshot>('repair');
      setSnapshot(snap);
      setRepairPlan(null);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleChooseDir = async () => {
    try {
      const path = await invoke<string | null>('choose_install_dir');
      if (path) {
        setInstallDir(path);
      }
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleSaveSettings = async () => {
    setBusy(true);
    setError(null);
    try {
      const settings: LauncherSettingsInput = {
        install_dir: installDir,
        bandwidth_limit_kib: bandwidthLimit ? parseInt(bandwidthLimit, 10) : null,
        start_minimized: startMinimized,
      };
      const snap = await invoke<LauncherSnapshot>('save_settings', { settings });
      setSnapshot(snap);
      setShowSettings(false);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleLaunch = async () => {
    setBusy(true);
    setError(null);
    try {
      await invoke<number>('launch_game');
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleOpenLogs = async () => {
    setError(null);
    try {
      await invoke('open_logs');
    } catch (e: unknown) {
      const msg = typeof e === 'string' ? e : (e as {message?: string})?.message ?? String(e);
      setError(msg);
    }
  };

  /* ---------- Derived state ---------- */

  const phase = snapshot?.phase ?? 'needs_install';
  const isInstalled = phase === 'ready' || phase === 'update_available' || phase === 'repair_required';
  const canPlay = phase === 'ready' && !busy;
  const canUpdate = (phase === 'needs_install' || phase === 'update_available') && !busy;
  const PhaseIcon = PHASE_ICONS[phase] ?? AlertCircle;

  const progressPercent = progress?.percent ?? 0;
  const progressLabel = progress?.stage === 'downloading'
    ? `Baixando... ${progressPercent.toFixed(0)}%`
    : progress?.stage === 'extracting'
      ? 'Extraindo arquivos...'
      : progress?.stage === 'verifying'
        ? 'Verificando integridade...'
        : progress?.stage === 'promoting'
          ? 'Finalizando instalação...'
          : progress?.stage ?? '';

  /* ---------- Render ---------- */

  return (
    <main className="launcher-shell">
      <section className="launcher-card" aria-labelledby="launcher-title">
        <img className="launcher-logo" src={voxteraLogo} alt="" />
        <p className="launcher-eyebrow">Launcher oficial</p>

        {/* Status line */}
        <div className="launcher-status">
          <PhaseIcon size={18} className="status-icon" />
          <span className="status-text">{formatPhase(phase)}</span>
          {snapshot?.installed_version && (
            <span className="status-version">v{snapshot.installed_version}</span>
          )}
          {snapshot?.available_version && snapshot.available_version !== snapshot.installed_version && (
            <span className="status-version available">→ v{snapshot.available_version}</span>
          )}
        </div>

        {/* Progress bar */}
        {busy && progress && (
          <div className="progress-bar-container">
            <div className="progress-bar-track">
              <div
                className="progress-bar-fill"
                style={{ width: `${Math.max(2, progressPercent)}%` }}
              />
            </div>
            <p className="progress-label">
              <Loader2 size={14} className="spin" />
              {progressLabel}
            </p>
          </div>
        )}

        {/* Error message */}
        {error && !busy && (
          <div className="error-banner" role="alert">
            <AlertCircle size={16} />
            <span>{error}</span>
            <button className="error-dismiss" onClick={() => setError(null)}>
              <X size={14} />
            </button>
          </div>
        )}

        {/* Repair plan display */}
        {repairPlan && !repairPlan.invalid_files.length && (
          <div className="repair-clean">
            <CheckCircle2 size={16} />
            <span>Verificação concluída: {repairPlan.checked_files} arquivos íntegros</span>
          </div>
        )}
        {repairPlan && repairPlan.invalid_files.length > 0 && (
          <div className="repair-found">
            <AlertCircle size={16} />
            <span>
              {repairPlan.invalid_files.length} arquivo(s) corrompido(s) — {formatBytes(repairPlan.invalid_bytes)}
            </span>
          </div>
        )}

        {/* Action buttons */}
        <div className="launcher-actions">
          {canPlay && (
            <button className="btn-primary" onClick={handleLaunch} disabled={busy}>
              <Play size={18} /> Jogar
            </button>
          )}
          {canUpdate && (
            <button className="btn-primary" onClick={handleInstallOrUpdate} disabled={busy}>
              <Download size={18} /> {phase === 'needs_install' ? 'Instalar' : 'Atualizar'}
            </button>
          )}
          {phase === 'repair_required' && !repairPlan && (
            <button className="btn-primary" onClick={handleScanRepair} disabled={busy}>
              <Wrench size={18} /> Verificar arquivos
            </button>
          )}
          {repairPlan && repairPlan.invalid_files.length > 0 && (
            <button className="btn-primary" onClick={handleRepair} disabled={busy}>
              <Wrench size={18} /> Reparar agora
            </button>
          )}

          {busy && (
            <button className="btn-secondary" onClick={handleCancel}>
              <X size={16} /> Cancelar
            </button>
          )}

          {!busy && isInstalled && (
            <button className="btn-secondary" onClick={handleScanRepair}>
              <RefreshCw size={16} /> Verificar
            </button>
          )}

          {!busy && phase === 'offline' && (
            <button className="btn-secondary" onClick={handleCheckRelease}>
              <RefreshCw size={16} /> Tentar novamente
            </button>
          )}
        </div>

        {/* Secondary actions row */}
        <div className="launcher-secondary">
          <button className="btn-icon" onClick={handleChooseDir} title="Escolher pasta de instalação">
            <FolderOpen size={16} />
          </button>
          <button className="btn-icon" onClick={() => setShowSettings(!showSettings)} title="Configurações">
            <Settings size={16} />
          </button>
          <button className="btn-icon" onClick={handleOpenLogs} title="Abrir logs">
            <ScrollText size={16} />
          </button>
          <button className="btn-icon" onClick={handleCheckRelease} title="Verificar atualizações">
            <RefreshCw size={16} />
          </button>
        </div>

        {/* Install directory display */}
        <p className="install-dir">
          {snapshot?.install_dir ?? installDir ?? 'Pasta não definida'}
        </p>

        {/* Settings panel */}
        {showSettings && (
          <div className="settings-panel" role="dialog" aria-label="Configurações">
            <div className="settings-header">
              <h2>Configurações</h2>
              <button className="btn-icon" onClick={() => setShowSettings(false)}>
                <X size={16} />
              </button>
            </div>
            <label className="settings-field">
              <span>Pasta de instalação</span>
              <div className="settings-input-row">
                <input
                  type="text"
                  value={installDir}
                  onChange={(e) => setInstallDir(e.target.value)}
                  placeholder="C:\Voxtera"
                />
                <button onClick={handleChooseDir}>
                  <FolderOpen size={14} />
                </button>
              </div>
            </label>
            <label className="settings-field">
              <span>Limite de banda (KB/s, 0 = sem limite)</span>
              <input
                type="number"
                value={bandwidthLimit}
                onChange={(e) => setBandwidthLimit(e.target.value)}
                placeholder="0"
              />
            </label>
            <label className="settings-checkbox">
              <input
                type="checkbox"
                checked={startMinimized}
                onChange={(e) => setStartMinimized(e.target.checked)}
              />
              <span>Iniciar minimizado</span>
            </label>
            <button className="btn-primary" onClick={handleSaveSettings} disabled={busy}>
              Salvar
            </button>
          </div>
        )}
      </section>
    </main>
  );
}
