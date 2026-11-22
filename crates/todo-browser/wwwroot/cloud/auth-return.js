export const ACCOUNT_KEY = "ferroui.todo.microsoft.account";
const RETURN_QUERY_KEY = "ferroui.todo.microsoft.return-query";
const AUTH_PARAMETERS = new Set([
  "code", "state", "session_state", "error", "error_description", "error_uri", "client_info",
  "id_token", "access_token", "refresh_token", "token_type", "expires_in", "scope", "nonce",
  "code_verifier", "code_challenge", "code_challenge_method", "correlation_id", "client-request-id",
]);

export function cleanQuery(query) {
  const values = new URLSearchParams(query);
  for (const key of [...values.keys()]) if (AUTH_PARAMETERS.has(key.toLowerCase())) values.delete(key);
  const result = values.toString();
  return result ? `?${result}` : "";
}

export function hasAuthResponse(address = location) {
  return [address.search, address.hash?.replace(/^#/, "")].some(value => {
    const parameters = new URLSearchParams(value);
    return parameters.has("code") || parameters.has("error") || parameters.has("id_token") || parameters.has("access_token");
  });
}

export function rememberReturn() { sessionStorage.setItem(RETURN_QUERY_KEY, cleanQuery(location.search)); }

export function finishReturn() {
  const saved = sessionStorage.getItem(RETURN_QUERY_KEY);
  // Decode and re-encode even saved data: this value is a query, never a redirect destination.
  const query = cleanQuery(saved ?? location.search);
  history.replaceState(null, "", `${location.pathname}${query}`);
  sessionStorage.removeItem(RETURN_QUERY_KEY);
}

export function forgetReturn() { sessionStorage.removeItem(RETURN_QUERY_KEY); }
