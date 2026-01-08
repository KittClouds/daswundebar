[Highlighter] Ready for instant decorations (kittcore v0.1.0)
App.tsx:40 Highlighter ready for instant decorations
UnifiedRegistry.ts:96 [CozoUnifiedRegistry] Initializing...
db.ts:64 [CozoDB] Starting initialization...
db.ts:68 [CozoDB] ✅ WASM module loaded
db.ts:72 [CozoDB] ✅ Database instance created
db-client.ts:46 [DBClient] Initializing SQLite worker...
sqlite-worker.ts:1098 [SQLite Worker] Worker ready
init.ts:22 [SQLite Worker] Loading SQLite3 module...
init.ts:34 [SQLite Worker] SQLite3 version: 3.50.4
init.ts:37 [SQLite Worker] Database opened: /canvas.sqlite3
init.ts:225 [SQLite Worker] Running schema migrations...
init.ts:81 [SQLite Worker] Running orphan cleanup...
init.ts:158 [SQLite Worker] No orphaned tables found (nodes_temp_old: false , nodes: true )
init.ts:234 [SQLite Worker] Current schema version: 7, target: 7
init.ts:261 [SQLite Worker] Creating 19 tables...
init.ts:271 [SQLite Worker] Creating 51 indexes...
init.ts:276 [SQLite Worker] Creating 1 virtual tables...
init.ts:281 [SQLite Worker] Creating 3 FTS triggers...
init.ts:286 [SQLite Worker] Creating 3 validation triggers...
init.ts:298 [SQLite Worker] Schema version: 7
init.ts:42 [SQLite Worker] Initialized in 2324.64ms
db-client.ts:76 [DBClient] SQLite ready in 2459.45ms
db.ts:76 [CozoDB] ✅ SQLite persistence layer ready
db.ts:115 [CozoDB] ✅ Hydrated 7 tables, 0 rows from SQLite
db.ts:80 [CozoDB] ✅ Initialization complete
UnifiedRegistry.ts:107 [CozoUnifiedRegistry] Creating schemas...
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema entities created
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema entity_aliases created
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema entity_mentions created
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema entity_metadata created
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema relationships created
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema relationship_provenance created
UnifiedRegistry.ts:131 [CozoUnifiedRegistry] Schema relationship_attributes created
UnifiedRegistry.ts:142 [CozoUnifiedRegistry] Schema creation complete
UnifiedRegistry.ts:103 [CozoUnifiedRegistry] ✅ Initialized
init.ts:136 CozoDB schema initialized: 1.5.0
App.tsx:51 Unified Registry and Layer 2 Schemas initialized
startup.ts:10 [RelationshipSystem] setRelationshipStore called (legacy compatibility)
startup.ts:21 [RelationshipSystem] Initialized Cozo-backed registry. Total relationships: 0
App.tsx:56 SQLite initialized: 6 nodes, 0 embeddings
BlueprintStoreImpl.ts:38 BlueprintStore initialized (in-memory)
index.ts:65 Storage service initialized
App.tsx:60 Storage service initialized
BlueprintStoreImpl.ts:38 BlueprintStore initialized (in-memory)
App.tsx:64 Blueprint store initialized
store.ts:24 [Store] Initializing Jotai store...
notes-async.ts:119 [Atoms] Loading data from database...
notes-async.ts:132 [Atoms] Loaded 1 notes, 5 folders
notes-async.ts:150 [Atoms] ✅ Store hydrated successfully
search.ts:266 [Search] Services initialized
store.ts:37 [Store] ✅ Jotai store initialized
App.tsx:69 Jotai store initialized
BindingEngine.ts:72 [BindingEngine] Initialized with 0 bindings
App.tsx:99 Binding engine initialized
CozoContext.tsx:58 [CozoContext] Initialized with 0 entities
BlueprintHubContext.tsx:92 Blueprint Hub: Refresh called
BlueprintHubContext.tsx:107 Blueprint Hub: Successfully compiled blueprint for version 019b9a24-a7eb-731f-9c56-d6986c4f6970
useEntitySync.ts:148 [useEntitySync] Synced 1 notes, hydrated scanner/highlighter with 8 entities
useEntitySync.ts:161 [useEntitySync] Immediate scan after hydration: 019b9230-29d3-7075-b97d-b785ec62f460
RichEditor.tsx:510 Canvas2D: Multiple readback operations using getImageData are faster with the willReadFrequently attribute set to true. See: https://html.spec.whatwg.org/multipage/canvas.html#concept-canvas-will-read-frequently
(anonymous) @ reactjs-tiptap-editor_emoji.js?v=95e8d831:706
te2 @ reactjs-tiptap-editor_emoji.js?v=95e8d831:686
(anonymous) @ reactjs-tiptap-editor_emoji.js?v=95e8d831:20119
addStorage @ reactjs-tiptap-editor_emoji.js?v=95e8d831:20115
q @ chunk-HYN6MI4U.js?v=95e8d831:4589
get storage @ chunk-HYN6MI4U.js?v=95e8d831:5251
(anonymous) @ chunk-SRZMG2AG.js?v=95e8d831:1954
flattenExtensions @ chunk-SRZMG2AG.js?v=95e8d831:1950
resolveExtensions @ chunk-SRZMG2AG.js?v=95e8d831:2305
ExtensionManager @ chunk-SRZMG2AG.js?v=95e8d831:4008
createExtensionManager @ chunk-SRZMG2AG.js?v=95e8d831:5237
Editor @ chunk-SRZMG2AG.js?v=95e8d831:4975
createEditor @ @tiptap_react.js?v=95e8d831:940
getInitialEditor @ @tiptap_react.js?v=95e8d831:872
_EditorInstanceManager @ @tiptap_react.js?v=95e8d831:847
(anonymous) @ @tiptap_react.js?v=95e8d831:1071
mountState @ chunk-SM3OW2QE.js?v=95e8d831:12005
useState @ chunk-SM3OW2QE.js?v=95e8d831:12545
useState @ chunk-7SNDHR3H.js?v=95e8d831:1066
useEditor @ @tiptap_react.js?v=95e8d831:1071
RichEditor @ RichEditor.tsx:510
renderWithHooks @ chunk-SM3OW2QE.js?v=95e8d831:11548
mountIndeterminateComponent @ chunk-SM3OW2QE.js?v=95e8d831:14926
beginWork @ chunk-SM3OW2QE.js?v=95e8d831:15914
beginWork$1 @ chunk-SM3OW2QE.js?v=95e8d831:19753
performUnitOfWork @ chunk-SM3OW2QE.js?v=95e8d831:19198
workLoopSync @ chunk-SM3OW2QE.js?v=95e8d831:19137
renderRootSync @ chunk-SM3OW2QE.js?v=95e8d831:19116
performSyncWorkOnRoot @ chunk-SM3OW2QE.js?v=95e8d831:18874
flushSyncCallbacks @ chunk-SM3OW2QE.js?v=95e8d831:9119
(anonymous) @ chunk-SM3OW2QE.js?v=95e8d831:18627Understand this warning
notes-autosave.ts:53 [Autosave] Saving note: 019b9230-29d3-7075-b97d-b785ec62f460
notes-async.ts:194 [Atoms] ✅ Updated note 019b9230-29d3-7075-b97d-b785ec62f460
notes-autosave.ts:57 [Autosave] ✅ Saved successfully
bridge.ts:201 [ConductorBridge] Initialized with RelationFilter, state: initialized
bridge.ts:247 [ConductorBridge] Hydrated 8 entities, state: ready
extractor-facade.ts:199 [Extractor] Initialized with 8 entities
RichEditor.tsx:376 [RichEditor] Extractor initialized
HighlighterBridge.ts:59 [HighlighterBridge] WASM initialized
HighlighterBridge.ts:103 [HighlighterBridge] Hydrated with 8 entities
decoration-cache.ts:41 [DecorationCache] All caches invalidated (entity version → 1)
RichEditor.tsx:596 [RichEditor] Entity hydration detected, triggering highlighter rescan
kittcore.js:3620 [WASM] scan() len=1842 spans=0
kittcore.js:3620 [WASM] scan_full len=1842 spans=0
kittcore.js:3620 [SVO] VPs:27 entities:39
kittcore.js:3620 [SVO] VP 'Learning' sent:0-40 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'Combat' sent:0-452 subj:true obj:true
kittcore.js:3620 [SVO] VP 'binds' sent:600-640 subj:false obj:true
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'leads' sent:850-888 subj:true obj:false
kittcore.js:3620 [SVO] VP 'killed' sent:888-946 subj:true obj:true
kittcore.js:3620 [SVO] VP 'defeated' sent:946-979 subj:true obj:false
kittcore.js:3620 [SVO] VP 'mentors' sent:979-1016 subj:true obj:true
kittcore.js:3620 [SVO] VP 'teaches' sent:1016-1051 subj:true obj:true
kittcore.js:3620 [SVO] VP 'shaping' sent:1016-1051 subj:true obj:false
kittcore.js:3620 [SVO] VP 'loves' sent:1051-1096 subj:true obj:true
3kittcore.js:3620 [SVO] VP 'ALLY' sent:1224-1351 subj:true obj:true
kittcore.js:3620 [SVO] VP 'bed' sent:1400-1427 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knocking' sent:1427-1462 subj:true obj:false
kittcore.js:3620 [SVO] VP 'morning' sent:1462-1486 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'yelled' sent:1486-1499 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'groaned' sent:1499-1515 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knew' sent:1515-1564 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'destroy' sent:1515-1564 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'needed' sent:1564-1598 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'find' sent:1564-1598 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'guards' sent:1598-1637 subj:true obj:false
kittcore.js:3620 [SVO] VP 'holds' sent:1637-1681 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'possesses' sent:1681-1715 subj:true obj:false
kittcore.js:3620 [SVO] VP 'fought' sent:1715-1773 subj:true obj:true
kittcore.js:3620 [SVO] VP 'laughed' sent:1773-1808 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [CST Debug] Phase 7: 39 entity spans for CST: ["Zorian Kazinski@63-78", "Zach Noveda@131-142", "Red Robe@197-205", "Quatach-Ichl@259-271", "Alanic@320-326", "Xvim@383-387", "Kirielle@443-451", "Silverlake@501-511", "Zorian Kazinski@615-621", "Zach Noveda@626-630", "Zorian Kazinski@642-657", "Zach Noveda@673-684", "Red Robe@688-696", "Zorian Kazinski@712-727", "Red Robe@732-740", "Zach Noveda@756-767", "Quatach-Ichl@851-863", "Quatach-Ichl@889-901", "Zorian Kazinski@909-924", "Zorian Kazinski@947-953", "Alanic@980-986", "Zorian Kazinski@995-1001", "Xvim@1017-1021", "Zorian Kazinski@1030-1036", "Zorian Kazinski@1052-1058", "Kirielle@1065-1073", "Zorian Kazinski@1197-1203", "Zorian Kazinski@1227-1242", "Zorian Kazinski@1266-1281", "Zach Noveda@1307-1318", "Zorian Kazinski@1333-1348", "Zorian Kazinski@1401-1407", "Kirielle@1428-1436", "Zorian Kazinski@1500-1506", "Silverlake@1599-1609", "Quatach-Ichl@1682-1694", "Zorian Kazinski@1716-1722", "Zach Noveda@1727-1731", "Quatach-Ichl@1739-1751"]
kittcore.js:3620 [RelationEngine] extract called with 39 entities, 14 existing edges
kittcore.js:3620 [SVO] VPs:27 entities:39
kittcore.js:3620 [SVO] VP 'Learning' sent:0-40 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'Combat' sent:0-452 subj:true obj:true
kittcore.js:3620 [SVO] VP 'binds' sent:600-640 subj:false obj:true
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'leads' sent:850-888 subj:true obj:false
kittcore.js:3620 [SVO] VP 'killed' sent:888-946 subj:true obj:true
kittcore.js:3620 [SVO] VP 'defeated' sent:946-979 subj:true obj:false
kittcore.js:3620 [SVO] VP 'mentors' sent:979-1016 subj:true obj:true
kittcore.js:3620 [SVO] VP 'teaches' sent:1016-1051 subj:true obj:true
kittcore.js:3620 [SVO] VP 'shaping' sent:1016-1051 subj:true obj:false
kittcore.js:3620 [SVO] VP 'loves' sent:1051-1096 subj:true obj:true
3kittcore.js:3620 [SVO] VP 'ALLY' sent:1224-1351 subj:true obj:true
kittcore.js:3620 [SVO] VP 'bed' sent:1400-1427 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knocking' sent:1427-1462 subj:true obj:false
kittcore.js:3620 [SVO] VP 'morning' sent:1462-1486 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'yelled' sent:1486-1499 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'groaned' sent:1499-1515 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knew' sent:1515-1564 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'destroy' sent:1515-1564 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'needed' sent:1564-1598 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'find' sent:1564-1598 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'guards' sent:1598-1637 subj:true obj:false
kittcore.js:3620 [SVO] VP 'holds' sent:1637-1681 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'possesses' sent:1681-1715 subj:true obj:false
kittcore.js:3620 [SVO] VP 'fought' sent:1715-1773 subj:true obj:true
kittcore.js:3620 [SVO] VP 'laughed' sent:1773-1808 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [CST] chunks:152 vps:27 svo_patterns:17 raw_rels:17
kittcore.js:3620 [RelationEngine] CST projection returned 17 relations
KittHighlighter.ts:736 [KittHighlighter] Rust: 37.0ms, 40 spans
RichEditor.tsx:381 [RichEditor] HighlighterBridge ready
RichEditor.tsx:397 [RichEditor] Extraction scan on note open: 019b9230-29d3-7075-b97d-b785ec62f460
kittcore.js:3620 [WASM] scan() len=1841 spans=0
kittcore.js:3620 [WASM] scan_full len=1841 spans=0
kittcore.js:3620 [SVO] VPs:27 entities:39
kittcore.js:3620 [SVO] VP 'Learning' sent:0-39 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'Combat' sent:0-451 subj:true obj:true
kittcore.js:3620 [SVO] VP 'binds' sent:599-639 subj:false obj:true
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'leads' sent:849-887 subj:true obj:false
kittcore.js:3620 [SVO] VP 'killed' sent:887-945 subj:true obj:true
kittcore.js:3620 [SVO] VP 'defeated' sent:945-978 subj:true obj:false
kittcore.js:3620 [SVO] VP 'mentors' sent:978-1015 subj:true obj:true
kittcore.js:3620 [SVO] VP 'teaches' sent:1015-1050 subj:true obj:true
kittcore.js:3620 [SVO] VP 'shaping' sent:1015-1050 subj:true obj:false
kittcore.js:3620 [SVO] VP 'loves' sent:1050-1095 subj:true obj:true
3kittcore.js:3620 [SVO] VP 'ALLY' sent:1223-1350 subj:true obj:true
kittcore.js:3620 [SVO] VP 'bed' sent:1399-1426 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knocking' sent:1426-1461 subj:true obj:false
kittcore.js:3620 [SVO] VP 'morning' sent:1461-1485 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'yelled' sent:1485-1498 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'groaned' sent:1498-1514 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knew' sent:1514-1563 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'destroy' sent:1514-1563 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'needed' sent:1563-1597 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'find' sent:1563-1597 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'guards' sent:1597-1636 subj:true obj:false
kittcore.js:3620 [SVO] VP 'holds' sent:1636-1680 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'possesses' sent:1680-1714 subj:true obj:false
kittcore.js:3620 [SVO] VP 'fought' sent:1714-1772 subj:true obj:true
kittcore.js:3620 [SVO] VP 'laughed' sent:1772-1807 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [CST Debug] Phase 7: 39 entity spans for CST: ["Zorian Kazinski@62-77", "Zach Noveda@130-141", "Red Robe@196-204", "Quatach-Ichl@258-270", "Alanic@319-325", "Xvim@382-386", "Kirielle@442-450", "Silverlake@500-510", "Zorian Kazinski@614-620", "Zach Noveda@625-629", "Zorian Kazinski@641-656", "Zach Noveda@672-683", "Red Robe@687-695", "Zorian Kazinski@711-726", "Red Robe@731-739", "Zach Noveda@755-766", "Quatach-Ichl@850-862", "Quatach-Ichl@888-900", "Zorian Kazinski@908-923", "Zorian Kazinski@946-952", "Alanic@979-985", "Zorian Kazinski@994-1000", "Xvim@1016-1020", "Zorian Kazinski@1029-1035", "Zorian Kazinski@1051-1057", "Kirielle@1064-1072", "Zorian Kazinski@1196-1202", "Zorian Kazinski@1226-1241", "Zorian Kazinski@1265-1280", "Zach Noveda@1306-1317", "Zorian Kazinski@1332-1347", "Zorian Kazinski@1400-1406", "Kirielle@1427-1435", "Zorian Kazinski@1499-1505", "Silverlake@1598-1608", "Quatach-Ichl@1681-1693", "Zorian Kazinski@1715-1721", "Zach Noveda@1726-1730", "Quatach-Ichl@1738-1750"]
kittcore.js:3620 [RelationEngine] extract called with 39 entities, 14 existing edges
kittcore.js:3620 [SVO] VPs:27 entities:39
kittcore.js:3620 [SVO] VP 'Learning' sent:0-39 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'Combat' sent:0-451 subj:true obj:true
kittcore.js:3620 [SVO] VP 'binds' sent:599-639 subj:false obj:true
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'leads' sent:849-887 subj:true obj:false
kittcore.js:3620 [SVO] VP 'killed' sent:887-945 subj:true obj:true
kittcore.js:3620 [SVO] VP 'defeated' sent:945-978 subj:true obj:false
kittcore.js:3620 [SVO] VP 'mentors' sent:978-1015 subj:true obj:true
kittcore.js:3620 [SVO] VP 'teaches' sent:1015-1050 subj:true obj:true
kittcore.js:3620 [SVO] VP 'shaping' sent:1015-1050 subj:true obj:false
kittcore.js:3620 [SVO] VP 'loves' sent:1050-1095 subj:true obj:true
3kittcore.js:3620 [SVO] VP 'ALLY' sent:1223-1350 subj:true obj:true
kittcore.js:3620 [SVO] VP 'bed' sent:1399-1426 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knocking' sent:1426-1461 subj:true obj:false
kittcore.js:3620 [SVO] VP 'morning' sent:1461-1485 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'yelled' sent:1485-1498 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'groaned' sent:1498-1514 subj:true obj:false
kittcore.js:3620 [SVO] VP 'knew' sent:1514-1563 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'destroy' sent:1514-1563 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'needed' sent:1563-1597 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'find' sent:1563-1597 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'guards' sent:1597-1636 subj:true obj:false
kittcore.js:3620 [SVO] VP 'holds' sent:1636-1680 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [SVO] VP 'possesses' sent:1680-1714 subj:true obj:false
kittcore.js:3620 [SVO] VP 'fought' sent:1714-1772 subj:true obj:true
kittcore.js:3620 [SVO] VP 'laughed' sent:1772-1807 subj:false obj:false
kittcore.js:3620 [SVO] SKIP: no subject
kittcore.js:3620 [CST] chunks:152 vps:27 svo_patterns:17 raw_rels:17
kittcore.js:3620 [RelationEngine] CST projection returned 17 relations
extractor-facade.ts:107 [Extractor] Persisted 22/43 unified relations (CST+Graph)
extractor-facade.ts:158 [Extractor] Converted 14/14 triples → relationships
extractor-facade.ts:164 [Extractor] Extraction complete: {implicit: 39, unified: 43, triples: 14, temporal: 1, time_us: 10664}
extractor-facade.ts:174 [Extractor] Unified Relations: (43) ['Zach Noveda -[FOUGHT]-> Red Robe (CST)', 'Quatach-Ichl -[LEADS]->  (CST)', 'Quatach-Ichl -[KILLED]-> Zorian Kazinski (CST)', 'Zorian Kazinski -[DEFEATED]->  (CST)', 'Alanic -[MENTORED]-> Zorian Kazinski (CST)', 'Xvim -[TAUGHT]-> Zorian Kazinski (CST)', 'Zorian Kazinski -[SHAPING]->  (CST)', 'Zorian Kazinski -[LOVES]-> Kirielle (CST)', 'Zorian Kazinski -[ALLIED_WITH]-> Zorian Kazinski (CST)', 'Zorian Kazinski -[ALLIED_WITH]-> Zach Noveda (CST)', 'Zach Noveda -[ALLIED_WITH]-> Zorian Kazinski (CST)', 'Zorian Kazinski -[BED]->  (CST)', 'Kirielle -[KNOCKING]->  (CST)', 'Zorian Kazinski -[GROANED]->  (CST)', 'Silverlake -[GUARDS]->  (CST)', 'Quatach-Ichl -[POSSESSES]->  (CST)', 'Zach Noveda -[FOUGHT]-> Quatach-Ichl (CST)', 'Zach Noveda -[ASSOCIATED_WITH]-> Kael (Inferred)', 'Zach Noveda -[ASSOCIATED_WITH]-> Taiven (Inferred)', 'Zach Noveda -[ASSOCIATED_WITH]-> Alanic (Inferred)', 'Zach Noveda -[ASSOCIATED_WITH]-> Xvim (Inferred)', 'Zach Noveda -[ASSOCIATED_WITH]-> Kirielle (Inferred)', 'Red Robe -[ASSOCIATED_WITH]-> Quatach-Ichl (Inferred)', 'Red Robe -[ASSOCIATED_WITH]-> Kael (Inferred)', 'Red Robe -[ASSOCIATED_WITH]-> Taiven (Inferred)', 'Red Robe -[ASSOCIATED_WITH]-> Alanic (Inferred)', 'Red Robe -[ASSOCIATED_WITH]-> Xvim (Inferred)', 'Red Robe -[ASSOCIATED_WITH]-> Kirielle (Inferred)', 'Quatach-Ichl -[ASSOCIATED_WITH]-> Kael (Inferred)', 'Quatach-Ichl -[ASSOCIATED_WITH]-> Taiven (Inferred)', 'Quatach-Ichl -[ASSOCIATED_WITH]-> Alanic (Inferred)', 'Quatach-Ichl -[ASSOCIATED_WITH]-> Xvim (Inferred)', 'Quatach-Ichl -[ASSOCIATED_WITH]-> Kirielle (Inferred)', 'Kael -[ASSOCIATED_WITH]-> Taiven (Inferred)', 'Kael -[ASSOCIATED_WITH]-> Alanic (Inferred)', 'Kael -[ASSOCIATED_WITH]-> Xvim (Inferred)', 'Kael -[ASSOCIATED_WITH]-> Kirielle (Inferred)', 'Taiven -[ASSOCIATED_WITH]-> Alanic (Inferred)', 'Taiven -[ASSOCIATED_WITH]-> Xvim (Inferred)', 'Taiven -[ASSOCIATED_WITH]-> Kirielle (Inferred)', 'Alanic -[ASSOCIATED_WITH]-> Xvim (Inferred)', 'Alanic -[ASSOCIATED_WITH]-> Kirielle (Inferred)', 'Xvim -[ASSOCIATED_WITH]-> Kirielle (Inferred)']
extractor-facade.ts:186 [Extractor] Graph synced: {entities: 10, edges: 14, relations: 0, ms: 3}