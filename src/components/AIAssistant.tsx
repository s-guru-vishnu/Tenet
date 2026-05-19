import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Bot,
  Send,
  Trash2,
  User,
  AlertTriangle,
  Cpu,
  Loader2,
} from 'lucide-react';
import { useAppStore, type ChatMessage } from '../store/useAppStore';

// ─── Message Bubble ────────────────────────────────────────────────────────────

function MessageBubble({ msg }: { msg: ChatMessage }) {
  const isUser = msg.role === 'user';
  return (
    <motion.div
      initial={{ opacity: 0, y: 12, scale: 0.97 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.25, ease: 'easeOut' }}
      className={`flex gap-3 ${isUser ? 'flex-row-reverse' : 'flex-row'}`}
    >
      <div
        className={`shrink-0 h-8 w-8 rounded-full flex items-center justify-center text-sm font-bold shadow-md
          ${isUser
            ? 'bg-brand-600 text-white'
            : 'bg-gradient-to-br from-purple-500 to-brand-500 text-white'
          }`}
      >
        {isUser ? <User size={15} /> : <Bot size={15} />}
      </div>
      <div
        className={`max-w-[78%] px-4 py-3 rounded-2xl text-sm leading-relaxed shadow-sm
          ${isUser
            ? 'bg-brand-600 text-white rounded-tr-sm'
            : 'bg-surface border border-border text-text-main rounded-tl-sm'
          }`}
      >
        {msg.isLoading ? (
          <span className="flex items-center gap-2 text-text-muted">
            <Loader2 size={14} className="animate-spin" />
            Thinking…
          </span>
        ) : (
          <pre className="whitespace-pre-wrap font-sans">{msg.content}</pre>
        )}
        {!msg.isLoading && (
          <p className={`text-[10px] mt-1.5 ${isUser ? 'text-white/60 text-right' : 'text-text-muted'}`}>
            {msg.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          </p>
        )}
      </div>
    </motion.div>
  );
} 

const PROVIDERS = [
  { id: 'groq' as const, name: 'Groq', model: 'Llama 3.3 70B', icon: '', color: 'from-orange-500 to-amber-500', link: 'https://console.groq.com/keys' },
  { id: 'openai' as const, name: 'OpenAI', model: 'GPT-4o Mini', icon: '', color: 'from-green-500 to-emerald-500', link: 'https://platform.openai.com/api-keys' },
  { id: 'anthropic' as const, name: 'Claude', model: 'Sonnet 3.5', icon: '', color: 'from-amber-500 to-yellow-500', link: 'https://console.anthropic.com/settings/keys' },
  { id: 'gemini' as const, name: 'Gemini', model: '2.0 Flash', icon: '', color: 'from-blue-500 to-cyan-500', link: 'https://aistudio.google.com/app/apikey' },
];

function ApiKeyGate() {
  const { apiKey, setApiKey, llmProvider, setLlmProvider } = useAppStore();
  const [draft, setDraft] = useState(apiKey);

  const selected = PROVIDERS.find(p => p.id === llmProvider) || PROVIDERS[0];

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      className="flex-1 flex items-center justify-center p-6"
    >
      <div className="glass-panel rounded-2xl p-6 max-w-md w-full shadow-xl">
        <div className="flex items-center gap-3 mb-4">
          <div className="h-10 w-10 rounded-xl bg-brand-500/10 flex items-center justify-center">
            <Cpu size={20} className="text-brand-400" />
          </div>
          <div>
            <h3 className="font-semibold text-text-main">Connect AI Provider</h3>
            <p className="text-xs text-text-muted">Choose a model to power TENET AI</p>
          </div>
        </div>

        {/* Provider cards */}
        <div className="grid grid-cols-2 gap-2 mb-4">
          {PROVIDERS.map((p) => (
            <button
              key={p.id}
              onClick={() => setLlmProvider(p.id)}
              className={`relative flex flex-col items-start gap-1 p-3 rounded-xl border text-left transition-all duration-200
                ${llmProvider === p.id
                  ? 'border-brand-500/50 bg-brand-500/10 shadow-md shadow-brand-500/10'
                  : 'border-border/50 bg-surface/50 hover:border-border hover:bg-surface-hover'
                }`}
            >
              <div className="flex items-center gap-2 w-full">
                <span className="text-lg">{p.icon}</span>
                <span className={`text-sm font-semibold ${llmProvider === p.id ? 'text-text-main' : 'text-text-muted'}`}>
                  {p.name}
                </span>
              </div>
              <span className="text-[10px] text-text-muted/70 ml-7">{p.model}</span>
            </button>
          ))}
        </div>

        {/* API Key input */}
        <div className="flex gap-2 mb-3">
          <input
            id="api-key-input"
            type="password"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && draft.trim() && setApiKey(draft.trim())}
            placeholder={`Enter your ${selected.name} API key`}
            className="flex-1 bg-background border border-border rounded-xl px-3 py-2.5 text-sm outline-none
                       focus:border-brand-500 focus:ring-1 focus:ring-brand-500/30 transition-all
                       placeholder:text-text-muted/40 font-mono"
          />
          <button
            id="save-api-key-btn"
            onClick={() => draft.trim() && setApiKey(draft.trim())}
            className="bg-brand-600 hover:bg-brand-500 text-white px-4 py-2.5 rounded-xl text-sm font-medium
                       transition-all shadow-lg shadow-brand-500/20 active:scale-95"
          >
            Save
          </button>
        </div>

        <p className="text-[11px] text-text-muted leading-relaxed">
          Key stays in this session only.{' '}
          <a href={selected.link} target="_blank" rel="noreferrer"
            className="text-brand-400 hover:text-brand-300 underline">
            Get a {selected.name} Key
          </a>
        </p>
      </div>
    </motion.div>
  );
}

