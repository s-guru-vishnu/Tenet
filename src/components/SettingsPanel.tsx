import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Save, Settings } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

export default function SettingsPanel() {
  const { currentPath, addNotification } = useAppStore();
  const [ignoreRules, setIgnoreRules] = useState('');
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!currentPath) return;
    setLoading(true);
    invoke('get_ignore_rules', { path: currentPath })
      .then((res) => {
        setIgnoreRules(res as string);
      })
      .catch((e: any) => addNotification('error', `Failed to load settings: ${e.toString()}`))
      .finally(() => setLoading(false));
  }, [currentPath]);

  const handleSave = async () => {
    if (!currentPath) return;
    setSaving(true);
    try {
      await invoke('save_ignore_rules', { path: currentPath, rules: ignoreRules });
      addNotification('success', 'Settings saved successfully!');
    } catch (e: any) {
      addNotification('error', `Failed to save settings: ${e.toString()}`);
    } finally {
      setSaving(false);
    }
  };

  if (!currentPath) {
    return (
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-center">
          <Settings size={40} className="text - text-muted/30 mx-auto mb-3" />
          <p className="text - text-muted text-sm">Watch a directory from the Dashboard to configure settings.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="px-4 sm:px-6 lg:px-8 py-5">
        <h2 className="text-2xl sm:text-3xl font-semibold mb-1">Settings</h2>
        <p className="text - text-muted text-sm truncate">
          Configuration for <span className="text - text-main font-medium">{currentPath.split(/[/\\]/).pop()}</span>
        </p>
      </div>

      <div className="flex-1 overflow-y-auto px-4 sm:px-6 lg:px-8 pb-8">
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="glass-panel p-4 sm:p-6 lg:p-8 rounded-2xl max-w-3xl"
        >
          <div className="flex items-center gap-3 mb-3">
            <h3 className="text-lg font-medium">Ignore Rules</h3>
            <span className="text-xs text - text-muted bg-surface px-2 py-0.5 rounded-md font-mono">.tenetignore</span>
          </div>
          <p className="text - text-muted text-sm mb-5">
            Specify files and directories that TENET should completely ignore. Uses standard glob patterns (e.g. <code className="bg-surface px-1.5 py-0.5 rounded text-xs">*.log</code>, <code className="bg-surface px-1.5 py-0.5 rounded text-xs">node_modules/</code>).
          </p>

          {loading ? (
            <div className="flex items-center gap-3 p-6 text - text-muted text-sm">
              <span className="h-4 w-4 border-2 border-brand-400/30 border-t-brand-400 rounded-full animate-spin" />
              Loading configuration...
            </div>
          ) : (
            <div className="flex flex-col gap-4">
              <textarea
                value={ignoreRules}
                onChange={(e) => setIgnoreRules(e.target.value)}
                className="w-full bg-surface border border-border rounded-xl p-3 sm:p-4 min-h-[180px] sm:min-h-[220px] outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/30 font-mono text-sm transition-all resize-y placeholder:text - text-muted/50"
                placeholder="# Add paths to ignore...
node_modules/
*.log
dist/
.env"
              />

              <div className="flex justify-end">
                <button
                  onClick={handleSave}
                  disabled={saving}
                  className="bg-brand-600 hover:bg-brand-500 text-white px-5 py-2.5 rounded-xl text-sm font-medium transition-all flex items-center gap-2 shadow-lg shadow-brand-500/20 disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98]"
                >
                  {saving ? (
                    <>
                      <span className="h-4 w-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                      Saving...
                    </>
                  ) : (
                    <>
                      <Save size={16} />
                      Save Settings
                    </>
                  )}
                </button>
              </div>
            </div>
          )}
        </motion.div>
      </div>
    </div>
  );
}
