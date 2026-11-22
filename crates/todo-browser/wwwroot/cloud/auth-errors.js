const INTERACTION = new Set(["interaction_required", "consent_required", "login_required", "no_tokens_found", "no_account_error", "token_refresh_required", "refresh_token_expired"]);
const MESSAGES = {
  user_cancelled: "Microsoft sign-in was cancelled. Your local tasks are unchanged.",
  access_denied: "Microsoft sign-in or consent was declined. You can keep using local tasks or sign in again.",
  interaction_in_progress: "Microsoft sign-in is already in progress. Complete it before starting another sign-in.",
  invalid_client: "Microsoft did not accept this client ID. Check the app registration and supported account types.",
  unauthorized_client: "This Microsoft app registration does not allow this account. Check its supported account types and tenant.",
  invalid_resource: "The Microsoft registration must allow delegated Microsoft Graph Tasks.ReadWrite and User.Read permissions.",
  invalid_scope: "The Microsoft registration must allow delegated Microsoft Graph Tasks.ReadWrite and User.Read permissions.",
  redirect_uri_mismatch: "The Microsoft registration must contain this app's exact address as a Single-page application redirect URI.",
  post_request_failed: "Microsoft sign-in could not reach the token service. Check your connection and the registration's Single-page application redirect URI.",
  get_request_failed: "Microsoft sign-in could not reach the identity service. Check your connection and try again.",
  no_network_connectivity: "Microsoft sign-in is unavailable while offline. Your local tasks remain available.",
  library_load_failed: "Could not load Microsoft sign-in. Try again when online.",
  account_changed: "The Microsoft account changed while a request was pending. Sync again with the connected account.",
  national_cloud_personal_account: "National Microsoft clouds support work or school accounts only. Connect personal Microsoft accounts through a global deployment.",
};

export function authenticationError(error, operation = "sign-in") {
  const code = error?.errorCode || error?.code || "authentication_failed";
  const requiresInteraction = INTERACTION.has(code) || error?.name === "InteractionRequiredAuthError" || /AADSTS65001/.test(error?.message || "");
  let message = requiresInteraction ? "Your Microsoft session needs sign-in again. Choose Reconnect to continue." : MESSAGES[code];
  if (!message && /AADSTS50011/.test(error?.message || "")) message = MESSAGES.redirect_uri_mismatch;
  if (/AADSTS65001/.test(error?.message || "")) message = "Microsoft requires consent for this account. Choose Reconnect; your organization may require administrator approval.";
  const result = new Error(message || `Microsoft ${operation} could not complete. Try again; your local tasks are unchanged.`);
  result.code = code;
  result.requiresInteraction = requiresInteraction;
  return result;
}
