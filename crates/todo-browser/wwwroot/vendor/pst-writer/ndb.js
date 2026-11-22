// MIT; John McMillan Consulting LLC / PstBuilder contributors. See LICENSE.
import { W, crc, signature } from './bytes.js';
export const HEADER_SIZE = 564;
export const PAGE = 512;
const TRAILER = 16;
export const MAX_BLOCK_DATA = 8192 - 16;
const ALIGN = 64;
const AMAP_DATA = PAGE - TRAILER;
const AMAP_SPAN = AMAP_DATA * 8 * ALIGN;
const FIRST_AMAP = 0x4400;
const AMAPS_PER_PMAP = 8;
const FIRST_FMAP_REGION = 128;
const AMAPS_PER_FMAP = 496;
const FIRST_FPMAP_REGION = 8192;
const AMAPS_PER_FPMAP = 31744;
const MAX_REGIONS = 14000;
export const MAX_FILE = MAX_REGIONS * AMAP_SPAN;
const PT = {
    bbt: 0x80,
    nbt: 0x81,
    fmap: 0x82,
    pmap: 0x83,
    amap: 0x84,
    fpmap: 0x85
};
export const NidType = {
    hid: 0x00,
    internal: 0x01,
    normalFolder: 0x02,
    searchFolder: 0x03,
    normalMessage: 0x04,
    attachment: 0x05,
    assocMessage: 0x08,
    hierarchyTable: 0x0d,
    contentsTable: 0x0e,
    assocContentsTable: 0x0f,
    ltp: 0x1f
};
export const nid = (type, index)=>(index << 5 | type) >>> 0;
export const nidIndex = (n)=>n >>> 5;
export const nidType = (n)=>n & 0x1f;
export class PstTooBig extends Error {
}
export class PstEntryTooBig extends Error {
}
const blockSize = (n)=>Math.ceil((n + TRAILER) / ALIGN) * ALIGN;
const alignUp = (x, a)=>Math.ceil(x / a) * a;
function pageTrailer(buf, ptype, sig, bid) {
    const c = crc(buf.subarray(0, AMAP_DATA));
    new W(buf).seek(AMAP_DATA).u8(ptype).u8(ptype).u16(sig).u32(c).u64(bid);
}
function mapPage(ptype, ib, fill) {
    const b = new Uint8Array(PAGE);
    if (typeof fill === 'number') b.fill(fill, 0, AMAP_DATA);
    else b.set(fill.subarray(0, AMAP_DATA));
    pageTrailer(b, ptype, 0, ib);
    return b;
}
function amapPage(ib, allocatedEnd) {
    const b = new Uint8Array(PAGE);
    const units = Math.min(allocatedEnd - ib, AMAP_SPAN) / ALIGN;
    const full = Math.floor(units / 8);
    b.fill(0xff, 0, full);
    const rem = units % 8;
    if (rem) b[full] = 0xff00 >> rem & 0xff;
    pageTrailer(b, PT.amap, 0, ib);
    return b;
}
export class NdbWriter {
    out;
    bbt = [];
    nodes = [];
    maxIndex = new Map();
    nextBlock = 4;
    nextPage = 4;
    regionStart = FIRST_AMAP;
    region = 0;
    curAMap = FIRST_AMAP;
    amapFree = 0;
    lastAMap = FIRST_AMAP;
    fmaps = [];
    regionFree = [];
    constructor(out){
        this.out = out;
        out.appendZeros(HEADER_SIZE);
        this.padTo(FIRST_AMAP);
        this.startRegion();
    }
    get size() {
        return this.out.position;
    }
    startRegion() {
        if (this.region >= MAX_REGIONS) throw new PstTooBig('The PST has reached the largest size one file can have.');
        this.curAMap = this.regionStart;
        this.lastAMap = this.regionStart;
        this.out.appendZeros(PAGE);
        if (this.region % AMAPS_PER_PMAP === 0) this.out.append(mapPage(PT.pmap, this.out.position, 0xff));
        if (this.region >= FIRST_FMAP_REGION && (this.region - FIRST_FMAP_REGION) % AMAPS_PER_FMAP === 0) {
            this.fmaps.push({
                offset: this.out.position,
                cover: this.region
            });
            this.out.append(mapPage(PT.fmap, this.out.position, 0));
        }
        if (this.region >= FIRST_FPMAP_REGION && (this.region - FIRST_FPMAP_REGION) % AMAPS_PER_FPMAP === 0) {
            this.out.append(mapPage(PT.fpmap, this.out.position, 0xff));
        }
    }
    addBlock(data, internal = false, cref = 2) {
        if (data.length > MAX_BLOCK_DATA) throw new Error('block too large');
        const bid = this.nextBlock + (internal ? 2 : 0);
        this.nextBlock += 4;
        const total = blockSize(data.length);
        const ib = this.reserve(total, ALIGN);
        const buf = new Uint8Array(total);
        buf.set(data);
        new W(buf).seek(total - TRAILER).u16(data.length).u16(signature(ib, bid)).u32(crc(data)).u64(bid);
        this.out.append(buf);
        this.bbt.push({
            bid,
            ib,
            cb: data.length,
            cref
        });
        return bid;
    }
    addNode(n, data, sub, parent) {
        this.nodes.push({
            nid: n,
            data,
            sub,
            parent
        });
        const t = nidType(n);
        const i = nidIndex(n);
        if ((this.maxIndex.get(t) ?? -1) < i) this.maxIndex.set(t, i);
    }
    finish() {
        const bbt = this.buildTree(PT.bbt, [
            ...this.bbt
        ].sort((a, b)=>a.bid - b.bid), 24, 20);
        const nbt = this.buildTree(PT.nbt, [
            ...this.nodes
        ].sort((a, b)=>a.nid - b.nid), 32, 15);
        for (const p of bbt.all)this.streamPage(p);
        for (const p of nbt.all)this.streamPage(p);
        this.patchAMap(this.curAMap, this.out.position);
        this.patchFMaps();
        const eof = this.regionStart + AMAP_SPAN;
        this.padTo(eof);
        this.out.patch(0, this.header(eof, nbt.root, bbt.root));
    }
    buildTree(ptype, entries, cbEnt, max) {
        const all = [];
        let level = [];
        const isNbt = ptype === PT.nbt;
        for(let i = 0; i < entries.length || i === 0; i += max){
            const chunk = entries.slice(i, i + max);
            const page = {
                ptype,
                level: 0,
                bid: this.nextPage++,
                cbEnt,
                cEntMax: max,
                ib: -1,
                firstKey: chunk.length ? isNbt ? chunk[0].nid : chunk[0].bid : 0
            };
            if (isNbt) page.nbt = chunk;
            else page.bbt = chunk;
            level.push(page);
            all.push(page);
            if (!entries.length) break;
        }
        while(level.length > 1){
            const parents = [];
            for(let i = 0; i < level.length; i += 20){
                const chunk = level.slice(i, i + 20);
                const page = {
                    ptype,
                    level: chunk[0].level + 1,
                    bid: this.nextPage++,
                    cbEnt: 24,
                    cEntMax: 20,
                    ib: -1,
                    firstKey: chunk[0].firstKey,
                    children: chunk
                };
                parents.push(page);
                all.push(page);
            }
            level = parents;
        }
        return {
            root: level[0],
            all
        };
    }
    streamPage(p) {
        p.ib = this.reserve(PAGE, PAGE);
        const b = new Uint8Array(PAGE);
        const w = new W(b);
        let count = 0;
        if (p.children) {
            for (const c of p.children)w.u64(c.firstKey).u64(c.bid).u64(c.ib);
            count = p.children.length;
        } else if (p.nbt) {
            for (const e of p.nbt)w.u64(e.nid).u64(e.data).u64(e.sub).u32(e.parent).u32(0);
            count = p.nbt.length;
        } else if (p.bbt) {
            for (const e of p.bbt)w.u64(e.bid).u64(e.ib).u16(e.cb).u16(e.cref).u32(0);
            count = p.bbt.length;
        }
        w.seek(488).u8(count).u8(p.cEntMax).u8(p.cbEnt).u8(p.level).u32(0);
        pageTrailer(b, p.ptype, signature(p.ib, p.bid), p.bid);
        this.out.append(b);
    }
    reserve(size, alignment) {
        const end = this.regionStart + AMAP_SPAN;
        let at = alignUp(this.out.position, alignment);
        if (at + size > end) {
            this.patchAMap(this.curAMap, this.out.position);
            this.padTo(end);
            this.regionStart = end;
            this.region++;
            this.startRegion();
            at = alignUp(this.out.position, alignment);
        }
        this.padTo(at);
        return at;
    }
    patchAMap(offset, allocatedEnd) {
        this.out.patch(offset, amapPage(offset, allocatedEnd));
        const free = AMAP_SPAN - Math.min(allocatedEnd - offset, AMAP_SPAN);
        this.amapFree += free;
        this.regionFree.push(Math.min(Math.floor(free / ALIGN), 255));
    }
    patchFMaps() {
        for (const { offset, cover } of this.fmaps){
            const map = new Uint8Array(AMAP_DATA);
            for(let i = 0; i < map.length; i++)map[i] = this.regionFree[cover + i] ?? 0;
            this.out.patch(offset, mapPage(PT.fmap, offset, map));
        }
    }
    padTo(target) {
        const n = target - this.out.position;
        if (n < 0) throw new Error('layout overran a boundary');
        if (n) this.out.appendZeros(n);
    }
    header(eof, nbt, bbt) {
        const rgnid = new Array(32).fill(0x400);
        rgnid[NidType.searchFolder] = 0x4000;
        rgnid[NidType.normalMessage] = 0x10000;
        rgnid[NidType.assocMessage] = 0x8000;
        for (const [t, i] of this.maxIndex)if (t < 32 && i + 1 > rgnid[t]) rgnid[t] = i + 1;
        const b = new Uint8Array(HEADER_SIZE);
        const w = new W(b);
        w.u32(0x4e444221).u32(0).u16(0x4d53).u16(23).u16(19).u8(1).u8(1).u32(0).u32(0).u64(0).u64(this.nextPage).u32(1);
        for (const r of rgnid)w.u32(r);
        w.u64(0);
        w.u32(0).u64(eof).u64(this.lastAMap).u64(this.amapFree).u64(0).u64(nbt.bid).u64(nbt.ib).u64(bbt.bid).u64(bbt.ib).u8(2).u8(0).u16(0);
        w.u32(0);
        for(let i = 0; i < 256; i++)w.u8(0xff);
        w.u8(0x80).u8(0).u16(0).u64(this.nextBlock).u32(0);
        new W(b).seek(4).u32(crc(b.subarray(8, 8 + 471)));
        new W(b).seek(524).u32(crc(b.subarray(8, 8 + 516)));
        return b;
    }
}
export function dataTree(ndb, data) {
    if (data.length <= MAX_BLOCK_DATA) return ndb.addBlock(data);
    const bids = [];
    const lens = [];
    for(let off = 0; off < data.length; off += MAX_BLOCK_DATA){
        const chunk = data.subarray(off, off + MAX_BLOCK_DATA);
        bids.push(ndb.addBlock(chunk));
        lens.push(chunk.length);
    }
    return xTree(ndb, bids, lens);
}
export function blocksTree(ndb, blocks) {
    if (blocks.length === 1) return ndb.addBlock(blocks[0]);
    return xTree(ndb, blocks.map((b)=>ndb.addBlock(b)), blocks.map((b)=>b.length));
}
export function rowMatrixTree(ndb, rows, rowSize) {
    if (rows.length <= MAX_BLOCK_DATA) return ndb.addBlock(rows);
    const per = Math.floor(MAX_BLOCK_DATA / rowSize) * rowSize;
    const bids = [];
    const lens = [];
    for(let off = 0; off < rows.length; off += per){
        const n = Math.min(per, rows.length - off);
        const final = off + n >= rows.length;
        const chunk = new Uint8Array(final ? n : MAX_BLOCK_DATA);
        chunk.set(rows.subarray(off, off + n));
        bids.push(ndb.addBlock(chunk));
        lens.push(chunk.length);
    }
    return xTree(ndb, bids, lens);
}
const PER_X = (MAX_BLOCK_DATA - 8) / 8;
function xBlock(bids, total, level) {
    const b = new Uint8Array(8 + bids.length * 8);
    const w = new W(b).u8(1).u8(level).u16(bids.length).u32(total);
    for (const x of bids)w.u64(x);
    return b;
}
function xTree(ndb, bids, lens) {
    const total = lens.reduce((a, b)=>a + b, 0);
    if (bids.length <= PER_X) return ndb.addBlock(xBlock(bids, total, 1), true);
    const xs = [];
    for(let i = 0; i < bids.length; i += PER_X){
        const n = Math.min(PER_X, bids.length - i);
        let len = 0;
        for(let k = 0; k < n; k++)len += lens[i + k];
        xs.push(ndb.addBlock(xBlock(bids.slice(i, i + n), len, 1), true));
    }
    if (xs.length > PER_X) throw new PstEntryTooBig('One value is too large for a PST.');
    return ndb.addBlock(xBlock(xs, total, 2), true);
}
const PER_SL = Math.floor((MAX_BLOCK_DATA - 8) / 24);
const PER_SI = Math.floor((MAX_BLOCK_DATA - 8) / 16);
function slBlock(entries) {
    const b = new Uint8Array(8 + entries.length * 24);
    const w = new W(b).u8(2).u8(0).u16(entries.length).u32(0);
    for (const e of entries)w.u64(e.nid).u64(e.data).u64(e.sub);
    return b;
}
export function subnodeTree(ndb, entries) {
    const sorted = [
        ...entries
    ].sort((a, b)=>a.nid - b.nid);
    if (sorted.length <= PER_SL) return ndb.addBlock(slBlock(sorted), true);
    if (sorted.length > PER_SL * PER_SI) throw new PstEntryTooBig('A message has too many attachments for one PST entry.');
    const si = new Uint8Array(8 + Math.ceil(sorted.length / PER_SL) * 16);
    const w = new W(si).u8(2).u8(1).u16(Math.ceil(sorted.length / PER_SL)).u32(0);
    for(let i = 0; i < sorted.length; i += PER_SL){
        const chunk = sorted.slice(i, i + PER_SL);
        w.u64(chunk[0].nid).u64(ndb.addBlock(slBlock(chunk), true));
    }
    return ndb.addBlock(si, true);
}
