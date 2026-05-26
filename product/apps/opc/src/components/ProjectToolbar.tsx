import React from 'react';
import { motion } from 'framer-motion';
import { Activity, FileText, Scan, Shield, Search, Share2, GitBranch, History, LayoutGrid, ShieldCheck } from 'lucide-react';
import { TooltipProvider, TooltipSimple } from '@opc/ui/tooltip-modern';
import { useTabState } from '@/hooks/useTabState';
import { api } from '@/lib/api';
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
    createSpecMarkdownTab,
    createGitContextTab,
    createXrayTab,
    createGovernanceTab,
    createSemanticSearchTab,
    createCallGraphTab,
    createCheckpointTab,
    createLiveSessionsTab,
    createPortfolioTab,
    createPromotionTab,
  } = useTabState();
  const projectPath = getProjectPath(activeTab);

  if (!projectPath) return null;

  // When a project is active, prefer the project-root CLAUDE.md over the
  // global ~/.claude/CLAUDE.md system prompt. Deeper CLAUDE.md files in
  // the tree are surfaced by the CLAUDE.md Memories dropdown on the
  // session screen — this button is the root-only fast path.
  const handleClaudeMd = async () => {
    try {
      const files = await api.findClaudeMdFiles(projectPath);
      const rootCandidates = files
        .filter((f) => f.relative_path === 'CLAUDE.md')
        .sort((a, b) => a.absolute_path.length - b.absolute_path.length);
      if (rootCandidates.length > 0) {
        createSpecMarkdownTab(rootCandidates[0].absolute_path, 'CLAUDE.md');
        return;
      }
    } catch {
      // fall through to the global system-prompt tab
    }
    createClaudeMdTab();
  };

  const tools = [
    { key: 'claude-md', icon: FileText, label: 'CLAUDE.md', onClick: () => { void handleClaudeMd(); } },
    { key: 'git-context', icon: GitBranch, label: 'Git Context', onClick: () => createGitContextTab(projectPath) },
    { key: 'xray', icon: Scan, label: 'Xray Analysis', onClick: () => createXrayTab(projectPath) },
    { key: 'governance', icon: Shield, label: 'Governance', onClick: () => createGovernanceTab(projectPath) },
    { key: 'semantic-search', icon: Search, label: 'Semantic Search', onClick: () => createSemanticSearchTab(projectPath) },
    { key: 'call-graph', icon: Share2, label: 'Call Graph', onClick: () => createCallGraphTab(projectPath) },
    { key: 'checkpoint', icon: History, label: 'Checkpoint', onClick: () => createCheckpointTab(projectPath) },
    { key: 'live-sessions', icon: Activity, label: 'Live Sessions', onClick: () => createLiveSessionsTab(projectPath) },
    { key: 'portfolio', icon: LayoutGrid, label: 'Portfolio', onClick: () => createPortfolioTab(projectPath) },
    { key: 'promotion', icon: ShieldCheck, label: 'Promotion', onClick: () => createPromotionTab(projectPath) },
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
