// MIT; John McMillan Consulting LLC / PstBuilder contributors. See LICENSE.
// FerroUI extension: typed task properties and store-specific named-property map.
import { crc, fileTime, fileTimeDate, guidBytes, randomBytes, utf16, i32le, u64le } from './bytes.js';
import { PT, TableContext, prop, propertyContext, tag } from './ltp.js';
import { NdbWriter, NidType, blocksTree, dataTree, nid, nidIndex, rowMatrixTree, subnodeTree } from './ndb.js';
import { nameMap } from './names.js';
const T = {
    recordKey: 0x0ff9,
    displayName: 0x3001,
    ipmSubtreeEntryId: 0x35e0,
    wastebasketEntryId: 0x35e3,
    finderEntryId: 0x35e7,
    contentCount: 0x3602,
    contentUnread: 0x3603,
    subfolders: 0x360a,
    containerClass: 0x3613,
    hiddenCount: 0x6635,
    hiddenUnread: 0x6636,
    password: 0x67ff,
    validFolderMask: 0x35df,
    replVersionHistory: 0x0e34,
    replFlags: 0x0e38,
    lrNoRestrictions: 0x6633,
    idsToPids: 0x66fa,
    storeVersion: 0x66fc,
    messageStatus: 0x0e17,
    conversationTopic: 0x0070,
    responsibility: 0x0e0f,
    displayType: 0x3900,
    sevenBitName: 0x39ff,
    sendRichInfo: 0x3a40,
    messageClass: 0x001a,
    importance: 0x0017,
    sensitivity: 0x0036,
    subject: 0x0037,
    submitTime: 0x0039,
    sentRepName: 0x0042,
    sentRepEmail: 0x0065,
    senderName: 0x0c1a,
    senderEmail: 0x0c1f,
    displayCc: 0x0e03,
    displayTo: 0x0e04,
    deliveryTime: 0x0e06,
    messageFlags: 0x0e07,
    messageSize: 0x0e08,
    body: 0x1000,
    html: 0x1013,
    nativeBody: 0x1016,
    creationTime: 0x3007,
    lastModTime: 0x3008,
    searchKey: 0x300b,
    attachSize: 0x0e20,
    attachNumber: 0x0e21,
    attachData: 0x3701,
    attachFilename: 0x3704,
    attachMethod: 0x3705,
    attachLongFilename: 0x3707,
    renderingPosition: 0x370b,
    attachMime: 0x370e,
    attachContentId: 0x3712,
    attachFlags: 0x3714,
    recipientType: 0x0c15,
    objectType: 0x0ffe,
    entryId: 0x0fff,
    addressType: 0x3002,
    emailAddress: 0x3003,
    recipientFlags: 0x5ffd,
    transportHeaders: 0x007d,
    internetCodepage: 0x3fde,
    senderAddrType: 0x0c1e,
    sentRepAddrType: 0x0064,
    internetMessageId: 0x1035
};
const RECIPIENT_TABLE = 0x692;
const ATTACHMENT_TABLE = 0x671;
const MESSAGE_SIZE_BASE = 2234;
const cell = (id, type, data)=>[
        tag(id, type),
        data
    ];
