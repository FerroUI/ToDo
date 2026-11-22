export class W {
    b;
    v;
    pos = 0;
    constructor(b){
        this.b = b;
        this.v = new DataView(b.buffer, b.byteOffset, b.byteLength);
    }
    seek(p) {
        this.pos = p;
        return this;
    }
    u8(x) {
        this.b[this.pos++] = x & 0xff;
        return this;
    }
    u16(x) {
        this.v.setUint16(this.pos, x & 0xffff, true);
        this.pos += 2;
        return this;
    }
    u32(x) {
        this.v.setUint32(this.pos, x >>> 0, true);
        this.pos += 4;
        return this;
    }
    u64(x) {
        this.v.setBigUint64(this.pos, typeof x === 'bigint' ? x : BigInt(x), true);
        this.pos += 8;
        return this;
    }
    bytes(x) {
        this.b.set(x, this.pos);
        this.pos += x.length;
        return this;
    }
}
export const u32le = (x)=>{
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, x >>> 0, true);
    return b;
};
export const i32le = (x)=>{
    const b = new Uint8Array(4);
    new DataView(b.buffer).setInt32(0, x | 0, true);
    return b;
};
export const u64le = (x)=>{
    const b = new Uint8Array(8);
    new DataView(b.buffer).setBigInt64(0, x, true);
    return b;
};
export function utf16(s) {
    const b = new Uint8Array(s.length * 2);
    for(let i = 0; i < s.length; i++){
        const c = s.charCodeAt(i);
        b[i * 2] = c & 0xff;
        b[i * 2 + 1] = c >> 8;
    }
    return b;
}
export function fileTimeDate(d) {
    const t = d instanceof Date ? d.getTime() : NaN;
    return t >= -11644473600000 && t <= 910692730085477;
}
export function fileTime(d) {
    return (BigInt(d.getTime()) + 11644473600000n) * 10000n;
}
const TABLE = (()=>{
    const t = new Uint32Array(256);
    for(let i = 0; i < 256; i++){
        let c = i;
        for(let k = 0; k < 8; k++)c = c & 1 ? 0xedb88320 ^ c >>> 1 : c >>> 1;
        t[i] = c >>> 0;
    }
    return t;
})();
export function crc(data, seed = 0) {
    let c = seed >>> 0;
    for(let i = 0; i < data.length; i++)c = TABLE[(c ^ data[i]) & 0xff] ^ c >>> 8;
    return c >>> 0;
}
export function signature(ib, bid) {
    const x = BigInt(ib) ^ BigInt(bid);
    return Number((x >> 16n ^ x) & 0xffffn);
}
export function guidBytes(g) {
    const h = g.replace(/[{}-]/g, '');
    const raw = new Uint8Array(16);
    for(let i = 0; i < 16; i++)raw[i] = parseInt(h.slice(i * 2, i * 2 + 2), 16);
    const order = [
        3,
        2,
        1,
        0,
        5,
        4,
        7,
        6,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15
    ];
    return Uint8Array.from(order, (i)=>raw[i]);
}
let random = (n)=>{
    const c = globalThis.crypto;
    if (c?.getRandomValues) return c.getRandomValues(new Uint8Array(n));
    return Uint8Array.from({
        length: n
    }, ()=>Math.floor(Math.random() * 256));
};
export function randomBytes(n) {
    return random(n);
}
export function setRandom(f) {
    random = f;
}
const CHUNK = 4 << 20;
export class MemorySink {
    chunks = [];
    position = 0;
    append(data) {
        let p = 0;
        while(p < data.length){
            const at = this.position % CHUNK;
            if (at === 0) this.chunks.push(new Uint8Array(CHUNK));
            const n = Math.min(CHUNK - at, data.length - p);
            this.chunks[this.chunks.length - 1].set(data.subarray(p, p + n), at);
            p += n;
            this.position += n;
        }
    }
    appendZeros(n) {
        while(n > 0){
            const at = this.position % CHUNK;
            if (at === 0) this.chunks.push(new Uint8Array(CHUNK));
            const k = Math.min(CHUNK - at, n);
            this.position += k;
            n -= k;
        }
    }
    patch(offset, data) {
        if (offset < 0 || offset + data.length > this.position) throw new Error('patch outside written data');
        let p = 0;
        while(p < data.length){
            const c = Math.floor((offset + p) / CHUNK);
            const at = (offset + p) % CHUNK;
            const n = Math.min(CHUNK - at, data.length - p);
            this.chunks[c].set(data.subarray(p, p + n), at);
            p += n;
        }
    }
    async drain() {}
    blob() {
        const parts = this.chunks.map((c, i)=>i === this.chunks.length - 1 ? c.subarray(0, this.position - i * CHUNK || CHUNK) : c);
        return new Blob(parts, {
            type: 'application/vnd.ms-outlook'
        });
    }
}
export class FileSink {
    out;
    buf = new Uint8Array(CHUNK);
    used = 0;
    base = 0;
    queue = Promise.resolve();
    failure = null;
    constructor(out){
        this.out = out;
    }
    get position() {
        return this.base + this.used;
    }
    write(position, data) {
        const copy = data.slice();
        this.queue = this.queue.then(()=>this.out.write({
                type: 'write',
                position,
                data: copy
            })).catch((e)=>{
            this.failure ??= e;
        });
    }
    flushBuffer() {
        if (!this.used) return;
        this.write(this.base, this.buf.subarray(0, this.used));
        this.base += this.used;
        this.used = 0;
    }
    append(data) {
        let p = 0;
        while(p < data.length){
            const n = Math.min(CHUNK - this.used, data.length - p);
            this.buf.set(data.subarray(p, p + n), this.used);
            this.used += n;
            p += n;
            if (this.used === CHUNK) this.flushBuffer();
        }
    }
    appendZeros(n) {
        while(n > 0){
            const k = Math.min(CHUNK - this.used, n);
            this.buf.fill(0, this.used, this.used + k);
            this.used += k;
            n -= k;
            if (this.used === CHUNK) this.flushBuffer();
        }
    }
    patch(offset, data) {
        const inBuf = Math.max(offset, this.base);
        if (inBuf < offset + data.length) this.buf.set(data.subarray(inBuf - offset), inBuf - this.base);
        if (offset < this.base) this.write(offset, data.subarray(0, Math.min(data.length, this.base - offset)));
    }
    async drain() {
        await this.queue;
        if (this.failure) throw this.failure;
    }
    async close() {
        this.flushBuffer();
        await this.drain();
        await this.out.close();
    }
    async abort() {
        await this.queue;
        await this.out.abort();
    }
}
