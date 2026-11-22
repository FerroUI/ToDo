// MIT; FerroUI task extension to pst-writer. MS-PST section 2.4.7.
import { crc, guidBytes, utf16 } from "./bytes.js";
import { prop } from "./ltp.js";

export const PUBLIC_STRINGS = "00020329-0000-0000-c000-000000000046";
export const COMMON = "00062008-0000-0000-c000-000000000046";
export const TASK = "00062003-0000-0000-c000-000000000046";

export function nameMap(additional) {
  const definitions = [{guid: PUBLIC_STRINGS, name: "Keywords"}, {guid: COMMON, lid: 0x8552}, ...additional];
  if (definitions.length > 32768) throw new Error("Too many PST named properties.");
  const guids = [], strings = [], entries = [], hashes = [], seen = new Set();
  let stringBytes = 0;
  for (const definition of definitions) {
    const guid = definition.guid.toLowerCase(), isName = typeof definition.name === "string";
    const identity = JSON.stringify([guid, isName ? definition.name : definition.lid]);
    if (seen.has(identity)) throw new Error("Duplicate PST named property.");
    seen.add(identity);
    let guidIndex = guid === PUBLIC_STRINGS ? 2 : guids.indexOf(guid) + 3;
    if (guid !== PUBLIC_STRINGS && guidIndex === 2) { guids.push(guid); guidIndex = guids.length + 2; }
    let value, hash;
    if (isName) {
      const name = utf16(definition.name), padded = Math.ceil((4 + name.length) / 4) * 4;
      const entry = new Uint8Array(padded);
      new DataView(entry.buffer).setUint32(0, name.length, true); entry.set(name, 4);
      value = stringBytes; stringBytes += padded; strings.push(entry); hash = crc(name);
    } else {
      if (!Number.isInteger(definition.lid) || definition.lid < 0 || definition.lid > 0xffffffff) throw new Error("Invalid PST named property LID.");
      value = definition.lid; hash = value;
    }
    entries.push([value, (guidIndex << 1) | Number(isName)]); hashes.push(hash);
  }
  const entry = new Uint8Array(8 * entries.length), ev = new DataView(entry.buffer), buckets = new Map();
  entries.forEach(([value, guid], index) => {
    ev.setUint32(index*8,value,true); ev.setUint16(index*8+4,guid,true); ev.setUint16(index*8+6,index,true);
    const bucket = ((hashes[index] ^ guid) >>> 0) % 251;
    buckets.set(bucket,[...(buckets.get(bucket)||[]),index]);
  });
  const stringStream = new Uint8Array(stringBytes), guidStream = new Uint8Array(16 * guids.length);
  let offset = 0; for (const part of strings) { stringStream.set(part,offset); offset += part.length; }
  guids.forEach((guid,index) => guidStream.set(guidBytes(guid),index*16));
  const properties = [prop.int32(1,251),prop.binary(2,guidStream),prop.binary(3,entry),prop.binary(4,stringStream)];
  for (const [bucket,indices] of buckets) {
    const bytes = new Uint8Array(8*indices.length), view = new DataView(bytes.buffer);
    indices.forEach((index,position) => {view.setUint32(position*8,hashes[index],true);view.setUint16(position*8+4,entries[index][1],true);view.setUint16(position*8+6,index,true);});
    properties.push(prop.binary(0x1000+bucket,bytes));
  }
  return properties;
}
