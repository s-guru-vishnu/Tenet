import { motion, AnimatePresence } from 'framer-motion';
import { CheckCircle, XCircle, Info, X } from 'lucide-react';
import { useAppStore } from '../store/useAppStore';

export default function Notifications() {
  const { notifications, removeNotification } = useAppStore();

  return (
    <div className="fixed bottom-4 right-4 sm:bottom-6 sm:right-6 z-50 flex flex-col gap-2.5 pointer-events-none max-w-[calc(100vw-2rem)]">
      <AnimatePresence mode="popLayout">
        {notifications.map((n) => (
          <motion.div
            key={n.id}
            layout
            initial={{ opacity: 0, y: 20, scale: 0.9 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, x: 50, transition: { duration: 0.2 } }}
            className={`
              pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-xl shadow-2xl min-w-[260px] sm:min-w-[300px] max-w-[400px]
              backdrop-blur-xl border
              ${n.type === 'success' ? 'bg-emerald-950/80 text-emerald-100 border-emerald-500/30' : 
                n.type === 'error' ? 'bg-red-950/80 text-red-100 border-red-500/30' : 
                'bg-blue-950/80 text-blue-100 border-blue-500/30'}
            `}
          >
            <div className={`shrink-0 ${
              n.type === 'success' ? 'text-emerald-400' : 
              n.type === 'error' ? 'text-red-400' : 'text-blue-400'
            }`}>
              {n.type === 'success' && <CheckCircle size={18} />}
              {n.type === 'error' && <XCircle size={18} />}
              {n.type === 'info' && <Info size={18} />}
            </div>
            
            <p className="flex-1 text-sm font-medium leading-snug">{n.message}</p>

            <button 
              onClick={() => removeNotification(n.id)}
              className="shrink-0 text-white/30 hover:text-white/70 transition-colors p-0.5"
            >
              <X size={16} />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
