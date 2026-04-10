import { create } from 'zustand';

export type VersionType = 'Snapshot' | 'Delta' | 'Deletion';

export interface FileVersion {
  hash: string;
  timestamp: string;
  size: number;
  version_type: VersionType;
}

export interface FileEntry {
  path: string;
  versions: FileVersion[];
}

export interface StatusReport {
  watched_dir: string;
  created_at: string;
  last_updated: string;
  file_count: number;
  version_count: number;
  blob_count: number;
  storage_size: number;
}

interface AppState {
  currentPath: string;
  status: StatusReport | null;
  selectedFile: string | null;
  fileHistory: FileEntry | null;
  
  // Actions
  setCurrentPath: (path: string) => void;
  setStatus: (status: StatusReport | null) => void;
  setSelectedFile: (file: string | null) => void;
  setFileHistory: (history: FileEntry | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentPath: '',
  status: null,
  selectedFile: null,
  fileHistory: null,
  setCurrentPath: (path) => set({ currentPath: path }),
  setStatus: (status) => set({ status }),
  setSelectedFile: (file) => set({ selectedFile: file }),
  setFileHistory: (history) => set({ fileHistory: history }),
}));
