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

export const ProjectToolbar: React.FC = () => {
  const {
    activeTab,
    createClaudeMdTab,
    createGitContextTab,
    createXrayTab,
    createGovernanceTab,
    createSemanticSearchTab,
    createCallGraphTab,
    createCheckpointTab,
  } = useTabState();
  const projectPath = getProjectPath(activeTab);

  if (!projectPath) return null;

  const tools = [
    { key: 'claude-md', icon: FileText, label: 'CLAUDE.md', onClick: () => createClaudeMdTab() },
    { key: 'git-context', icon: GitBranch, label: 'Git Context', onClick: () => createGitContextTab(projectPath) },
    { key: 'xray', icon: Scan, label: 'Xray Analysis', onClick: () => createXrayTab(projectPath) },
    { key: 'governance', icon: Shield, label: 'Governance', onClick: () => createGovernanceTab(projectPath) },
    { key: 'semantic-search', icon: Search, label: 'Semantic Search', onClick: () => createSemanticSearchTab(projectPath) },
    { key: 'call-graph', icon: Share2, label: 'Call Graph', onClick: () => createCallGraphTab(projectPath) },
    { key: 'checkpoint', icon: History, label: 'Checkpoint', onClick: () => createCheckpointTab(projectPath) },
  ];

  return (
    <TooltipProvider>
      <div className="flex items-center h-9 px-4 gap-0.5 bg-background/80 border-b border-border/30">
        <span className="text-[11px] text-muted-foreground/60 uppercase tracking-wider font-medium mr-2 select-none">
          Tools
        </span>
        {tools.map(({ key, icon: Icon, label, onClick }) => (
          <TooltipSimple key={key} content={label} side="bottom">
            <motion.button
              onClick={onClick}
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