function brief(s) {
    if (s.length <= 255) return utf16(s);
    let cut = 255;
    const last = s.charCodeAt(cut - 1);
    if (last >= 0xd800 && last <= 0xdbff) cut--;
    return utf16(s.slice(0, cut));
}
export class PstStore {
    storeName;
    ndb;
    uid = randomBytes(16);
    folders = [];
    byPath = new Map();
    nextFolder = 0x401;
    nextMessage = 0x10000;
    nextRecipientRow = 1;
    top;
    deleted;
    finder;
    searchRoot;
    messages = 0;
    constructor(sink, storeName = 'Personal Folders', namedProperties = []){
        this.storeName = storeName;
        this.namedProperties = namedProperties;
        this.ndb = new NdbWriter(sink);
        const root = this.newFolder('Root', 0x122, 0x122);
        this.top = this.child(root, 'Top of Personal Folders');
        this.deleted = this.child(this.top, 'Deleted Items');
        this.byPath.set('deleted items', this.deleted);
        this.finder = this.child(root, 'Finder');
        this.searchRoot = {
            nid: 0x2223,
            parent: root.nid,
            name: 'Search Root',
            count: 0,
            unread: 0,
            children: [],
            rows: []
        };
        root.children.push(this.searchRoot);
    }
    get size() {
        return this.ndb.size;
    }
    folder(path, containerClass) {
        let cur = this.top;
        let key = '';
        for (const seg of path.split(/[/\\]/).map((s)=>s.trim()).filter(Boolean)){
            key = key ? `${key}/${seg}` : seg;
            const k = key.toLowerCase();
            let next = this.byPath.get(k);
            if (!next) {
                next = this.child(cur, seg);
                this.byPath.set(k, next);
            }
            cur = next;
        }
        if (containerClass && !cur.containerClass) cur.containerClass = containerClass;
        return cur;
    }
    addMessage(folder, m) {
        if (!fileTimeDate(m.received) || !fileTimeDate(m.sent)) {
            const either = fileTimeDate(m.received) ? m.received : fileTimeDate(m.sent) ? m.sent : new Date();
            m = {
                ...m,
                received: fileTimeDate(m.received) ? m.received : either,
                sent: fileTimeDate(m.sent) ? m.sent : either
            };
        }
        const id = nid(NidType.normalMessage, this.nextMessage++);
        const flags = (m.read === false ? 0 : 1) | (m.attachments.length ? 0x10 : 0);
        const size = this.writeMessage(folder, id, m, flags);
        folder.count++;
        if (m.read === false) folder.unread++;
        folder.rows.push({
            id,
            cls: m.messageClass ?? 'IPM.Note',
            subject: m.subject,
            sender: m.senderName,
            to: m.displayTo,
            cc: m.displayCc ?? '',
            flags,
            delivery: fileTime(m.received),
            submit: fileTime(m.sent),
            mod: fileTime(m.received),
            size
        });
        this.messages++;
    }
    finish() {
        for (const f of this.folders)this.writeFolder(f);
        this.writeStore();
        this.writeSkeleton();
        this.writeNameToId();
        this.ndb.finish();
    }
    newFolder(name, n, parent) {
        const f = {
            nid: n,
            parent,
            name,
            count: 0,
            unread: 0,
            children: [],
            rows: []
        };
        this.folders.push(f);
        return f;
    }
    child(parent, name) {
        const f = this.newFolder(name, nid(NidType.normalFolder, this.nextFolder++), parent.nid);
        parent.children.push(f);
        return f;
    }
    stage(c, extra = []) {
        const data = blocksTree(this.ndb, c.blocks);
        const entries = c.subnodes.map((s)=>({
                nid: s.nid,
                data: dataTree(this.ndb, s.data),
                sub: 0
            }));
        if (c.rowMatrix) entries.push({
            nid: c.rowMatrix.nid,
            data: rowMatrixTree(this.ndb, c.rowMatrix.rows, c.rowMatrix.rowSize),
            sub: 0
        });
        entries.push(...extra);
        return {
            data,
            sub: entries.length ? subnodeTree(this.ndb, entries) : 0
        };
    }
    node(n, c, parent = 0) {
        const { data, sub } = this.stage(c);
        this.ndb.addNode(n, data, sub, parent);
    }
    writeFolder(f) {
        const props = [
            prop.string(T.displayName, f.name),
            prop.int32(T.contentCount, f.count),
            prop.int32(T.contentUnread, f.unread),
            prop.bool(T.subfolders, f.children.length > 0),
            prop.int32(T.hiddenCount, 0),
            prop.int32(T.hiddenUnread, 0)
        ];
        if (f.containerClass) props.push(prop.string(T.containerClass, f.containerClass));
        this.node(f.nid, propertyContext(props), f.parent);
        const hier = hierarchyColumns(new TableContext());
        for (const s of f.children){
            const cells = [
                cell(T.displayName, PT.string, brief(s.name)),
                cell(T.contentCount, PT.int32, i32le(s.count)),
                cell(T.contentUnread, PT.int32, i32le(s.unread)),
                cell(T.subfolders, PT.bool, Uint8Array.of(s.children.length ? 1 : 0)),
                cell(T.hiddenCount, PT.int32, i32le(0)),
                cell(T.hiddenUnread, PT.int32, i32le(0))
            ];
            if (s.containerClass) cells.push(cell(T.containerClass, PT.string, brief(s.containerClass)));
            hier.row(s.nid, cells);
        }
        this.node(nid(NidType.hierarchyTable, nidIndex(f.nid)), hier.build());
        const contents = contentsColumns(new TableContext());
        for (const r of f.rows){
            contents.row(r.id, [
                cell(T.messageClass, PT.string, brief(r.cls)),
                cell(T.subject, PT.string, brief(r.subject)),
                cell(T.sentRepName, PT.string, brief(r.sender)),
                cell(T.messageFlags, PT.int32, i32le(r.flags)),
                cell(T.deliveryTime, PT.time, u64le(r.delivery)),
                cell(T.messageSize, PT.int32, i32le(r.size)),
                cell(T.displayTo, PT.string, brief(r.to)),
                cell(T.importance, PT.int32, i32le(1)),
                cell(T.sensitivity, PT.int32, i32le(0)),
                cell(T.messageStatus, PT.int32, i32le(0)),
                cell(T.submitTime, PT.time, u64le(r.submit)),
                cell(T.lastModTime, PT.time, u64le(r.mod)),
                cell(T.conversationTopic, PT.string, brief(r.subject)),
                ...r.cc ? [
                    cell(T.displayCc, PT.string, brief(r.cc))
                ] : []
            ]);
        }
        this.node(nid(NidType.contentsTable, nidIndex(f.nid)), contents.build());
        this.node(nid(NidType.assocContentsTable, nidIndex(f.nid)), faiColumns(new TableContext()).build());
        f.rows = [];
    }
    writeMessage(folder, id, m, flags) {
        const attachProps = m.attachments.map((a, i)=>attachmentProps(a, i));
        const attachSizes = attachProps.map((ps)=>4 + ps.reduce((s, p)=>s + p.data.length, 0));
        const sa = attachSizes.reduce((a, b)=>a + b, 0);
        const sr = m.recipients.reduce((s, r)=>s + recipientRowSize(r), 0);
        const props = [
            prop.string(T.messageClass, m.messageClass ?? 'IPM.Note'),
            prop.int32(T.messageFlags, flags),
            prop.int32(T.importance, 1),
            prop.int32(T.sensitivity, 0),
            prop.int32(T.messageStatus, 0),
            prop.bool(0x0e1b, m.attachments.length > 0),
            prop.string(T.conversationTopic, m.subject),
            prop.string(T.subject, m.subject),
            prop.string(T.body, m.body),
            prop.string(T.displayTo, m.displayTo),
            prop.string(T.senderName, m.senderName),
            prop.string(T.senderEmail, m.senderEmail),
            prop.string(T.sentRepName, m.senderName),
            prop.string(T.sentRepEmail, m.senderEmail),
            prop.string(T.senderAddrType, 'SMTP'),
            prop.string(T.sentRepAddrType, 'SMTP'),
            prop.time(T.deliveryTime, fileTime(m.received)),
            prop.time(T.submitTime, fileTime(m.sent)),
            prop.time(T.creationTime, fileTime(m.received)),
            prop.time(T.lastModTime, fileTime(m.received)),
            prop.binary(T.searchKey, randomBytes(16))
        ];
        if (m.displayCc) props.push(prop.string(T.displayCc, m.displayCc));
        if (m.headers) props.push(prop.string(T.transportHeaders, m.headers));
        if (m.messageId) props.push(prop.string(T.internetMessageId, m.messageId));
        if (m.html) {
            props.push(prop.binary(T.html, new TextEncoder().encode(m.html)), prop.int32(T.internetCodepage, 65001), prop.int32(T.nativeBody, 3));
        } else props.push(prop.int32(T.nativeBody, 1));
        for (const extra of m.extraProperties || []) {
            const previous = props.findIndex(value => value.id === extra.id);
            if (previous >= 0) props[previous] = extra;
            else props.push(extra);
        }
        const size = 4 + props.reduce((s, p)=>s + p.data.length, 0) + sr + sa + MESSAGE_SIZE_BASE;
        props.push(prop.int32(T.messageSize, size));
        const subs = [];
        const recip = this.stage(this.recipientTable(m));
        subs.push({
            nid: RECIPIENT_TABLE,
            data: recip.data,
            sub: recip.sub
        });
        if (m.attachments.length) {
            const table = new TableContext().column(T.attachSize, PT.int32).column(T.attachMethod, PT.int32).column(T.renderingPosition, PT.int32).column(T.attachFilename, PT.string).column(T.attachLongFilename, PT.string).column(T.displayName, PT.string);
            m.attachments.forEach((a, i)=>{
                const an = nid(NidType.attachment, i);
                table.row(an, [
                    cell(T.attachSize, PT.int32, i32le(attachSizes[i])),
                    cell(T.attachMethod, PT.int32, i32le(1)),
                    cell(T.renderingPosition, PT.int32, i32le(-1)),
                    cell(T.attachFilename, PT.string, utf16(a.name)),
                    cell(T.attachLongFilename, PT.string, utf16(a.name)),
                    cell(T.displayName, PT.string, utf16(a.name))
                ]);
                const s = this.stage(propertyContext([
                    ...attachProps[i],
                    prop.int32(T.attachSize, attachSizes[i])
                ]));
                subs.push({
                    nid: an,
                    data: s.data,
                    sub: s.sub
                });
            });
            const t = this.stage(table.build());
            subs.push({
                nid: ATTACHMENT_TABLE,
                data: t.data,
                sub: t.sub
            });
        }
        const { data, sub } = this.stage(propertyContext(props), subs);
        this.ndb.addNode(id, data, sub, folder.nid);
        return size;
    }
    recipientTable(m) {
        const tc = new TableContext().column(T.recipientType, PT.int32).column(T.objectType, PT.int32).column(T.recipientFlags, PT.int32).column(T.displayName, PT.string).column(T.emailAddress, PT.string).column(T.addressType, PT.string).column(T.responsibility, PT.bool).column(T.recordKey, PT.binary).column(T.entryId, PT.binary).column(T.searchKey, PT.binary).column(T.displayType, PT.int32).column(T.sevenBitName, PT.string).column(T.sendRichInfo, PT.bool);
        for (const r of m.recipients){
            tc.row(this.nextRecipientRow++, [
                cell(T.recipientType, PT.int32, i32le(r.type)),
                cell(T.objectType, PT.int32, i32le(6)),
                cell(T.recipientFlags, PT.int32, i32le(1)),
                cell(T.displayName, PT.string, utf16(r.name)),
                cell(T.emailAddress, PT.string, utf16(r.email)),
                cell(T.addressType, PT.string, utf16('SMTP')),
                cell(T.responsibility, PT.bool, Uint8Array.of(0)),
                cell(T.displayType, PT.int32, i32le(0)),
                cell(T.sevenBitName, PT.string, utf16(r.name)),
                cell(T.sendRichInfo, PT.bool, Uint8Array.of(0))
            ]);
        }
        return tc.build();
    }
    entryId(n) {
        const b = new Uint8Array(24);
        b.set(this.uid, 4);
        new DataView(b.buffer).setUint32(20, n, true);
        return b;
    }
    writeStore() {
        this.node(0x21, propertyContext([
            prop.binary(T.recordKey, this.uid),
            prop.string(T.displayName, this.storeName),
            prop.int32(T.password, 0),
            prop.int32(T.validFolderMask, 0x1 | 0x8 | 0x80),
            prop.int32(T.replFlags, 0),
            prop.binary(T.replVersionHistory, new Uint8Array(0)),
            prop.bool(T.lrNoRestrictions, true),
            prop.int32(T.idsToPids, 0),
            prop.int32(T.storeVersion, 0),
            prop.binary(T.ipmSubtreeEntryId, this.entryId(this.top.nid)),
            prop.binary(T.wastebasketEntryId, this.entryId(this.deleted.nid)),
            prop.binary(T.finderEntryId, this.entryId(this.finder.nid))
        ]));
    }
    writeSkeleton() {
        this.node(0x60d, hierarchyColumns(new TableContext()).build());
        this.node(0x60e, contentsColumns(new TableContext()).build());
        this.node(0x60f, faiColumns(new TableContext()).build());
        this.node(0x610, tc(SEARCH_COLUMNS));
        const receive = new TableContext().column(0x001a, PT.string).column(0x6605, PT.int32);
        receive.row(this.top.nid, [
            cell(0x001a, PT.string, new Uint8Array(0)),
            cell(0x6605, PT.int32, i32le(this.top.nid))
        ]);
        this.node(0x62b, receive.build());
        this.node(0x64c, tc([
            [
                0x0039,
                PT.time
            ],
            [
                0x0e10,
                PT.int32
            ],
            [
                0x0e14,
                PT.int32
            ]
        ]));
        this.node(0x671, tc([
            [
                T.attachSize,
                PT.int32
            ],
            [
                T.attachFilename,
                PT.string
            ],
            [
                T.attachMethod,
                PT.int32
            ],
            [
                T.renderingPosition,
                PT.int32
            ]
        ]));
        this.node(0x692, tc([
            [
                T.recipientType,
                PT.int32
            ],
            [
                T.responsibility,
                PT.bool
            ],
            [
                T.recordKey,
                PT.binary
            ],
            [
                T.objectType,
                PT.int32
            ],
            [
                T.entryId,
                PT.binary
            ],
            [
                T.displayName,
                PT.string
            ],
            [
                T.addressType,
                PT.string
            ],
            [
                T.emailAddress,
                PT.string
            ],
            [
                T.searchKey,
                PT.binary
            ],
            [
                T.displayType,
                PT.int32
            ],
            [
                T.sevenBitName,
                PT.string
            ],
            [
                T.sendRichInfo,
                PT.bool
            ]
        ]));
        this.node(0x6b6, tc([
            [
                0x0e33,
                PT.int64
            ],
            [
                0x0e37,
                PT.binary
            ],
            [
                0x0e38,
                PT.int32
            ]
        ]));
        this.node(0x6d7, tc([
            [
                0x001a,
                PT.string
            ],
            [
                0x0e30,
                PT.binary
            ],
            [
                0x0e31,
                PT.binary
            ],
            [
                0x0e33,
                PT.int64
            ],
            [
                0x0e34,
                PT.binary
            ],
            [
                0x0e38,
                PT.int32
            ],
            [
                0x0e3e,
                PT.binary
            ]
        ]));
        this.node(0x6f8, tc([
            [
                0x0e33,
                PT.int64
            ],
            [
                0x3007,
                PT.time
            ]
        ]));
        this.node(this.searchRoot.nid, propertyContext([
            prop.string(T.displayName, this.searchRoot.name),
            prop.int32(T.contentCount, 0),
            prop.int32(T.contentUnread, 0),
            prop.bool(T.subfolders, false),
            prop.int32(T.hiddenCount, 0),
            prop.int32(T.hiddenUnread, 0)
        ]), this.searchRoot.parent);
        this.node(0x2230, tc(SEARCH_COLUMNS));
        this.node(0x2227, propertyContext([
            prop.int32(0x660b, 0)
        ]));
        for (const n of [
            0x1e1,
            0x201,
            0x261,
            0xe41,
            0xec1,
            0xf21,
            0x2226
        ])this.ndb.addNode(n, 0, 0, 0);
    }
    writeNameToId() {
        this.node(0x61, propertyContext(nameMap(this.namedProperties)));
    }
}
function attachmentProps(a, i) {
    const ps = [
        prop.int32(T.attachMethod, 1),
        prop.int32(T.attachNumber, i),
        prop.int32(T.renderingPosition, -1),
        prop.int32(T.objectType, 7),
        prop.string(T.displayName, a.name),
        prop.string(T.attachFilename, a.name),
        prop.string(T.attachLongFilename, a.name),
        prop.binary(T.attachData, a.data)
    ];
    if (a.mime) ps.push(prop.string(T.attachMime, a.mime));
    if (a.contentId) ps.push(prop.string(T.attachContentId, a.contentId));
    if (a.inline) ps.push(prop.int32(T.attachFlags, 0x04));
    return ps;
}
function recipientRowSize(r) {
    return 12 + r.name.length * 2 + r.email.length * 2 + 4 * 2 + 1 + 4 + r.name.length * 2 + 1 + 8;
}
function tc(cols) {
    const t = new TableContext();
    for (const [id, type] of cols)t.column(id, type);
    return t.build();
}
const SEARCH_COLUMNS = [
    [
        0x0017,
        PT.int32
    ],
    [
        0x001a,
        PT.string
    ],
    [
        0x0036,
        PT.int32
    ],
    [
        0x0037,
        PT.string
    ],
    [
        0x0042,
        PT.string
    ],
    [
        0x0057,
        PT.bool
    ],
    [
        0x0058,
        PT.bool
    ],
    [
        0x0e03,
        PT.string
    ],
    [
        0x0e04,
        PT.string
    ],
    [
        0x0e05,
        PT.string
    ],
    [
        0x0e06,
        PT.time
    ],
    [
        0x0e07,
        PT.int32
    ],
    [
        0x0e08,
        PT.int32
    ],
    [
        0x0e17,
        PT.int32
    ],
    [
        0x0e2a,
        PT.bool
    ],
    [
        0x3008,
        PT.time
    ],
    [
        0x67f1,
        PT.int32
    ]
];
function hierarchyColumns(t) {
    return t.column(T.displayName, PT.string).column(T.contentCount, PT.int32).column(T.contentUnread, PT.int32).column(T.subfolders, PT.bool).column(T.containerClass, PT.string).column(0x0e30, PT.binary).column(0x0e33, PT.int64).column(0x0e34, PT.binary).column(0x0e38, PT.int32).column(T.hiddenCount, PT.int32).column(T.hiddenUnread, PT.int32);
}
function contentsColumns(t) {
    return t.column(T.messageClass, PT.string).column(T.subject, PT.string).column(T.sentRepName, PT.string).column(T.messageFlags, PT.int32).column(T.deliveryTime, PT.time).column(T.messageSize, PT.int32).column(T.displayTo, PT.string).column(T.importance, PT.int32).column(T.sensitivity, PT.int32).column(T.submitTime, PT.time).column(0x0057, PT.bool).column(0x0058, PT.bool).column(T.conversationTopic, PT.string).column(0x0071, PT.binary).column(0x0e03, PT.string).column(T.messageStatus, PT.int32).column(0x1097, PT.int32).column(T.lastModTime, PT.time).column(0x3013, PT.binary).column(0x65c6, PT.int32).column(0x0e30, PT.binary).column(0x0e33, PT.int64).column(0x0e34, PT.binary).column(0x0e38, PT.int32).column(0x0e3c, PT.binary).column(0x0e3d, PT.binary);
}
function faiColumns(t) {
    return t.column(T.messageClass, PT.string).column(T.messageFlags, PT.int32).column(T.messageStatus, PT.int32).column(T.displayName, PT.string).column(0x7003, PT.int32).column(0x7004, PT.binary).column(0x7005, PT.binary).column(0x7006, PT.string).column(0x7007, PT.int32).column(0x6800, PT.string).column(0x6803, PT.bool).column(0x6805, PT.multiInt32).column(0x682f, PT.string);
}
