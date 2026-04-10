import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { useAppStore, type FileEntry } from '../store/useAppStore';
import { Search, File as FileIcon, Clock, RefreshCcw, SplitSquareHorizontal } from 'lucide-react';
import DiffViewer from './DiffViewer';

function DiffContainer({ filePath, hash, onClose }: { filePath: string, hash: string, onClose: () => void }) {
  const { currentPath } = useAppStore();
  const [oldStr, setOldStr] = useState<string>('');
  const [newStr, setNewStr] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    const fetchDiffs = async () => {
      setLoading(true);
      setError('');
      try {
        const fullPath = currentPath + '/' + filePath;
        const oldContent = await invoke('get_file_content', { path: fullPath, hash: hash });
        const newContent = await invoke('get_file_content', { path: fullPath, hash: null });
        if (active) {
          setOldStr(oldContent as string);
          setNewStr(newContent as string);
        }
      } catch (e: any) {
        if (active) setError(e.toString());
      } finally {
        if (active) setLoading(false);
      }
    };
    fetchDiffs();
    return () => { active = false; };
  }, [filePath, hash, currentPath]);

  return (
    <div className="mt-4 border-t border-border pt-4">
      <div className="flex justify-between items-center mb-2">
        <h4 className="text-sm font-medium text-text-muted">Comparing version to current file</h4>
        <button onClick={onClose} className="text-xs text-brand-400 hover:text-brand-300">Close Diff</button>
      </div>
      {loading ? (
        <div className="animate-pulse bg-surface h-32 rounded-xl"></div>
      ) : error ? (
        <p className="text-red-400 text-sm">Failed to load content: {error}</p>
      ) : (
        <DiffViewer oldStr={oldStr} newStr={newStr} />
      )}
    </div>
  );
}

