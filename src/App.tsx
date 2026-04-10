import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { motion, AnimatePresence } from 'framer-motion';
import { Clock, FolderInput, HardDrive, History, FileText, Database, Settings, FolderTree } from 'lucide-react';
import { useAppStore, type FileEntry } from './store/useAppStore';
import HistoryPage from './components/Timeline';
import FileExplorer from './components/FileExplorer';
import SettingsPanel from './components/SettingsPanel';

function App() {
  const { currentPath, setCurrentPath, status, setStatus, setFileHistory } = useAppStore();
  const [inputPath, setInputPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');
  const [activeTab, setActiveTab] = useState<'dashboard' | 'explorer' | 'history' | 'settings'>('dashboard');

  const navToFileHistory = async (path: string) => {
    try {
      const fullPath = currentPath + '/' + path;
      const history = await invoke('get_history', { file: fullPath });
      setFileHistory(history as FileEntry);
      setActiveTab('history');
    } catch(e) {
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
    setErrorMsg('');
    try {
      await invoke('watch_directory', { path: inputPath });
      setCurrentPath(inputPath);
      await refreshStatus(inputPath);
    } catch (e: any) {
      setErrorMsg(e.toString());
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

  return (
    <div className="min-h-screen bg-background text-text-main flex overflow-hidden">
      {/* Sidebar */}
      <aside className="w-64 border-r border-border bg-surface flex flex-col pt-8">
        <div className="px-6 mb-10 flex items-center gap-3">
          <div className="h-10 w-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-brand-400 to-brand-600 shadow-lg shadow-brand-500/20">
            <Clock className="text-white" size={24} />
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight">TENET</h1>
            <p className="text-xs text-text-muted">Time-Travel FS</p>
          </div>
        </div>

        <nav className="flex-1 px-4 space-y-2">
          <NavItem 
            icon={<HardDrive size={20} />} 
            label="Dashboard" 
            active={activeTab === 'dashboard'} 
            onClick={() => setActiveTab('dashboard')} 
          />
          <NavItem 
            icon={<FolderTree size={20} />} 
            label="Explorer" 
            active={activeTab === 'explorer'} 
            onClick={() => setActiveTab('explorer')} 
          />
          <NavItem 
            icon={<History size={20} />} 
            label="History" 
            active={activeTab === 'history'} 
            onClick={() => setActiveTab('history')} 
          />
          <NavItem 
            icon={<Settings size={20} />} 
            label="Settings" 
            active={activeTab === 'settings'} 
            onClick={() => setActiveTab('settings')} 
          />
        </nav>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col h-screen overflow-y-auto relative">
        {/* Header decoration */}
        <div className="absolute top-0 left-0 w-full h-64 bg-brand-500/5 blur-[120px] pointer-events-none -z-10" />
        
        {activeTab === 'dashboard' && (
          <>
            <header className="px-10 py-8">
              <h2 className="text-3xl font-semibold mb-2">Welcome to Tenet</h2>
              <p className="text-text-muted">Select a directory to begin tracking file versions.</p>
            </header>

            <div className="px-10 pb-10 flex-1 flex flex-col gap-8 flex-shrink-0">
              {/* Watcher Input Card */}
              <motion.div 
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className="glass-panel rounded-2xl p-6 shadow-xl w-full max-w-3xl"
              >
                <h3 className="text-lg mb-4 font-medium flex items-center gap-2">
                  <FolderInput size={20} /> Watch Directory
                </h3>
                <div className="flex gap-4">
                  <input
                    className="flex-1 bg-background border border-border rounded-xl px-4 py-3 outline-none focus:border-brand-500 transition-colors"
                    placeholder="C:\Users\John\Documents\Project"
                    value={inputPath}
                    onChange={(e) => setInputPath(e.target.value)}
                  />
                  <button 
                    onClick={handleBrowse}
                    className="bg-surface hover:bg-surface-hover text-text-main border border-border px-6 py-3 rounded-xl font-medium transition-colors"
                  >
                    Browse...
                  </button>
                  <button 
                    onClick={handleWatch}
                    className="bg-brand-600 hover:bg-brand-500 text-white px-8 py-3 rounded-xl font-medium transition-colors shadow-lg shadow-brand-500/20 active:scale-95"
                  >
                    {loading ? 'Starting...' : 'Watch'}
                  </button>
                </div>
                {errorMsg && <p className="text-red-400 mt-4 text-sm font-medium">{errorMsg}</p>}
              </motion.div>

              {/* Status Dashboard */}
              <AnimatePresence>
                {status && (
                  <motion.div 
                    initial={{ opacity: 0, scale: 0.95 }}
                    animate={{ opacity: 1, scale: 1 }}
                    className="grid grid-cols-1 md:grid-cols-3 gap-6 w-full max-w-4xl"
                  >
                    <StatCard 
                      icon={<FileText className="text-blue-400" size={24} />}
                      title="Tracked Files"
                      value={status.file_count.toString()}
                    />
                    <StatCard 
                      icon={<Database className="text-brand-400" size={24} />}
                      title="Total Versions"
                      value={status.version_count.toString()}
                    />
                    <StatCard 
                      icon={<HardDrive className="text-emerald-400" size={24} />}
                      title="Storage Used"
                      value={(status.storage_size / 1024).toFixed(2) + ' KB'}
                    />
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </>
        )}

        {activeTab === 'explorer' && <FileExplorer onSelectFile={navToFileHistory} />}
        {activeTab === 'history' && <HistoryPage />}
        {activeTab === 'settings' && <SettingsPanel />}
      </main>
    </div>
  );
}

function NavItem({ icon, label, active = false, onClick }: { icon: React.ReactNode, label: string, active?: boolean, onClick?: () => void }) {
  return (
    <button 
      onClick={onClick}
      className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all ${active ? 'bg-brand-500/10 text-brand-400 font-medium' : 'text-text-muted hover:bg-surface-hover hover:text-text-main'}`}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

function StatCard({ icon, title, value }: { icon: React.ReactNode, title: string, value: string }) {
  return (
    <div className="glass-panel p-6 rounded-2xl flex flex-col gap-4">
      <div className="h-12 w-12 rounded-xl bg-surface-hover flex items-center justify-center">
        {icon}
      </div>
      <div>
        <h4 className="text-text-muted text-sm font-medium mb-1">{title}</h4>
        <p className="text-3xl font-semibold text-text-main">{value}</p>
      </div>
    </div>
  );
}

export default App;
