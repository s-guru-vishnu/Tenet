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

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  isLoading?: boolean;
}

interface AppState {
  currentPath: string;
  status: StatusReport | null;
  selectedFile: string | null;
  fileHistory: FileEntry | null;
  notifications: Notification[];
  // AI Agent
  apiKey: string;
  llmProvider: 'groq' | 'openai' | 'anthropic' | 'gemini';
  chatMessages: ChatMessage[];

  // Actions
  setCurrentPath: (path: string) => void;
  setStatus: (status: StatusReport | null) => void;
  setSelectedFile: (file: string | null) => void;
  setFileHistory: (history: FileEntry | null) => void;
  addNotification: (type: 'success' | 'error' | 'info', message: string) => void;
  removeNotification: (id: string) => void;
  setApiKey: (key: string) => void;
  setLlmProvider: (provider: 'groq' | 'openai' | 'anthropic' | 'gemini') => void;
  addChatMessage: (msg: Omit<ChatMessage, 'id' | 'timestamp'>) => string;
  updateChatMessage: (id: string, updates: Partial<ChatMessage>) => void;
  clearChat: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentPath: '',
  status: null,
  selectedFile: null,
  fileHistory: null,
  notifications: [],
  apiKey: '',
  llmProvider: 'groq',
  chatMessages: [],

  setCurrentPath: (path) => set({ currentPath: path }),
  setStatus: (status) => set({ status }),
  setSelectedFile: (file) => set({ selectedFile: file }),
  setFileHistory: (history) => set({ fileHistory: history }),

  setApiKey: (key) => set({ apiKey: key }),
  setLlmProvider: (provider) => set({ llmProvider: provider }),

  addChatMessage: (msg) => {
    const id = Math.random().toString(36).substring(2, 11);
    const timestamp = new Date();
    set((state) => ({
      chatMessages: [...state.chatMessages, { ...msg, id, timestamp }],
    }));
    return id;
  },

  updateChatMessage: (id, updates) =>
    set((state) => ({
      chatMessages: state.chatMessages.map((m) =>
        m.id === id ? { ...m, ...updates } : m
      ),
    })),

  clearChat: () => set({ chatMessages: [] }),

  addNotification: (type, message) => {
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => ({
      notifications: [...state.notifications, { id, type, message }],
    }));
    setTimeout(() => {
      set((state) => ({
        notifications: state.notifications.filter((n) => n.id !== id),
      }));
    }, 5000);
  },
  removeNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),
}));