export default function HistoryPage() {
  const { currentPath, fileHistory, setFileHistory } = useAppStore();
  const [searchPath, setSearchPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');
  const [restoring, setRestoring] = useState(false);
  const [successMsg, setSuccessMsg] = useState('');
  const [viewingDiff, setViewingDiff] = useState<string | null>(null);

  const fetchHistory = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchPath) return;
    
    setLoading(true);
    setErrorMsg('');
    setSuccessMsg('');
    setViewingDiff(null);
    try {
      // Find full path or use relative
      const fullPath = searchPath.includes(':') || searchPath.startsWith('/') 
        ? searchPath 
        : currentPath + '/' + searchPath;

      const history = await invoke('get_history', { file: fullPath });
      setFileHistory(history as FileEntry);
    } catch (e: any) {
      setErrorMsg(e.toString());
      setFileHistory(null);
    } finally {
      setLoading(false);
    }
  };

  const handleRestore = async (timestamp: string) => {
    if (!confirm('Are you sure you want to restore this version? Ongoing changes will be overwritten.')) return;
    
    setRestoring(true);
    setErrorMsg('');
    setSuccessMsg('');
    
    try {
      const result = await invoke('restore_version', { 
        file: currentPath + '/' + fileHistory?.path, 
        timestamp 
      });
      setSuccessMsg(result as string);
      
      // Refresh history
      const history = await invoke('get_history', { file: currentPath + '/' + fileHistory?.path });
      setFileHistory(history as FileEntry);
    } catch(e: any) {
      setErrorMsg(e.toString());
    } finally {
      setRestoring(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col pt-8">
      <div className="px-10 mb-8">
        <h2 className="text-3xl font-semibold mb-2">Version History</h2>
        <p className="text-text-muted">Search for a tracked file to view its timeline.</p>
      </div>

      <div className="px-10 mb-8">
        <form onSubmit={fetchHistory} className="flex gap-4 max-w-3xl">
          <div className="flex-1 relative">
            <Search className="absolute left-4 top-3 text-text-muted" size={20} />
            <input 
              value={searchPath}
              onChange={(e) => setSearchPath(e.target.value)}
              placeholder="Enter relative file path (e.g. src/index.css)"
              className="w-full bg-surface border border-border rounded-xl pl-12 pr-4 py-3 outline-none focus:border-brand-500 transition-colors"
            />
          </div>
          <button 
            type="submit"
            disabled={loading}
            className="bg-brand-600 hover:bg-brand-500 text-white px-6 py-3 rounded-xl font-medium transition-colors disabled:opacity-50"
          >
            {loading ? 'Searching...' : 'View Timeline'}
          </button>
        </form>
        {errorMsg && <p className="text-red-400 mt-4 text-sm">{errorMsg}</p>}
        {successMsg && <p className="text-emerald-400 mt-4 text-sm">{successMsg}</p>}
      </div>

      {fileHistory && (
        <div className="flex-1 px-10 pb-10 overflow-y-auto">
          <div className="glass-panel p-8 rounded-2xl max-w-4xl border-border">
            <div className="flex items-center gap-4 mb-8 pb-6 border-b border-border">
              <div className="p-3 bg-brand-500/10 rounded-xl">
                <FileIcon className="text-brand-400" size={28} />
              </div>
              <div>
                <h3 className="text-xl font-medium">{fileHistory.path}</h3>
                <p className="text-text-muted">{fileHistory.versions.length} versions tracked in TENET</p>
              </div>
            </div>

            <div className="relative pl-6 border-l-2 border-border space-y-8">
              {[...fileHistory.versions].reverse().map((version, idx) => (
                <motion.div 
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: idx * 0.05 }}
                  key={version.timestamp} 
                  className="relative"
                >
                  <div className="absolute -left-[35px] top-1 h-4 w-4 rounded-full border-4 border-[#0f1115] bg-brand-500" />
                  
                  <div className="glass-panel p-5 rounded-xl border border-border hover:border-brand-500/30 transition-colors group">
                    <div className="flex justify-between items-start">
                      <div>
                        <div className="flex items-center gap-2 mb-1">
                          <span className="text-brand-400 font-mono font-medium">v{fileHistory.versions.length - idx}</span>
                          <span className="text-text-muted">•</span>
                          <span className="text-text-main flex items-center gap-2">
                            <Clock size={14} className="text-text-muted"/> 
                            {new Date(version.timestamp).toLocaleString()}
                          </span>
                        </div>
                        <div className="flex gap-4 text-sm mt-3 font-mono text-text-muted">
                          <span className="bg-surface py-1 px-2 rounded-md">HASH: {version.hash ? version.hash.substring(0, 8) : 'DELETED'}</span>
                          <span className="bg-surface py-1 px-2 rounded-md">SIZE: {(version.size / 1024).toFixed(2)} KB</span>
                          <span className={`py-1 px-2 rounded-md ${version.version_type === 'Snapshot' ? 'text-emerald-400 bg-emerald-400/10' : 'text-red-400 bg-red-400/10'}`}>
                            {version.version_type}
                          </span>
                        </div>
                      </div>
                      
                      {version.version_type !== 'Deletion' && (
                        <div className="flex gap-2">
                            <button 
                              onClick={() => setViewingDiff(viewingDiff === version.hash ? null : version.hash)}
                              className="opacity-0 group-hover:opacity-100 transition-opacity bg-blue-500/10 hover:bg-blue-500/20 text-blue-400 px-3 py-2 rounded-lg text-sm font-medium flex items-center gap-2"
                            >
                              <SplitSquareHorizontal size={16} /> Diff
                            </button>
                            <button 
                              onClick={() => handleRestore(version.timestamp)}
                              disabled={restoring}
                              className="opacity-0 group-hover:opacity-100 transition-opacity bg-brand-500/10 hover:bg-brand-500/20 text-brand-400 px-3 py-2 rounded-lg text-sm font-medium flex items-center gap-2"
                            >
                              <RefreshCcw size={16} /> Restore
                            </button>
                        </div>
                      )}
                    </div>
                    {viewingDiff === version.hash && version.version_type !== 'Deletion' && (
                        <DiffContainer 
                          filePath={fileHistory.path} 
                          hash={version.hash} 
                          onClose={() => setViewingDiff(null)} 
                        />
                    )}
                  </div>
                </motion.div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
