import React, { createContext, useState, useContext, useCallback, useEffect, useRef } from 'react';
import { api } from '../lib/api';

// Types matching the Rust specta types
export interface AuthUser {
  id: string;
  email: string;
  name: string;
  github_login: string;
  avatar_url: string;
}

export interface AuthOrg {
  org_id: string;
  org_slug: string;
  github_org_login: string;
  platform_role: string;
}

type AuthStatus = 'loading' | 'unauthenticated' | 'authenticated' | 'org-selection';

interface AuthContextType {
  status: AuthStatus;
  user: AuthUser | null;
  org: AuthOrg | null;
  availableOrgs: AuthOrg[];
  pendingOrgs: AuthOrg[] | null;
  pendingId: string | null;
  error: string | null;
  login: () => Promise<void>;
  selectOrg: (orgId: string) => Promise<void>;
  switchOrg: (orgId: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>('loading');
  const [user, setUser] = useState<AuthUser | null>(null);
  const [org, setOrg] = useState<AuthOrg | null>(null);
  const [pendingOrgs, setPendingOrgs] = useState<AuthOrg[] | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [availableOrgs, setAvailableOrgs] = useState<AuthOrg[]>([]);
  const [error, setError] = useState<string | null>(null);
  const refreshTimerRef = useRef<number | null>(null);

  // Check auth status on mount
  useEffect(() => {
    (async () => {
      try {
        const result = await api.authGetStatus();
        if (result.authenticated && result.user && result.org) {
          setUser(result.user);
          setOrg(result.org);
          setStatus('authenticated');
          scheduleRefresh(result.expires_at);
        } else {
          setStatus('unauthenticated');
        }
      } catch {
        setStatus('unauthenticated');
      }
    })();
  }, []);

  // Listen for deep-link auth callbacks
  useEffect(() => {
    if (!window.__TAURI__) return;
    let unlisten: (() => void) | undefined;
    (async () => {
      const { listen } = await import('@tauri-apps/api/event');
      unlisten = await listen<string>('auth-callback', async (event) => {
        try {
          setError(null);
          const result = await api.authHandleCallback(event.payload);
          handleAuthResult(result);
        } catch (err) {
          setError(String(err));
          setStatus('unauthenticated');
        }
      });
    })();
    return () => { unlisten?.(); };
  }, []);

  // Cleanup refresh timer
  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) window.clearInterval(refreshTimerRef.current);
    };
  }, []);

  function scheduleRefresh(expiresAt: number | null) {
    if (refreshTimerRef.current) window.clearInterval(refreshTimerRef.current);
    if (!expiresAt) return;
    // Check every 60s, refresh when within 5 min of expiry
    refreshTimerRef.current = window.setInterval(async () => {
      const now = Math.floor(Date.now() / 1000);
      if (expiresAt - now < 300) {
        try {
          const newExpiresAt = await api.authRefreshToken();
          scheduleRefresh(newExpiresAt);
        } catch {
          // Refresh failed — force re-login
          setStatus('unauthenticated');
          setUser(null);
          setOrg(null);
        }
      }
    }, 60_000);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function handleAuthResult(result: any) {
    if (result.type === 'authenticated') {
      setUser(result.user);
      setOrg(result.org);
      if (result.available_orgs) setAvailableOrgs(result.available_orgs);
      else if (result.org) setAvailableOrgs([result.org]);
      setPendingOrgs(null);
      setPendingId(null);
      setStatus('authenticated');
      scheduleRefresh(result.expires_at);
    } else if (result.type === 'org_selection') {
      setUser(result.user);
      setPendingOrgs(result.orgs);
      setAvailableOrgs(result.orgs ?? []);
      setPendingId(result.pending_id);
      setStatus('org-selection');
    } else if (result.type === 'error') {
      setError(result.message || result.code);
      setStatus('unauthenticated');
    }
  }

  const login = useCallback(async () => {
    setError(null);
    setStatus('loading');
    try {
      await api.authStartLogin();
      // Browser will open — callback arrives via deep-link event listener above
    } catch {
      setError('Could not open browser. Check your default browser settings.');
      setStatus('unauthenticated');
    }
  }, []);

  const selectOrg = useCallback(async (orgId: string) => {
    if (!pendingId) return;
    setStatus('loading');
    try {
      const result = await api.authSelectOrg(pendingId, orgId);
      handleAuthResult(result);
    } catch (err) {
      setError(String(err));
      setStatus('org-selection');
    }
  }, [pendingId]);

  const switchOrg = useCallback(async (orgId: string) => {
    try {
      const result = await api.authSwitchOrg(orgId);
      if (result.type === 'authenticated') {
        handleAuthResult(result);
      } else if (result.org) {
        setOrg(result.org);
        if (result.expires_at) scheduleRefresh(result.expires_at);
      }
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const logout = useCallback(async () => {
    try {
      await api.authLogout();
    } catch {
      // Best-effort
    }
    setUser(null);
    setOrg(null);
    setPendingOrgs(null);
    setPendingId(null);
    setError(null);
    setStatus('unauthenticated');
    if (refreshTimerRef.current) window.clearInterval(refreshTimerRef.current);
  }, []);

  return (
    <AuthContext.Provider value={{ status, user, org, availableOrgs, pendingOrgs, pendingId, error, login, selectOrg, switchOrg, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth must be used within an AuthProvider');
  return context;
}
