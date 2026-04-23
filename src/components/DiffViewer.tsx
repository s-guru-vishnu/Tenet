import { useMemo } from 'react';
import { diffLines } from 'diff';
import { motion } from 'framer-motion';

interface DiffViewerProps {
  oldStr: string;
  newStr: string;
}

export default function DiffViewer({ oldStr, newStr }: DiffViewerProps) {
  const diffs = useMemo(() => {
    return diffLines(oldStr, newStr);
  }, [oldStr, newStr]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="font-mono text-[13px] bg-[#0A0C10] rounded-xl border border-border mt-4 max-h-[600px] overflow-hidden flex flex-col shadow-2xl shadow-brand-500/5 ring-1 ring-white/5"
    >
      <div className="flex bg-surface/80 backdrop-blur px-4 py-3 border-b border-white/5 text - text-muted text-xs font-semibold tracking-wider">
        <span className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-red-400/80"></div> Removed
        </span>
        <span className="mx-4 text-white/20">|</span>
        <span className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-emerald-400/80"></div> Added
        </span>
      </div>
      <div className="overflow-y-auto p-2 custom-scrollbar">
        {diffs.map((part, index) => {
          const isAdded = part.added;
          const isRemoved = part.removed;

          let bgColor = 'transparent';
          let textColor = 'text-gray-300';
          let prefix = ' ';

          if (isAdded) {
            bgColor = 'bg-emerald-500/10 border-l border-emerald-500';
            textColor = 'text-emerald-300';
            prefix = '+';
          } else if (isRemoved) {
            bgColor = 'bg-red-500/10 border-l border-red-500';
            textColor = 'text-red-300';
            prefix = '-';
          } else {
            bgColor = 'border-l border-transparent';
          }

          return (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: index * 0.05 }}
              key={index}
              className={`${bgColor} ${textColor} my-[1px] rounded-r-md`}
            >
              {part.value.split('\n').map((line, i, arr) => {
                if (i === arr.length - 1 && line === '') return null;
                return (
                  <div key={i} className="flex group hover:bg-white/5 transition-colors px-2 py-[2px]">
                    <span className="select-none w-6 text-right mr-4 opacity-30 group-hover:opacity-80 transition-opacity">
                      {prefix}
                    </span>
                    <span className="whitespace-pre break-words break-all">{line || ' '}</span>
                  </div>
                );
              })}
            </motion.div>
          );
        })}
      </div>
    </motion.div>
  );
}
