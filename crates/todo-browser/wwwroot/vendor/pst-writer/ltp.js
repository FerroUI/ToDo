import { W, i32le, u64le, utf16 } from './bytes.js';
import { MAX_BLOCK_DATA, NidType, nid } from './ndb.js';
export const PT = {
    int16: 0x0002,
    int32: 0x0003,
    double: 0x0005,
    bool: 0x000b,
    int64: 0x0014,
    string: 0x001f,
    time: 0x0040,
    guid: 0x0048,
    binary: 0x0102,
    multiInt32: 0x1003
};
export function fixedSize(type) {
    switch(type){
        case PT.int16:
            return 2;
        case PT.int32:
        case 0x0004:
        case 0x000a:
            return 4;
        case PT.bool:
            return 1;
        case PT.double:
        case 0x0006:
        case 0x0007:
        case PT.int64:
        case PT.time:
            return 8;
        case PT.guid:
            return 16;
        default:
            return -1;
    }
}
const inlineInPc = (type)=>{
    const s = fixedSize(type);
    return s >= 0 && s <= 4;
};
export const prop = {
    int32: (id, v)=>({
            id,
            type: PT.int32,
            data: i32le(v)
        }),
    bool: (id, v)=>({
            id,
            type: PT.bool,
            data: Uint8Array.of(v ? 1 : 0)
        }),
    string: (id, v)=>({
            id,
            type: PT.string,
            data: utf16(v ?? '')
        }),
    binary: (id, v)=>({
            id,
            type: PT.binary,
            data: v
        }),
    time: (id, v)=>({
            id,
            type: PT.time,
            data: u64le(v)
        })
};
export const MAX_ITEM = 3580;
const HN_SIG = 0xec;
export const SIG_PC = 0xbc;
export const SIG_TC = 0x7c;
const hid = (index, block)=>(block << 16 | index << 5) >>> 0;
export class Heap {
    clientSig;
    blocks = [
        {
            items: [],
            bytes: 0
        }
    ];
    userRoot = 0;
    constructor(clientSig){
        this.clientSig = clientSig;
    }
    static header(block) {
        if (block === 0) return 12;
        if (block >= 8 && (block - 8) % 128 === 0) return 66;
        return 2;
    }
    add(item) {
        if (!item.length || item.length > MAX_ITEM) throw new Error('heap item size');
        let bi = this.blocks.length - 1;
        const b = this.blocks[bi];
        const count = b.items.length + 1;
        const total = Heap.header(bi) + b.bytes + item.length + 1 + 4 + 2 * (count + 1) + 6;
        if (total > MAX_BLOCK_DATA) {
            this.blocks.push({
                items: [],
                bytes: 0
            });
            bi++;
        }
        const block = this.blocks[bi];
        block.items.push(item);
        block.bytes += item.length;
        return hid(block.items.length, bi);
    }
    set(h, item) {
        const block = this.blocks[h >>> 16];
        const i = (h >>> 5 & 0x7ff) - 1;
        if (block.items[i].length !== item.length) throw new Error('heap item size changed');
        block.items[i] = item;
    }
    build() {
        return this.blocks.map((_, i)=>this.buildBlock(i, i === this.blocks.length - 1));
    }
    buildBlock(bi, final) {
        const block = this.blocks[bi];
        const header = Heap.header(bi);
        const real = block.items.length;
        let end = header;
        for (const it of block.items)end += it.length;
        let filler = 0;
        if (!final) {
            const target = MAX_BLOCK_DATA - (4 + 2 * (real + 2));
            if (target - end >= 1) filler = target - end;
        }
        const cAlloc = real + (filler ? 1 : 0);
        const ib = [];
        let cur = header;
        for (const it of block.items){
            ib.push(cur);
            cur += it.length;
        }
        if (filler) {
            ib.push(cur);
            cur += filler;
        }
        ib.push(cur);
        const ibHnpm = cur + cur % 2;
        const buf = new Uint8Array(ibHnpm + 4 + 2 * (cAlloc + 1));
        const w = new W(buf).u16(ibHnpm);
        if (bi === 0) w.u8(HN_SIG).u8(this.clientSig).u32(this.userRoot).u32(0);
        for(let i = 0; i < real; i++)buf.set(block.items[i], ib[i]);
        w.seek(ibHnpm).u16(cAlloc).u16(0);
        for (const x of ib)w.u16(x);
        return buf;
    }
}
function bth(heap, records, cbKey, cbEnt) {
    if (!records.length) return {
        root: 0,
        levels: 0
    };
    const leafSize = cbKey + cbEnt;
    const idxSize = cbKey + 4;
    let level = pack(heap, records, leafSize, cbKey, Math.floor(MAX_ITEM / leafSize));
    let levels = 0;
    while(level.length > 1){
        const idx = level.map(({ key, h })=>{
            const r = new Uint8Array(idxSize);
            r.set(key);
            new W(r).seek(cbKey).u32(h);
            return r;
        });
        level = pack(heap, idx, idxSize, cbKey, Math.floor(MAX_ITEM / idxSize));
        levels++;
    }
    return {
        root: level[0].h,
        levels
    };
}
function pack(heap, records, size, cbKey, max) {
    const out = [];
    for(let i = 0; i < records.length; i += max){
        const n = Math.min(max, records.length - i);
        const item = new Uint8Array(n * size);
        for(let k = 0; k < n; k++)item.set(records[i + k].subarray(0, size), k * size);
        out.push({
            key: records[i].slice(0, cbKey),
            h: heap.add(item)
        });
    }
    return out;
}
function bthHeader(cbKey, cbEnt, levels, root) {
    const b = new Uint8Array(8);
    new W(b).u8(0xb5).u8(cbKey).u8(cbEnt).u8(levels).u32(root);
    return b;
}
export function propertyContext(props) {
    const byId = new Map();
    for (const p of props)byId.set(p.id, p);
    const sorted = [
        ...byId.values()
    ].sort((a, b)=>a.id - b.id);
    const heap = new Heap(SIG_PC);
    const header = heap.add(new Uint8Array(8));
    const subnodes = [];
    const hnid = new Map();
    let nextSub = 1;
    for (const p of sorted){
        if (inlineInPc(p.type)) continue;
        if (!p.data.length) hnid.set(p.id, 0);
        else if (p.data.length > MAX_ITEM) {
            const n = nid(NidType.ltp, nextSub++);
            subnodes.push({
                nid: n,
                data: p.data
            });
            hnid.set(p.id, n);
        } else hnid.set(p.id, heap.add(p.data));
    }
    const records = sorted.map((p)=>{
        const r = new Uint8Array(8);
        const w = new W(r).u16(p.id).u16(p.type);
        if (inlineInPc(p.type)) {
            const slot = new Uint8Array(4);
            slot.set(p.data.subarray(0, 4));
            w.bytes(slot);
        } else w.u32(hnid.get(p.id));
        return r;
    });
    const { root, levels } = bth(heap, records, 2, 6);
    heap.set(header, bthHeader(2, 6, levels, root));
    heap.userRoot = header;
    return {
        blocks: heap.build(),
        subnodes
    };
}
export const tag = (id, type)=>(id << 16 | type) >>> 0;
const ROW_ID = 0x67f20003;
const ROW_VER = 0x67f30003;
export class TableContext {
    cols = [
        {
            tag: ROW_ID,
            type: PT.int32,
            cb: 4,
            bit: 0,
            ib: 0
        },
        {
            tag: ROW_VER,
            type: PT.int32,
            cb: 4,
            bit: 1,
            ib: 4
        }
    ];
    rows = [];
    column(id, type) {
        const s = fixedSize(type);
        this.cols.push({
            tag: tag(id, type),
            type,
            cb: s >= 1 && s <= 8 ? s : 4,
            bit: this.cols.length,
            ib: 0
        });
        return this;
    }
    row(id, cells) {
        this.rows.push({
            id,
            index: this.rows.length,
            cells: new Map(cells)
        });
        return this;
    }
    build() {
        let cur = 8;
        const others = this.cols.slice(2);
        for (const size of [
            8,
            4
        ])for (const c of others.filter((c)=>c.cb === size))c.ib = cur, cur += size;
        const end4 = cur;
        for (const c of others.filter((c)=>c.cb === 2))c.ib = cur, cur += 2;
        const end2 = cur;
        for (const c of others.filter((c)=>c.cb === 1))c.ib = cur, cur += 1;
        const end1 = cur;
        const ceb = Math.ceil(this.cols.length / 8);
        const rgib = [
            end4,
            end2,
            end1,
            end1 + ceb
        ];
        const rowSize = rgib[3];
        const heap = new Heap(SIG_TC);
        const bthHid = heap.add(new Uint8Array(8));
        const subnodes = [];
        let nextSub = 1;
        const cellHnid = new Map();
        for (const r of this.rows){
            for (const c of this.cols){
                const s = fixedSize(c.type);
                if (s >= 0 && s <= 8) continue;
                const data = r.cells.get(c.tag);
                if (!data?.length) continue;
                let h;
                if (data.length > MAX_ITEM) {
                    h = nid(NidType.ltp, nextSub++);
                    subnodes.push({
                        nid: h,
                        data
                    });
                } else h = heap.add(data);
                cellHnid.set(`${r.index}:${c.tag}`, h);
            }
        }
        let hnidRows = 0;
        let rowMatrix;
        if (this.rows.length) {
            const m = new Uint8Array(rowSize * this.rows.length);
            for (const r of this.rows){
                const base = r.index * rowSize;
                const w = new W(m.subarray(base, base + rowSize));
                w.u32(r.id).u32(r.index);
                const cebOff = rowSize - ceb;
                for (const c of this.cols){
                    const present = c.bit <= 1 || r.cells.has(c.tag);
                    if (c.bit > 1 && present) {
                        const s = fixedSize(c.type);
                        if (s >= 0 && s <= 8) {
                            const d = r.cells.get(c.tag) ?? new Uint8Array(0);
                            m.set(d.subarray(0, c.cb), base + c.ib);
                        } else new W(m).seek(base + c.ib).u32(cellHnid.get(`${r.index}:${c.tag}`) ?? 0);
                    }
                    if (present) m[base + cebOff + (c.bit >> 3)] |= 1 << 7 - c.bit % 8;
                }
            }
            if (m.length <= MAX_ITEM) hnidRows = heap.add(m);
            else {
                const n = nid(NidType.ltp, nextSub++);
                rowMatrix = {
                    nid: n,
                    rows: m,
                    rowSize
                };
                hnidRows = n;
            }
        }
        const records = [
            ...this.rows
        ].sort((a, b)=>a.id - b.id).map((r)=>{
            const rec = new Uint8Array(8);
            new W(rec).u32(r.id).u32(r.index);
            return rec;
        });
        const { root, levels } = bth(heap, records, 4, 4);
        heap.set(bthHid, bthHeader(4, 4, levels, root));
        const sortedCols = [
            ...this.cols
        ].sort((a, b)=>a.tag - b.tag);
        const info = new Uint8Array(22 + sortedCols.length * 8);
        const w = new W(info).u8(0x7c).u8(this.cols.length);
        for (const x of rgib)w.u16(x);
        w.u32(bthHid).u32(hnidRows).u32(0);
        for (const c of sortedCols)w.u32(c.tag).u16(c.ib).u8(c.cb).u8(c.bit);
        heap.userRoot = heap.add(info);
        return {
            blocks: heap.build(),
            subnodes,
            rowMatrix
        };
    }
}
