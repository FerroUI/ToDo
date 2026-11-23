import test from "node:test";
import assert from "node:assert/strict";
import { GRAPH_APPLICATION_ID, REGISTRATION_SCOPES, registrationSettings, registrationPayload, validateAzureContext } from "./microsoft-registration/payload.mjs";
import { planRegistration } from "./microsoft-registration/azure.mjs";

const TENANT = "11111111-1111-1111-1111-111111111111", CLIENT = "22222222-2222-2222-2222-222222222222", OBJECT = "33333333-3333-3333-3333-333333333333";
const principal = () => ({ appId: GRAPH_APPLICATION_ID, oauth2PermissionScopes: REGISTRATION_SCOPES.map((value, index) => ({ value, isEnabled: true, id: `1000000${index + 1}-1111-1111-1111-111111111111` })) });
const settings = options => registrationSettings({ tenant: TENANT, ...options });
const azureCloud = { name: "AzureCloud", endpoints: { activeDirectory: "https://login.microsoftonline.com/" } };

test("registration declares SPA and native PKCE platforms with dynamically discovered delegated IDs", () => {
  const payload = registrationPayload(settings(), principal());
  assert.equal(payload.signInAudience, "AzureADandPersonalMicrosoftAccount");
  assert.equal(payload.api.requestedAccessTokenVersion, 2);
  assert.deepEqual(payload.spa, { redirectUris: ["https://ferroui.com/ToDo/"] });
  assert.deepEqual(payload.publicClient, { redirectUris: ["http://localhost/"] });
  assert.deepEqual(payload.requiredResourceAccess, [{ resourceAppId: GRAPH_APPLICATION_ID, resourceAccess: principal().oauth2PermissionScopes.map(scope => ({ id: scope.id, type: "Scope" })) }]);
  assert.equal(payload.passwordCredentials, undefined); assert.equal(payload.keyCredentials, undefined);
  const otherIds = principal(); otherIds.oauth2PermissionScopes[0].id = CLIENT;
  assert.equal(registrationPayload(settings(), otherIds).requiredResourceAccess[0].resourceAccess[0].id, CLIENT);
});

test("explicit registration updates preserve unrelated redirects, API configuration and permissions", () => {
  const existing = { spa: { redirectUris: ["https://other.example.test/"] }, publicClient: { redirectUris: ["http://localhost/callback"] },
    api: { knownClientApplications: [CLIENT], requestedAccessTokenVersion: 1 },
    requiredResourceAccess: [{ resourceAppId: CLIENT, resourceAccess: [{ id: OBJECT, type: "Scope" }] }] };
  const payload = registrationPayload(settings(), principal(), existing);
  assert.deepEqual(payload.spa.redirectUris, ["https://other.example.test/", "https://ferroui.com/ToDo/"]);
  assert.deepEqual(payload.publicClient.redirectUris, ["http://localhost/callback", "http://localhost/"]);
  assert.deepEqual(payload.api, { knownClientApplications: [CLIENT], requestedAccessTokenVersion: 2 });
  assert.deepEqual(payload.requiredResourceAccess[0], existing.requiredResourceAccess[0]);
  assert.equal(registrationPayload(settings(), principal(), payload).requiredResourceAccess[1].resourceAccess.length, 5);
});

test("wrong tenant, changed identity endpoints and unsupported cloud combinations stop before requests", () => {
  assert.throws(() => validateAzureContext(settings(), azureCloud, { tenantId: CLIENT }), /different tenant/);
  assert.throws(() => validateAzureContext(settings(), { ...azureCloud, endpoints: { activeDirectory: "https://attacker.test" } }, { tenantId: TENANT }), /official identity/);
  assert.throws(() => settings({ cloud: "usgov", accounts: "both" }), /organizational/);
  assert.throws(() => settings({ cloud: "china" }), /21Vianet/);
  assert.throws(() => settings({ tenant: "common" }), /GUID/);
  const gov = settings({ cloud: "usgovdod" });
  assert.equal(gov.audience, "AzureADMultipleOrgs");
  validateAzureContext(gov, { name: "AzureUSGovernment", endpoints: { activeDirectory: "https://login.microsoftonline.us/" } }, { tenantId: TENANT });
});

test("registration URI validation rejects credential leaks and incorrect desktop callback platforms", () => {
  for (const uri of ["https://user:password@example.test/", "https://example.test/?code=x", "https://example.test/#token", "http://remote.test/", "https://example.test/index.html"]) {
    assert.throws(() => settings({ spaRedirects: [uri] }));
  }
  for (const uri of ["https://example.test/", "http://localhost.attacker.test/", "http://127.0.0.1/", "http://localhost/?code=x"]) {
    assert.throws(() => settings({ nativeRedirects: [uri] }));
  }
  assert.deepEqual(settings({ nativeRedirects: ["http://127.0.0.1:8400/callback"] }).nativeRedirects, ["http://127.0.0.1:8400/callback"]);
});

test("missing or disabled optional scopes are reported instead of substituting guessed permission IDs", () => {
  const disabled = principal(); disabled.oauth2PermissionScopes[2].isEnabled = false;
  assert.throws(() => registrationPayload(settings(), disabled), /MailboxSettings.Read/);
  const invalid = principal(); invalid.appId = CLIENT;
  assert.throws(() => registrationPayload(settings(), invalid), /could not be verified/);
});

test("registration review only reads; explicit apply creates once and returns public configuration", async () => {
  const calls = [];
  const run = async args => {
    calls.push(args);
    if (args[0] === "cloud") return azureCloud;
    if (args[0] === "account") return { tenantId: TENANT };
    const url = new URL(args[args.indexOf("--url") + 1]);
    assert.equal(args[args.indexOf("--resource") + 1], "https://graph.microsoft.com");
    const method = args[args.indexOf("--method") + 1];
    if (method === "get") return url.pathname.endsWith("/servicePrincipals") ? { value: [principal()] } : { value: [] };
    assert.equal(method, "post"); assert.equal(url.pathname, "/v1.0/applications");
    assert.equal(JSON.parse(args[args.indexOf("--body") + 1]).publicClient.redirectUris[0], "http://localhost/");
    return { id: OBJECT, appId: CLIENT };
  };
  const plan = await planRegistration(settings({ displayName: "FerroUI's To Do" }), run);
  assert.equal(plan.review.operation, "create"); assert.equal(calls.filter(args => args.includes("post")).length, 0);
  const result = await plan.apply();
  assert.equal(calls.filter(args => args.includes("post")).length, 1);
  assert.deepEqual(result.configuration, { clientId: CLIENT, tenant: "common", redirectUri: "https://ferroui.com/ToDo/", cloud: "global" });
  assert.equal(result.repositoryVariables.MICROSOFT_CLOUD, "global");
  assert.equal(calls.some(args => args.includes("login") || args.some(value => value.includes("oauth2PermissionGrants"))), false);
});

test("an existing display-name match is never adopted implicitly", async () => {
  const run = async args => {
    if (args[0] === "cloud") return azureCloud;
    if (args[0] === "account") return { tenantId: TENANT };
    const url = new URL(args[args.indexOf("--url") + 1]);
    return url.pathname.endsWith("/servicePrincipals") ? { value: [principal()] } : { value: [{ appId: CLIENT, displayName: "FerroUI To Do" }] };
  };
  await assert.rejects(planRegistration(settings(), run), /explicit --application-id/);
});
