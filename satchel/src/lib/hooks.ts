import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { api, CommandError, queryKeys } from "./api";

export function useStatus() {
  return useQuery({ queryKey: queryKeys.status, queryFn: api.status });
}

export function useUsage() {
  return useQuery({ queryKey: queryKeys.usage, queryFn: api.usage });
}

/** The test call is billed, so refresh the usage counter whatever the outcome. */
export function useConnectionTest() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: api.testConnection,
    onSettled: () => queryClient.invalidateQueries({ queryKey: queryKeys.usage }),
  });
}

/** What to do next after a failed Claude call, by error kind. */
export function errorHint(error: unknown): string | null {
  if (!(error instanceof CommandError)) return null;
  switch (error.kind) {
    case "ai.missing_key":
      return null; // the message already says it
    case "ai.auth":
      return "Check that you pasted the whole key, or create a new one in the Claude Console.";
    case "ai.model_not_found":
      return "Check the model name in Settings, or choose one from the list.";
    case "ai.network":
    case "ai.timeout":
      return "Check your internet connection and try again.";
    case "ai.invalid_request":
      return "If this mentions credits or billing, check your plan in the Claude Console.";
    default:
      return null;
  }
}
