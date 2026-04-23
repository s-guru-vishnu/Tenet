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

export interface Notification {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
}

interface AppState {
  currentPath: string;
  status: StatusReport | null;
  selectedFile: string | null;
  fileHistory: FileEntry | null;
  notifications: Notification[];
  
  // Actions
  setCurrentPath: (path: string) => void;
  setStatus: (status: StatusReport | null) => void;
  setSelectedFile: (file: string | null) => void;
  setFileHistory: (history: FileEntry | null) => void;
  addNotification: (type: 'success' | 'error' | 'info', message: string) => void;
  removeNotification: (id: string) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentPath: '',
  status: null,
  selectedFile: null,
  fileHistory: null,
  notifications: [],
  setCurrentPath: (path) => set({ currentPath: path }),
  setStatus: (status) => set({ status }),
  setSelectedFile: (file) => set({ selectedFile: file }),
  setFileHistory: (history) => set({ fileHistory: history }),
  addNotification: (type, message) => {
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => ({
      notifications: [...state.notifications, { id, type, message }]
    }));
    // Auto-remove after 5 seconds
    setTimeout(() => {
      set((state) => ({
        notifications: state.notifications.filter((n) => n.id !== id)
      }));
    }, 5000);
  },
  removeNotification: (id) => set((state) => ({
    notifications: state.notifications.filter((n) => n.id !== id)
  })),
}));
