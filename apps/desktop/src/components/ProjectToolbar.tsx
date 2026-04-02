import React from 'react';
import { motion } from 'framer-motion';
import { FileText, Scan, Shield, Search, Share2, GitBranch, History } from 'lucide-react';
import { TooltipProvider, TooltipSimple } from '@opc/ui/tooltip-modern';
import { useTabState } from '@/hooks/useTabState';
import type { Tab } from '@/contexts/TabContext';

function getProjectPath(tab: Tab | undefined): string | null {
  if (!tab) return null;
  if (tab.type === 'chat') return tab.initialProjectPath || null;
  if (tab.type === 'agent-execution') return tab.projectPath || null;
  return null;
}

const tools = [
  { key: 'claude-md', icon: FileText, label: 'CLAUDE.md', action: 'createClaudeMdTab' },
  { key: 'git-context', icon: GitBranch, label: 'Git Context', action: 'createGitContextTab' },
  { key: 'xray', icon: Scan, label: 'Xray Analysis', action: 'createXrayTab' },
  { key: 'governance', icon: Shield, label: 'Governance', action: 'createGovernanceTab' },
  { key: 'semantic-search', icon: Search, label: 'Semantic Search', action: 'createSemanticSearchTab' },
  { key: 'call-graph', icon: Share2, label: 'Call Graph', action: 'createCallGraphTab' },
  { key: 'checkpoint', icon: History, label: 'Checkpoint', action: 'createCheckpointTab' },
] as const;

type TabActions = ReturnType<typeof useTabState>;
type ToolAction = (typeof tools)[number]['action'];

export const ProjectToolbar: React.FC = () => {
  const tabState = useTabState();
  const { activeTab } = tabState;
  const projectPath = getProjectPath(activeTab);

  if (!projectPath) return null;

  return (
    <TooltipProvider>
      <div className="flex items-center h-9 px-4 gap-0.5 bg-background/80 border-b border-border/30">
        <span className="text-[11px] text-muted-foreground/60 uppercase tracking-wider font-medium mr-2 select-none">
          Tools
        </span>
        {tools.map(({ key, icon: Icon, label, action }) => (
          <TooltipSimple key={key} content={label} side="bottom">
            <motion.button
              onClick={() => (tabState[action as ToolAction] as TabActions[ToolAction])()}
              whileTap={{ scale: 0.95 }}
              transition={{ duration: 0.1 }}
              className="p-1.5 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors text-muted-foreground"
            >
              <Icon size={14} />
            </motion.button>
          </TooltipSimple>
        ))}
      </div>
    </TooltipProvider>
  );
};