// ─── Input Bar (reused in both centered & bottom positions) ────────────────────

function InputBar({
  inputValue, setInputValue, isLoading, onSend, onKeyDown, inputRef,
}: {
  inputValue: string;
  setInputValue: (v: string) => void;
  isLoading: boolean;
  onSend: () => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  inputRef: React.RefObject<HTMLTextAreaElement | null>;
}) {
  return (
    <div className="glass-panel rounded-2xl p-1 flex items-end gap-2 max-w-2xl w-full shadow-lg mx-auto">
      <textarea
        ref={inputRef}
        id="ai-chat-input"
        value={inputValue}
        onChange={(e) => setInputValue(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder='Ask anything about your files…'
        rows={1}
        disabled={isLoading}
        className="flex-1 bg-transparent resize-none px-3 py-2.5 text-sm outline-none
                   text-text-main placeholder:text-text-muted/50 min-h-[42px] max-h-32
                   disabled:opacity-50 leading-relaxed"
        style={{ scrollbarWidth: 'none' }}
        onInput={(e) => {
          const el = e.currentTarget;
          el.style.height = 'auto';
          el.style.height = `${el.scrollHeight}px`;
        }}
      />
      <button
        id="ai-send-btn"
        onClick={onSend}
        disabled={!inputValue.trim() || isLoading}
        className="shrink-0 h-9 w-9 rounded-xl bg-brand-600 hover:bg-brand-500
                   disabled:bg-surface-hover disabled:text-text-muted
                   text-white flex items-center justify-center transition-all
                   active:scale-95 shadow-md shadow-brand-500/20 mb-0.5 mr-0.5"
      >
        {isLoading ? <Loader2 size={16} className="animate-spin" /> : <Send size={16} />}
      </button>
    </div>
  );
}

// ─── Main AI Assistant Component ───────────────────────────────────────────────

export default function AIAssistant() {
  const {
    currentPath, apiKey, llmProvider, chatMessages,
    addChatMessage, updateChatMessage, clearChat,
  } = useAppStore();

  const [inputValue, setInputValue] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages]);

  const handleSend = async (query?: string) => {
    const text = (query ?? inputValue).trim();
    if (!text || isLoading) return;
    setInputValue('');
    setIsLoading(true);
    addChatMessage({ role: 'user', content: text });
    const assistantId = addChatMessage({ role: 'assistant', content: '', isLoading: true });
    try {
      const response = await invoke<string>('run_agent', {
        query: text, path: currentPath, apiKey: apiKey, provider: llmProvider,
      });
      updateChatMessage(assistantId, { content: response, isLoading: false });
    } catch (err: any) {
      updateChatMessage(assistantId, {
        content: `⚠️ ${err?.toString() ?? 'Unknown error'}`, isLoading: false,
      });
    } finally {
      setIsLoading(false);
      inputRef.current?.focus();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); }
  };

  if (!currentPath) {
    return (
      <div className="flex-1 flex items-center justify-center p-6">
        <div className="text-center">
          <div className="h-14 w-14 rounded-2xl bg-brand-500/10 flex items-center justify-center mx-auto mb-4">
            <Bot size={28} className="text-brand-400" />
          </div>
          <h3 className="font-semibold text-text-main mb-2">TENET AI Agent</h3>
          <p className="text-sm text-text-muted">Watch a directory from the Dashboard first, then come back here.</p>
        </div>
      </div>
    );
  }

  if (!apiKey) return <ApiKeyGate />;

  const hasMessages = chatMessages.length > 0;

  return (
    <div className="flex-1 flex flex-col h-full min-h-0">
      {/* Header */}
      <div className="px-4 sm:px-6 lg:px-8 py-5 border-b border-border/50 shrink-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-purple-500/20 to-brand-500/20 border border-purple-500/20 flex items-center justify-center">
              <Cpu size={20} className="text-purple-400" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">TENET AI</h2>
              <p className="text-xs text-text-muted">File Control Agent</p>
            </div>
          </div>
          {hasMessages && (
            <button
              id="clear-chat-btn"
              onClick={clearChat}
              title="Clear conversation"
              className="p-2 rounded-lg text-text-muted hover:text-red-400 hover:bg-red-400/10 transition-colors"
            >
              <Trash2 size={16} />
            </button>
          )}
        </div>
      </div>

      {/* ── Empty state: centered greeting + input ── */}
      {!hasMessages && (
        <div className="flex-1 flex flex-col items-center justify-center px-6">
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            className="text-center mb-8"
          >
            <h3 className="text-xl font-semibold text-text-main mb-2">What can I help you with?</h3>
            <p className="text-sm text-text-muted max-w-sm">
              Explore file history, restore versions, or compare changes using natural language.
            </p>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="w-full max-w-2xl"
          >
            <InputBar
              inputValue={inputValue}
              setInputValue={setInputValue}
              isLoading={isLoading}
              onSend={() => handleSend()}
              onKeyDown={handleKeyDown}
              inputRef={inputRef}
            />
            <p className="text-[10px] text-text-muted mt-2 text-center">
              Press Enter to send · Shift+Enter for new line ·{' '}
              <button
                onClick={() => useAppStore.getState().setApiKey('')}
                className="text-brand-400/70 hover:text-brand-400 transition-colors"
              >
                Change API key
              </button>
            </p>
          </motion.div>
        </div>
      )}

      {/* ── Chat mode: messages + bottom input ── */}
      {hasMessages && (
        <>
          <div className="flex-1 overflow-y-auto px-4 sm:px-6 lg:px-8 py-6 space-y-4 min-h-0">
            <AnimatePresence>
              {chatMessages.map((msg) => (
                <MessageBubble key={msg.id} msg={msg} />
              ))}
            </AnimatePresence>
            <div ref={bottomRef} />
          </div>

          <div className="shrink-0 px-4 sm:px-6 lg:px-8 pb-6 pt-3 border-t border-border/50">
            <InputBar
              inputValue={inputValue}
              setInputValue={setInputValue}
              isLoading={isLoading}
              onSend={() => handleSend()}
              onKeyDown={handleKeyDown}
              inputRef={inputRef}
            />
            <p className="text-[10px] text-text-muted mt-2 text-center">
              Press Enter to send · Shift+Enter for new line ·{' '}
              <button
                onClick={() => useAppStore.getState().setApiKey('')}
                className="text-brand-400/70 hover:text-brand-400 transition-colors"
              >
                Change API key
              </button>
            </p>
          </div>
        </>
      )}
    </div>
  );
}
