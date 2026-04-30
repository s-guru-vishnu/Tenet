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
        <h4 className="text-sm font-medium text - text-muted">Comparing version to current file</h4>
        <button onClick={onClose} className="text-xs text-brand-400 hover:text-brand-300">Close Diff</button>
      </div>
      {loading ? (
        <div className="animate-pulse bg-surface h-32 rounded-xl"></div>
      ) : error ? (
        error.includes('not valid UTF-8') ? (
          <div className="flex flex-col items-center justify-center py-8 text-text-muted bg-surface/30 rounded-xl border border-border">
             <SplitSquareHorizontal size={24} className="mb-2 opacity-50" />
             <p className="text-sm">Binary file</p>
             <p className="text-xs opacity-60">Diffing is not supported for binary formats.</p>
          </div>
        ) : (
          <p className="text-red-400 text-sm bg-red-400/10 p-3 rounded-lg border border-red-400/20">Failed to load content: {error}</p>
        )
      ) : (
        <DiffViewer oldStr={oldStr} newStr={newStr} />
      )}
    </div>
  );
}

export default function HistoryPage() {
  const { currentPath, fileHistory, setFileHistory, addNotification } = useAppStore();
  const [searchPath, setSearchPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [viewingDiff, setViewingDiff] = useState<string | null>(null);
  const [restoringHash, setRestoringHash] = useState<string | null>(null);

  const fetchHistory = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchPath) return;

    setLoading(true);
    setViewingDiff(null);
    try {
      // Find full path or use relative
      const fullPath = searchPath.includes(':') || searchPath.startsWith('/')
        ? searchPath
        : currentPath + '/' + searchPath;

      const history = await invoke('get_history', { file: fullPath });
      setFileHistory(history as FileEntry);
    } catch (e: any) {
      addNotification('error', `Failed to load history: ${e.toString()}`);
      setFileHistory(null);
    } finally {
      setLoading(false);
    }
  };

  const handleRestore = async (timestamp: string, hash: string) => {
    setRestoring(true);
    setRestoringHash(hash);

    try {
      const result = await invoke('restore_version', {
        file: currentPath + '/' + fileHistory?.path,
        timestamp
      });
      addNotification('success', result as string);

      // Refresh history
      const history = await invoke('get_history', { file: currentPath + '/' + fileHistory?.path });
      setFileHistory(history as FileEntry);
    } catch (e: any) {
      addNotification('error', `Restoration failed: ${e.toString()}`);
    } finally {
      setRestoring(false);
      setRestoringHash(null);
    }
  };

  if (!currentPath) {
    return (
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-center">
          <Search size={40} className="text - text-muted/30 mx-auto mb-3" />
          <p className="text - text-muted text-sm">Watch a directory from the Dashboard to view file history.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="px-4 sm:px-6 lg:px-8 py-5">
        <h2 className="text-2xl sm:text-3xl font-semibold mb-1">Version History</h2>
        <p className="text - text-muted text-sm">Search for a tracked file to view its timeline.</p>
      </div>

      <div className="px-4 sm:px-6 lg:px-8 mb-6">
        <form onSubmit={fetchHistory} className="flex flex-col sm:flex-row gap-3 max-w-3xl">
          <div className="flex-1 relative">
            <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 text - text-muted" size={18} />
            <input
              value={searchPath}
              onChange={(e) => setSearchPath(e.target.value)}
              placeholder="Enter file path (e.g. src/index.css)"
              className="w-full bg-surface border border-border rounded-xl pl-10 pr-4 py-2.5 text-sm outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/30 transition-all placeholder:text - text-muted/50"
            />
          </div>
          <button
            type="submit"
            disabled={loading || !searchPath}
            className="bg-brand-600 hover:bg-brand-500 text-white px-5 py-2.5 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
          >
            {loading ? (
              <span className="flex items-center gap-2 justify-center">
                <span className="h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                Searching...
              </span>
            ) : 'View Timeline'}
          </button>
        </form>
      </div>

      {/* Results area */}
      <div className="flex-1 overflow-y-auto px-4 sm:px-6 lg:px-8 pb-8">
        {fileHistory ? (
          <div className="glass-panel p-4 sm:p-6 lg:p-8 rounded-2xl max-w-4xl border-border">
            <div className="flex flex-col sm:flex-row sm:items-center gap-3 sm:gap-4 mb-6 pb-5 border-b border-border">
              <div className="p-2.5 bg-brand-500/10 rounded-xl w-fit">
                <FileIcon className="text-brand-400" size={24} />
              </div>
              <div className="min-w-0">
                <h3 className="text-lg font-medium truncate">{fileHistory.path}</h3>
                <p className="text - text-muted text-sm">{fileHistory.versions.length} version{fileHistory.versions.length !== 1 ? 's' : ''} tracked</p>
              </div>
            </div>

            <div className="relative pl-4 sm:pl-6 border-l-2 border-transparent space-y-5 sm:space-y-6">
              <div className="timeline-line"></div>
              {[...fileHistory.versions].reverse().map((version, idx) => {
                const versionNum = fileHistory.versions.length - idx;
                const isLatest = idx === 0;
                const isRestoringThis = restoring && restoringHash === version.hash;

                return (
                  <motion.div
                    initial={{ opacity: 0, x: -10 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: idx * 0.03 }}
                    key={version.timestamp}
                    className="relative"
                  >
                    <div className={`absolute -left-[25px] sm:-left-[33px] top-1.5 h-3.5 w-3.5 rounded-full border-[3px] border-[#0f1115] ${isLatest ? 'bg-emerald-400' : 'bg-brand-500'}`} />

                    <div className="glass-panel p-3 sm:p-4 rounded-xl hover:border-brand-500/30 transition-colors group">
                      <div className="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-2 sm:gap-4">
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-x-2 gap-y-1 mb-2">
                            <span className={`text-sm font-mono font-semibold ${isLatest ? 'text-emerald-400' : 'text-brand-400'}`}>
                              v{versionNum}
                            </span>
                            {isLatest && (
                              <span className="text-[10px] font-medium bg-emerald-500/15 text-emerald-400 px-1.5 py-0.5 rounded-md">
                                LATEST
                              </span>
                            )}
                            <span className="text - text-muted text-xs">•</span>
                            <span className="text - text-main text-xs sm:text-sm flex items-center gap-1.5">
                              <Clock size={12} className="text - text-muted shrink-0" />
                              {new Date(version.timestamp).toLocaleString()}
                            </span>
                          </div>
                          <div className="flex flex-wrap gap-2 text-xs font-mono text - text-muted">
                            <span className="bg-surface py-0.5 px-2 rounded-md">
                              {version.hash ? version.hash.substring(0, 8) : 'DELETED'}
                            </span>
                            <span className="bg-surface py-0.5 px-2 rounded-md">
                              {formatSize(version.size)}
                            </span>
                            <span className={`py-0.5 px-2 rounded-md ${version.version_type === 'Snapshot' ? 'text-emerald-400 bg-emerald-400/10' : 'text-red-400 bg-red-400/10'}`}>
                              {version.version_type}
                            </span>
                          </div>
                        </div>

                        {version.version_type !== 'Deletion' && (
                          <div className="flex gap-2 sm:opacity-0 sm:group-hover:opacity-100 transition-opacity shrink-0">
                            <button
                              onClick={() => setViewingDiff(viewingDiff === version.hash ? null : version.hash)}
                              className="bg-blue-500/10 hover:bg-blue-500/20 text-blue-400 px-2.5 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors"
                            >
                              <SplitSquareHorizontal size={14} /> Diff
                            </button>
                            <button
                              onClick={() => handleRestore(version.timestamp, version.hash)}
                              disabled={restoring}
                              className="bg-brand-500/10 hover:bg-brand-500/20 text-brand-400 px-2.5 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors disabled:opacity-50"
                            >
                              {isRestoringThis ? (
                                <span className="h-3.5 w-3.5 border-2 border-brand-400/30 border-t-brand-400 rounded-full animate-spin" />
                              ) : (
                                <RefreshCcw size={14} />
                              )}
                              Restore
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
                );
              })}
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-center py-16">
            <div className="text-center">
              <Clock size={40} className="text - text-muted/20 mx-auto mb-3" />
              <p className="text - text-muted text-sm">Search for a file to see its version history</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}
