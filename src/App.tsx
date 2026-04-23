import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { motion, AnimatePresence } from 'framer-motion';
import { Clock, FolderInput, HardDrive, History, FileText, Database, Settings, FolderTree, Menu, X, Eye, Zap, Shield } from 'lucide-react';
import { useAppStore, type FileEntry } from './store/useAppStore';
import HistoryPage from './components/Timeline';
import FileExplorer from './components/FileExplorer';
import SettingsPanel from './components/SettingsPanel';
import Notifications from './components/Notifications';

function App() {
  const { currentPath, setCurrentPath, status, setStatus, setFileHistory, addNotification } = useAppStore();
  const [inputPath, setInputPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<'dashboard' | 'explorer' | 'history' | 'settings'>('dashboard');
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const navToFileHistory = async (path: string) => {
    try {
      const fullPath = currentPath + '/' + path;
      const history = await invoke('get_history', { file: fullPath });
      setFileHistory(history as FileEntry);
      setActiveTab('history');
    } catch (e) {
      console.error(e);
    }
  };

  const handleBrowse = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        setInputPath(selected as string);
      }
    } catch (e) {
      console.error("Failed to open dialog", e);
    }
  };

  const handleWatch = async () => {
    if (!inputPath) return;
    setLoading(true);
    try {
      await invoke('watch_directory', { path: inputPath });
      setCurrentPath(inputPath);
      await refreshStatus(inputPath);
      addNotification('success', `Now watching: ${inputPath}`);
    } catch (e: any) {
      addNotification('error', e.toString());
    } finally {
      setLoading(false);
    }
  };

  const refreshStatus = async (path: string) => {
    try {
      const res = await invoke('get_status', { path });
      if (res) {
        setStatus(res as any);
      }
    } catch (e) {
      console.error(e);
    }
  };

  // Poll status occasionally if watching
  useEffect(() => {
    if (!currentPath) return;
    const interval = setInterval(() => {
      refreshStatus(currentPath);
    }, 2000);
    return () => clearInterval(interval);
  }, [currentPath]);

  const handleTabChange = (tab: typeof activeTab) => {
    setActiveTab(tab);
  };

  return (
    <div className="min-h-screen h-screen bg-background text - text-main flex overflow-hidden">
      <Notifications />

      {/* Mobile sidebar overlay */}
      <AnimatePresence>
        {sidebarOpen && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 z-30 lg:hidden"
            onClick={() => setSidebarOpen(false)}
          />
        )}
      </AnimatePresence>

      {/* Sidebar */}
      <aside className={`
        fixed lg:relative z-40 h-full
        w-56 min-w-[224px]
        border-r border-border bg-surface flex flex-col
        transition-transform duration-300 ease-in-out
        ${sidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0 lg:w-16 lg:min-w-[64px]'}
      `}>
        {/* Logo area */}
        <div className="px-4 py-5 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 flex items-center justify-center rounded-xl bg-gradient - to-br from-brand-400 to-brand-600 shadow-lg shadow-brand-500/20 shrink-0">
              <Clock className="text-white" size={20} />
            </div>
            <div className={`${!sidebarOpen ? 'lg:hidden' : ''}`}>
              <h1 className="text-lg font-bold tracking-tight leading-none">TENET</h1>
              <p className="text-[10px] text - text-muted leading-none mt-0.5">Time-Travel FS</p>
            </div>
          </div>
          <button
            onClick={() => setSidebarOpen(false)}
            className="lg:hidden text - text-muted hover:text - text-main p-1"
          >
            <X size={18} />
          </button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 px-3 py-2 space-y-1">
          <NavItem
            icon={<HardDrive size={18} />}
            label="Dashboard"
            active={activeTab === 'dashboard'}
            onClick={() => handleTabChange('dashboard')}
          />
          <NavItem
            icon={<FolderTree size={18} />}
            label="Explorer"
            active={activeTab === 'explorer'}
            onClick={() => handleTabChange('explorer')}
            disabled={!currentPath}
            tooltip={!currentPath ? 'Watch a directory first' : undefined}
          />
          <NavItem
            icon={<History size={18} />}
            label="History"
            active={activeTab === 'history'}
            onClick={() => handleTabChange('history')}
            disabled={!currentPath}
            tooltip={!currentPath ? 'Watch a directory first' : undefined}
          />
          <NavItem
            icon={<Settings size={18} />}
            label="Settings"
            active={activeTab === 'settings'}
            onClick={() => handleTabChange('settings')}
            disabled={!currentPath}
            tooltip={!currentPath ? 'Watch a directory first' : undefined}
          />
        </nav>

        {/* Sidebar footer with watched dir info */}
        {currentPath && (
          <div className="px-4 py-3 border-t border-border">
            <div className="flex items-center gap-2 text-xs text - text-muted">
              <Eye size={12} className="text-emerald-400 shrink-0" />
              <span className="truncate" title={currentPath}>
                {currentPath.split(/[/\\]/).pop()}
              </span>
            </div>
          </div>
        )}
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col h-screen overflow-y-auto relative min-w-0">
        {/* Top bar */}
        <div className="sticky top-0 z-20 bg-background/80 backdrop-blur-lg border-b border-border/50 px-4 sm:px-6 py-3 flex items-center gap-3">
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="lg:hidden p-2 rounded-lg hover:bg-surface-hover text - text-muted hover:text - text-main transition-colors"
          >
            <Menu size={20} />
          </button>
          <div className="flex items-center gap-2 text-sm text - text-muted">
            <span className="font-medium text - text-main capitalize">{activeTab}</span>
            {currentPath && activeTab === 'dashboard' && (
              <>
                <span className="text-border">•</span>
                <span className="truncate max-w-[200px] sm:max-w-none" title={currentPath}>{currentPath}</span>
              </>
            )}
          </div>
        </div>

        {/* Header decoration */}
        <div className="absolute top-0 left-0 w-full h-64 bg-brand-500/5 blur-[120px] pointer-events-none -z-10" />

        {activeTab === 'dashboard' && (
          <div className="flex-1 flex flex-col">
            <header className="px-4 sm:px-6 lg:px-8 py-6">
              <h2 className="text-2xl sm:text-3xl font-semibold mb-1">Welcome to Tenet</h2>
              <p className="text - text-muted text-sm sm:text-base">Select a directory to begin tracking file versions.</p>
            </header>

            <div className="px-4 sm:px-6 lg:px-8 pb-8 flex-1 flex flex-col gap-6">
              {/* Watcher Input Card */}
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className="glass-panel rounded-2xl p-4 sm:p-6 shadow-xl w-full max-w-3xl"
              >
                <h3 className="text-base sm:text-lg mb-4 font-medium flex items-center gap-2">
                  <FolderInput size={18} /> Watch Directory
                </h3>
                <div className="flex flex-col sm:flex-row gap-3">
                  <div className="flex-1 flex gap-2">
                    <input
                      className="flex-1 min-w-0 bg-background border border-border rounded-xl px-3 sm:px-4 py-2.5 text-sm outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/30 transition-all placeholder:text - text-muted/50"
                      placeholder="C:\Users\Project or Browse →"
                      value={inputPath}
                      onChange={(e) => setInputPath(e.target.value)}
                      onKeyDown={(e) => e.key === 'Enter' && handleWatch()}
                    />
                    <button
                      onClick={handleBrowse}
                      className="bg-surface hover:bg-surface-hover text - text-main border border-border px-3 sm:px-4 py-2.5 rounded-xl text-sm font-medium transition-colors whitespace-nowrap"
                    >
                      Browse
                    </button>
                  </div>
                  <button
                    onClick={handleWatch}
                    disabled={!inputPath || loading}
                    className="bg-brand-600 hover:bg-brand-500 disabled:bg-brand-600/50 disabled:cursor-not-allowed text-white px-6 py-2.5 rounded-xl text-sm font-medium transition-all shadow-lg shadow-brand-500/20 active:scale-[0.98] whitespace-nowrap"
                  >
                    {loading ? (
                      <span className="flex items-center gap-2">
                        <span className="h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                        Starting...
                      </span>
                    ) : (
                      <span className="flex items-center gap-2">
                        <Eye size={16} /> Watch
                      </span>
                    )}
                  </button>
                </div>
              </motion.div>

              {/* Status Dashboard */}
              <AnimatePresence>
                {status && (
                  <motion.div
                    initial={{ opacity: 0, scale: 0.95 }}
                    animate={{ opacity: 1, scale: 1 }}
                    className="grid grid-cols-1 sm:grid-cols-3 gap-4 w-full max-w-3xl"
                  >
                    <StatCard
                      icon={<FileText className="text-blue-400" size={22} />}
                      title="Tracked Files"
                      value={status.file_count.toString()}
                    />
                    <StatCard
                      icon={<Database className="text-brand-400" size={22} />}
                      title="Total Versions"
                      value={status.version_count.toString()}
                    />
                    <StatCard
                      icon={<HardDrive className="text-emerald-400" size={22} />}
                      title="Storage Used"
                      value={formatBytes(status.storage_size)}
                    />
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Quick actions when watching */}
              {currentPath && (
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.2 }}
                  className="w-full max-w-3xl"
                >
                  <h3 className="text-sm font-medium text - text-muted mb-3">Quick Actions</h3>
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                    <QuickAction
                      icon={<FolderTree size={18} />}
                      label="Explore Files"
                      description="Browse tracked files"
                      onClick={() => setActiveTab('explorer')}
                    />
                    <QuickAction
                      icon={<History size={18} />}
                      label="View History"
                      description="File version timeline"
                      onClick={() => setActiveTab('history')}
                    />
                    <QuickAction
                      icon={<Settings size={18} />}
                      label="Configure"
                      description="Ignore rules & more"
                      onClick={() => setActiveTab('settings')}
                    />
                  </div>
                </motion.div>
              )}

              {/* Empty state when not watching */}
              {!currentPath && !loading && (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ delay: 0.3 }}
                  className="flex-1 flex items-center justify-center max-w-3xl"
                >
                  <div className="text-center py-12">
                    <div className="grid grid-cols-3 gap-4 mb-8 max-w-sm mx-auto">
                      <FeatureChip icon={<Zap size={16} />} label="Auto-Tracking" />
                      <FeatureChip icon={<Clock size={16} />} label="Time Travel" />
                      <FeatureChip icon={<Shield size={16} />} label="Safe Restore" />
                    </div>
                    <p className="text - text-muted text-sm">Browse for a directory above to get started</p>
                  </div>
                </motion.div>
              )}
            </div>
          </div>
        )}

        {activeTab === 'explorer' && <FileExplorer onSelectFile={navToFileHistory} />}
        {activeTab === 'history' && <HistoryPage />}
        {activeTab === 'settings' && <SettingsPanel />}
      </main>
    </div>
  );
}

