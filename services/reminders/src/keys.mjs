import { randomBytes } from "node:crypto";
import { generateVAPIDKeys } from "web-push";
const keys = generateVAPIDKeys();
console.log(`TODO_PUSH_VAPID_PUBLIC_KEY=${keys.publicKey}\nTODO_PUSH_VAPID_PRIVATE_KEY=${keys.privateKey}\nTODO_PUSH_ENROLLMENT_KEY=${randomBytes(32).toString("base64url")}`);
