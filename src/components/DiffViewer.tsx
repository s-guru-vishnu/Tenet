import { useMemo } from 'react';
import { diffLines } from 'diff';
import { motion } from 'framer-motion';

interface DiffViewerProps {
  oldStr: string;
  newStr: string;
}

interface DiffLine {
  leftNum: number | null;
  rightNum: number | null;
  leftContent: string | null;
  rightContent: string | null;
  type: 'common' | 'added' | 'removed' | 'modified';
}

export default function DiffViewer({ oldStr, newStr }: DiffViewerProps) {
  const diffLinesData = useMemo(() => {
    const changes = diffLines(oldStr, newStr);
    const lines: DiffLine[] = [];
    
    let leftLineNum = 1;
    let rightLineNum = 1;

    // Process changes into a side-by-side format
    for (let i = 0; i < changes.length; i++) {
      const change = changes[i];
      const nextChange = i + 1 < changes.length ? changes[i + 1] : null;

      const changeLines = change.value.split('\n');
      if (changeLines[changeLines.length - 1] === '') {
        changeLines.pop(); // Remove trailing empty line from split
      }

      if (change.removed && nextChange?.added) {
        // Modified lines (removed followed by added)
        const nextLines = nextChange.value.split('\n');
        if (nextLines[nextLines.length - 1] === '') nextLines.pop();

        const maxLines = Math.max(changeLines.length, nextLines.length);
        for (let j = 0; j < maxLines; j++) {
          lines.push({
            leftNum: j < changeLines.length ? leftLineNum++ : null,
            rightNum: j < nextLines.length ? rightLineNum++ : null,
            leftContent: j < changeLines.length ? changeLines[j] : null,
            rightContent: j < nextLines.length ? nextLines[j] : null,
            type: 'modified',
          });
        }
        i++; // Skip the next change since we handled it
      } else if (change.removed) {
        // Only removed
        for (const line of changeLines) {
          lines.push({
            leftNum: leftLineNum++,
            rightNum: null,
            leftContent: line,
            rightContent: null,
            type: 'removed',
          });
        }
      } else if (change.added) {
        // Only added
        for (const line of changeLines) {
          lines.push({
            leftNum: null,
            rightNum: rightLineNum++,
            leftContent: null,
            rightContent: line,
            type: 'added',
          });
        }
      } else {
        // Common
        for (const line of changeLines) {
          lines.push({
            leftNum: leftLineNum++,
            rightNum: rightLineNum++,
            leftContent: line,
            rightContent: line,
            type: 'common',
          });
        }
      }
    }
    
    return lines;
  }, [oldStr, newStr]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="font-mono text-[13px] bg-[#0A0C10] rounded-xl border border-border mt-4 max-h-[600px] overflow-hidden flex flex-col shadow-2xl shadow-brand-500/5 ring-1 ring-white/5"
    >
      <div className="flex bg-surface/80 backdrop-blur border-b border-white/5 text-text-muted text-xs font-semibold tracking-wider">
        <div className="flex-1 py-3 px-4 border-r border-white/5 flex items-center gap-2">
           <div className="w-2 h-2 rounded-full bg-red-400/80"></div> Previous Version
        </div>
        <div className="flex-1 py-3 px-4 flex items-center gap-2">
           <div className="w-2 h-2 rounded-full bg-emerald-400/80"></div> Current Version
        </div>
      </div>
      
      <div className="overflow-y-auto custom-scrollbar flex-1">
        <div className="flex min-w-max pb-2">
          {/* Left Column (Old) */}
          <div className="flex-1 border-r border-white/5">
            {diffLinesData.map((line, i) => (
              <div 
                key={`left-${i}`} 
                className={`flex h-[22px] items-center ${
                  line.type === 'removed' || (line.type === 'modified' && line.leftContent !== null) 
                    ? 'bg-red-500/10 text-red-300' 
                    : line.type === 'added' ? 'bg-surface/30' : 'text-gray-300 hover:bg-white/5'
                }`}
              >
                <div className="w-10 shrink-0 text-right pr-3 text-xs opacity-40 select-none">
                  {line.leftNum || ''}
                </div>
                <div className="w-6 shrink-0 text-center opacity-50 select-none">
                  {line.type === 'removed' || (line.type === 'modified' && line.leftContent !== null) ? '-' : ''}
                </div>
                <div className="whitespace-pre break-all pl-1">{line.leftContent || ' '}</div>
              </div>
            ))}
          </div>

          {/* Right Column (New) */}
          <div className="flex-1">
            {diffLinesData.map((line, i) => (
              <div 
                key={`right-${i}`} 
                className={`flex h-[22px] items-center ${
                  line.type === 'added' || (line.type === 'modified' && line.rightContent !== null) 
                    ? 'bg-emerald-500/10 text-emerald-300' 
                    : line.type === 'removed' ? 'bg-surface/30' : 'text-gray-300 hover:bg-white/5'
                }`}
              >
                <div className="w-10 shrink-0 text-right pr-3 text-xs opacity-40 select-none border-l border-white/5">
                  {line.rightNum || ''}
                </div>
                <div className="w-6 shrink-0 text-center opacity-50 select-none">
                  {line.type === 'added' || (line.type === 'modified' && line.rightContent !== null) ? '+' : ''}
                </div>
                <div className="whitespace-pre break-all pl-1">{line.rightContent || ' '}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </motion.div>
  );
}