function NavItem({ icon, label, active = false, onClick, disabled = false, tooltip }: {
  icon: React.ReactNode, label: string, active?: boolean, onClick?: () => void, disabled?: boolean, tooltip?: string
}) {
  return (
    <button
      onClick={disabled ? undefined : onClick}
      title={tooltip}
      className={`
        w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm transition-all
        ${active
          ? 'bg-brand-500/15 text-brand-400 font-medium shadow-sm shadow-brand-500/5'
          : disabled
            ? 'text - text-muted/40 cursor-not-allowed'
            : 'text - text-muted hover:bg-surface-hover hover:text - text-main'}
      `}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

function StatCard({ icon, title, value }: { icon: React.ReactNode, title: string, value: string }) {
  return (
    <div className="glass-panel p-4 sm:p-5 rounded-2xl flex items-center gap-4 hover:border-brand-500/20 transition-colors">
      <div className="h-10 w-10 sm:h-11 sm:w-11 rounded-xl bg-surface-hover flex items-center justify-center shrink-0">
        {icon}
      </div>
      <div className="min-w-0">
        <h4 className="text - text-muted text-xs font-medium mb-0.5">{title}</h4>
        <p className="text-xl sm:text-2xl font-semibold text - text-main truncate">{value}</p>
      </div>
    </div>
  );
}

function QuickAction({ icon, label, description, onClick }: { icon: React.ReactNode, label: string, description: string, onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="glass-panel p-4 rounded-xl text-left hover:border-brand-500/30 transition-all group hover:bg-surface-hover"
    >
      <div className="flex items-center gap-2 mb-1">
        <span className="text-brand-400 group-hover:text-brand-300 transition-colors">{icon}</span>
        <span className="text-sm font-medium text - text-main">{label}</span>
      </div>
      <p className="text-xs text - text-muted">{description}</p>
    </button>
  );
}

function FeatureChip({ icon, label }: { icon: React.ReactNode, label: string }) {
  return (
    <div className="flex flex-col items-center gap-2 p-3 rounded-xl bg-surface/60 border border-border">
      <span className="text-brand-400">{icon}</span>
      <span className="text-xs text - text-muted font-medium">{label}</span>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

export default App;
