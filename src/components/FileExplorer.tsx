import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Folder, FolderOpen, FileText, ChevronRight, ChevronDown } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

type TreeNode = {
  name: string;
  path: string;
  type: 'file' | 'folder';
  children?: TreeNode[];
};

function buildTree(paths: string[]): TreeNode[] {
  const root: TreeNode = { name: 'root', path: '', type: 'folder', children: [] };
  
  for (const path of paths) {
    const parts = path.split('/');
    let current = root;
    let currentPath = '';

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      currentPath += (currentPath ? '/' : '') + part;
      
      let child = current.children!.find((c) => c.name === part);
      if (!child) {
        child = {
          name: part,
          path: currentPath,
          type: i === parts.length - 1 ? 'file' : 'folder',
          children: i === parts.length - 1 ? undefined : [],
        };
        current.children!.push(child);
      }
      current = child;
    }
  }

  // Sort: folders first, then files
  const sortTree = (node: TreeNode) => {
    if (node.children) {
      node.children.sort((a, b) => {
        if (a.type === b.type) return a.name.localeCompare(b.name);
        return a.type === 'folder' ? -1 : 1;
      });
      node.children.forEach(sortTree);
    }
  };
  
  sortTree(root);
  return root.children || [];
}

const TreeItem = ({ node, onSelectFile, level = 0 }: { node: TreeNode, onSelectFile: (path: string) => void, level?: number }) => {
  const [isOpen, setIsOpen] = useState(false);

  const isFolder = node.type === 'folder';

  const handleClick = () => {
    if (isFolder) {
      setIsOpen(!isOpen);
    } else {
      onSelectFile(node.path);
    }
  };

  return (
    <div className="select-none">
      <div 
        onClick={handleClick}
        className={`flex items-center gap-2 py-1.5 px-3 rounded-lg cursor-pointer transition-colors hover:bg-surface-hover ${!isFolder ? 'text-text-main' : 'text-text-muted hover:text-text-main'}`}
        style={{ paddingLeft: `${level * 16 + 12}px` }}
      >
        {isFolder ? (
          <span className="flex items-center gap-1.5 text-brand-400">
            {isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            {isOpen ? <FolderOpen size={16} /> : <Folder size={16} />}
          </span>
        ) : (
          <span className="flex items-center gap-1.5 ml-5 text-gray-400">
            <FileText size={16} />
          </span>
        )}
        <span className="text-sm font-medium">{node.name}</span>
      </div>
      
      {isFolder && isOpen && node.children && (
        <motion.div initial={false}
          animate={{ height: 'auto', opacity: 1 }}
          className="overflow-hidden"
        >
          {node.children.map((child) => (
            <TreeItem key={child.path} node={child} onSelectFile={onSelectFile} level={level + 1} />
          ))}
        </motion.div>
      )}
    </div>
  );
};

export default function FileExplorer({ onSelectFile }: { onSelectFile: (path: string) => void }) {
  const { currentPath } = useAppStore();
  const [files, setFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  const loadFiles = async () => {
    if (!currentPath) return;
    setLoading(true);
    try {
      const paths = await invoke<string[]>('get_tracked_files', { path: currentPath });
      setFiles(paths);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadFiles();
  }, [currentPath]);

  if (!currentPath) {
    return (
      <div className="flex-1 flex items-center justify-center text-text-muted">
        Select a directory from the Dashboard to explore files.
      </div>
    );
  }

  const tree = buildTree(files);

  return (
    <div className="flex-1 flex flex-col pt-8">
      <div className="px-10 mb-8 flex justify-between items-center">
        <div>
          <h2 className="text-3xl font-semibold mb-2">Explorer</h2>
          <p className="text-text-muted">Browse all files tracked by TENET.</p>
        </div>
        <button onClick={loadFiles} className="text-brand-400 hover:text-brand-300 transition-colors bg-brand-500/10 px-4 py-2 rounded-xl text-sm font-medium">Reloader / Refresh</button>
      </div>

      <div className="flex-1 px-10 pb-10 overflow-auto">
        <div className="glass-panel p-6 rounded-2xl max-w-2xl border-border">
          {loading ? (
             <p className="text-text-muted text-sm p-4">Loading tree...</p>
          ) : tree.length === 0 ? (
            <p className="text-text-muted text-sm p-4">No tracked files found.</p>
          ) : (
            <div className="space-y-1">
              {tree.map(node => (
                <TreeItem key={node.path} node={node} onSelectFile={onSelectFile} />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
