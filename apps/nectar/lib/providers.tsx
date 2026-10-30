"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";
import type { ReactNode } from "react";

import type { Credential } from "./hiveway";

interface Session {
  token: string;
  sessionKey: number;
  credential: Credential | null;
  setToken: (value: string) => void;
  setCredential: (value: Credential | null) => void;
}
const SessionContext = createContext<Session | null>(null);
export const useHiveSession = () => {
  const value = useContext(SessionContext);
  if (!value) {
    throw new Error("Hive providers missing");
  }
  return value;
};
export const Providers = ({ children }: { children: ReactNode }) => {
  const [client, setClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            gcTime: 300_000,
            refetchOnWindowFocus: true,
            retry: false,
            staleTime: 15_000,
          },
        },
      })
  );
  const [storedToken, setStoredToken] = useState("session");
  const [sessionKey, setSessionKey] = useState(0);
  const [credential, setCredential] = useState<Credential | null>(null);
  const setToken = useCallback(
    (value: string) => {
      void client.cancelQueries();
      client.clear();
      setStoredToken(value);
      setCredential(null);
      setSessionKey((key) => key + 1);
    },
    [client]
  );
  const value = useMemo(
    () => ({
      credential,
      sessionKey,
      setCredential,
      setToken,
      token: storedToken,
    }),
    [credential, sessionKey, setToken, storedToken]
  );
  void setClient;
  return (
    <QueryClientProvider client={client}>
      <SessionContext.Provider value={value}>
        {children}
      </SessionContext.Provider>
    </QueryClientProvider>
  );
};
