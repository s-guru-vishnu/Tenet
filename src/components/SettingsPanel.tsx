import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Save, AlertCircle } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

export default function SettingsPanel() {
  const { currentPath } = useAppStore();
  const [ignoreRules, setIgnoreRules] = useState('');
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [success, setSuccess] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');

  useEffect(() => {
    if (!currentPath) return;
    setLoading(true);
    invoke('get_ignore_rules', { path: currentPath })
      .then((res) => {
        setIgnoreRules(res as string);
        setErrorMsg('');
      })
      .catch((e: any) => setErrorMsg(e.toString()))
      .finally(() => setLoading(false));
  }, [currentPath]);

  const handleSave = async () => {
    if (!currentPath) return;
    setSaving(true);
    setSuccess(false);
    setErrorMsg('');
    try {
      await invoke('save_ignore_rules', { path: currentPath, rules: ignoreRules });
      setSuccess(true);
      setTimeout(() => setSuccess(false), 3000);
    } catch (e: any) {
      setErrorMsg(e.toString());
    } finally {
      setSaving(false);
    }
  };

  if (!currentPath) {
    return (
      <div className="flex-1 flex flex-col pt-8">
        <div className="px-10 py-8">
            <h2 className="text-3xl font-semibold mb-2">Settings</h2>
            <p className="text-text-muted">Select a directory in the Dashboard to edit its settings.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col pt-8">
      <div className="px-10 mb-8">
        <h2 className="text-3xl font-semibold mb-2">Settings</h2>
        <p className="text-text-muted">Manage configuration for {currentPath}</p>
      </div>

      <div className="px-10 pb-10 flex-1 overflow-auto">
        <motion.div 
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="glass-panel p-8 rounded-2xl max-w-3xl"
        >
          <div className="flex items-center gap-3 mb-4">
            <h3 className="text-xl font-medium">Ignore Rules (.tenetignore)</h3>
          </div>
          <p className="text-text-muted text-sm mb-6">
            Specify files and directories that TENET should completely ignore. Uses standard glob patterns (e.g. <code>*.log</code>, <code>node_modules/</code>).
          </p>

          {loading ? (
            <div className="animate-pulse flex space-x-4 bg-surface h-32 rounded-xl"></div>
          ) : (
            <div className="flex flex-col gap-4">
              <textarea
                value={ignoreRules}
                onChange={(e) => setIgnoreRules(e.target.value)}
                className="w-full bg-surface border border-border rounded-xl p-4 min-h-[200px] outline-none focus:border-brand-500 font-mono text-sm transition-colors"
                placeholder="# Add paths to ignore..."
              />
              
              <div className="flex justify-between items-center">
                <div className="flex flex-col">
                    {success && <span className="text-emerald-400 text-sm font-medium">Saved successfully!</span>}
                    {errorMsg && <span className="text-red-400 text-sm font-medium flex items-center gap-1"><AlertCircle size={14}/> {errorMsg}</span>}
                </div>
                <button
                  onClick={handleSave}
                  disabled={saving}
                  className="bg-brand-600 hover:bg-brand-500 text-white px-6 py-3 rounded-xl font-medium transition-colors flex items-center gap-2 shadow-lg shadow-brand-500/20 disabled:opacity-50"
                >
                  <Save size={18} />
                  {saving ? 'Saving...' : 'Save Settings'}
                </button>
              </div>
            </div>
          )}
        </motion.div>
      </div>
    </div>
  );
}
