import { request as httpsRequest } from "node:https";
import { generateRequestDetails } from "web-push";

/** The pinned library owns all encryption and VAPID signing. The small HTTP
 * boundary adds an absolute deadline (including DNS), an explicit safe agent,
 * no redirects and no unbounded provider response body. */
export function sendEncrypted(subscription, payload, options, request = httpsRequest) {
  const details = generateRequestDetails(subscription, payload, options);
  return new Promise((resolve, reject) => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), Math.max(1, Math.min(options.timeout, options.TTL * 1000)));
    const finish = (error, value) => { clearTimeout(timer); if (error) reject(error); else resolve(value); };
    try {
      const outgoing = request(new URL(details.endpoint), { method: details.method, headers: details.headers,
        agent: options.agent, signal: controller.signal, maxHeaderSize: 8192 }, response => {
        const statusCode = response.statusCode;
        // Successful push acceptance is communicated by the HTTP status. Error
        // bodies can echo endpoints; consume neither them nor arbitrary streams.
        const headers = { "retry-after": String(response.headers["retry-after"] || "").slice(0, 128) };
        response.destroy();
        if (statusCode >= 200 && statusCode < 300) finish(null, { statusCode });
        else finish(Object.assign(new Error(`Push provider returned HTTP ${statusCode}.`), { statusCode, headers }));
      });
      outgoing.once("error", error => finish(error));
      outgoing.end(details.body);
    } catch (error) { finish(error); }
  });
}
