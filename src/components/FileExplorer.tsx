import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { Folder, FolderOpen, FileText, ChevronRight, ChevronDown, RefreshCw, Search, FileJson, FileCode, FileImage, FileArchive } from 'lucide-react';
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

  // Sort: folders first, then files (natural numeric order)
  const sortTree = (node: TreeNode) => {
    if (node.children) {
      node.children.sort((a, b) => {
        if (a.type === b.type) return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
        return a.type === 'folder' ? -1 : 1;
      });
      node.children.forEach(sortTree);
    }
  };

  sortTree(root);
  return root.children || [];
}

function filterTree(nodes: TreeNode[], query: string): TreeNode[] {
  if (!query) return nodes;
  const lower = query.toLowerCase();

  return nodes.reduce<TreeNode[]>((acc, node) => {
    if (node.type === 'file') {
      if (node.name.toLowerCase().includes(lower) || node.path.toLowerCase().includes(lower)) {
        acc.push(node);
      }
    } else if (node.children) {
      const filteredChildren = filterTree(node.children, query);
      if (filteredChildren.length > 0) {
        acc.push({ ...node, children: filteredChildren });
      }
    }
    return acc;
  }, []);
}

const TreeItem = ({ node, onSelectFile, level = 0, defaultOpen = false }: { node: TreeNode, onSelectFile: (path: string) => void, level?: number, defaultOpen?: boolean }) => {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  const isFolder = node.type === 'folder';

  useEffect(() => {
    if (defaultOpen) setIsOpen(true);
  }, [defaultOpen]);

  const handleClick = () => {
    if (isFolder) {
      setIsOpen(!isOpen);
    } else {
      onSelectFile(node.path);
    }
  };

  const getFileIcon = (name: string) => {
    const ext = name.split('.').pop()?.toLowerCase();
    if (!ext || ext === name) return <FileText size={15} />;
    
    switch (ext) {
      case 'json':
      case 'lock':
        return <FileJson size={15} className="text-yellow-400" />;
      case 'ts':
      case 'tsx':
      case 'js':
      case 'jsx':
        return <FileCode size={15} className="text-blue-400" />;
      case 'rs':
        return <FileCode size={15} className="text-orange-400" />;
      case 'png':
      case 'jpg':
      case 'svg':
      case 'ico':
        return <FileImage size={15} className="text-purple-400" />;
      case 'zip':
      case 'tar':
      case 'gz':
        return <FileArchive size={15} className="text-red-400" />;
      case 'css':
        return <FileCode size={15} className="text-pink-400" />;
      case 'html':
        return <FileCode size={15} className="text-orange-500" />;
      default:
        return <FileText size={15} />;
    }
  };

  return (
    <div className="select-none">
      <div
        onClick={handleClick}
        className={`
          flex items-center gap-2 py-1.5 px-2 rounded-lg cursor-pointer transition-colors
          ${!isFolder
            ? 'text - text-main hover:bg-brand-500/10 hover:text-brand-300'
            : 'text - text-muted hover:bg-surface-hover hover:text - text-main'}
        `}
        style={{ paddingLeft: `${level * 14 + 8}px` }}
      >
        {isFolder ? (
          <span className="flex items-center gap-1.5 text-brand-400 shrink-0">
            {isOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            {isOpen ? <FolderOpen size={15} /> : <Folder size={15} />}
          </span>
        ) : (
          <span className="flex items-center gap-1.5 ml-5 shrink-0 opacity-80">
            {getFileIcon(node.name)}
          </span>
        )}
        <span className="text-sm truncate">{node.name}</span>
      </div>

      {isFolder && isOpen && node.children && (
        <motion.div initial={false}
          animate={{ height: 'auto', opacity: 1 }}
          className="overflow-hidden"
        >
          {node.children.map((child) => (
            <TreeItem key={child.path} node={child} onSelectFile={onSelectFile} level={level + 1} defaultOpen={defaultOpen} />
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
  const [filterQuery, setFilterQuery] = useState('');

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
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-center">
          <Folder size={40} className="text - text-muted/30 mx-auto mb-3" />
          <p className="text - text-muted text-sm">Watch a directory from the Dashboard to explore files.</p>
        </div>
      </div>
    );
  }

  const tree = buildTree(files);
  const filteredTree = filterTree(tree, filterQuery);

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="px-4 sm:px-6 lg:px-8 py-5 flex flex-col sm:flex-row sm:justify-between sm:items-center gap-3">
        <div>
          <h2 className="text-2xl sm:text-3xl font-semibold mb-1">Explorer</h2>
          <p className="text - text-muted text-sm">
            {files.length} file{files.length !== 1 ? 's' : ''} tracked • Click a file to view history
          </p>
        </div>
        <button
          onClick={loadFiles}
          disabled={loading}
          className="flex items-center gap-2 text-brand-400 hover:text-brand-300 transition-colors bg-brand-500/10 hover:bg-brand-500/15 px-3 py-2 rounded-xl text-xs font-medium disabled:opacity-50 self-start sm:self-auto"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          Refresh
        </button>
      </div>

      {/* Search filter */}
      <div className="px-4 sm:px-6 lg:px-8 mb-4">
        <div className="relative max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text - text-muted" size={16} />
          <input
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Filter files..."
            className="w-full bg-surface border border-border rounded-xl pl-9 pr-4 py-2 text-sm outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/30 transition-all placeholder:text - text-muted/50"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 sm:px-6 lg:px-8 pb-8">
        <div className="glass-panel p-3 sm:p-4 rounded-2xl max-w-2xl border-border">
          {loading ? (
            <div className="flex items-center gap-3 p-4 text - text-muted text-sm">
              <span className="h-4 w-4 border-2 border-brand-400/30 border-t-brand-400 rounded-full animate-spin" />
              Loading files...
            </div>
          ) : filteredTree.length === 0 ? (
            <p className="text - text-muted text-sm p-4">
              {filterQuery ? 'No files match your filter.' : 'No tracked files found.'}
            </p>
          ) : (
            <div className="space-y-0.5">
              {filteredTree.map(node => (
                <TreeItem
                  key={node.path}
                  node={node}
                  onSelectFile={onSelectFile}
                  defaultOpen={!!filterQuery}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
