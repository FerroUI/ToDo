# PST writer source and task extension

Based on John McMillan Consulting LLC's MIT `pst-writer`, commit
`72ac7a5142a1f1f493ccd5d1e91510b7a8593987`:
https://github.com/johnmcmillan/pst-writer

The upstream implementation is a TypeScript port of MIT PST Builder. Both
copyright notices are retained in `LICENSE`. The five upstream modules were
converted to ES modules with Node 24 `stripTypeScriptTypes` in transform mode,
with type-only imports removed. No npm build dependency or CDN is required.

FerroUI changes add a store-specific named-property map (`names.js`) and
typed message property overrides in `store.js`. `backup/pst-properties.js`
defines the task profile using MS-OXOTASK/MS-OXORMDR properties. Folders use
IPF.Task and messages use IPM.Task. No Outlook runtime is invoked.

Output is a new Unicode PST version23. Production code never modifies an
existing PST. The task fixture is independently read by Microsoft's Rust
outlook-pst1.2.0 implementation. Desktop Outlook acceptance is still a
separate manual check; compatibility with every Outlook version is not claimed.
