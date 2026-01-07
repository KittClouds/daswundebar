# Tauri Scanner Suite - Current State

**Date**: 2026-01-07  
**Status**: ✅ Core Scanner Suite + NarrativeGraph Complete  
**Tests**: 141 passing

---

## Ported Modules (19 total)

| Module | Lines | Tests | Description |
|--------|-------|-------|-------------|
| `scanner.rs` | ~230 | 4 | UnifiedScanner for wikilinks, entities, tags |
| `implicit.rs` | ~400 | 7 | ImplicitCortex - Aho-Corasick entity matching |
| `temporal.rs` | ~420 | 6 | TemporalCortex - temporal expression detection |
| `triple.rs` | ~240 | 11 | TripleCortex - explicit triple extraction |
| `verb_morphology.rs` | ~515 | 6 | VerbLexicon - 600+ verb forms with relations |
| `chunker.rs` | ~530 | 9 | Chunker - NP/VP/PP rule-based detection |
| `relation.rs` | ~450 | 8 | RelationEngine - CST + Graph inference |
| `structured_relation.rs` | ~500 | 5 | StructuredRelationExtractor - SVO patterns |
| `incremental.rs` | ~660 | 11 | IncrementalState - LCS-based delta scanning |
| `relation_schema.rs` | 1341 | 10 | Type-safe relation definitions |
| `document.rs` | ~500 | 6 | DocumentCortex - unified scanner coordinator |
| `conductor.rs` | ~220 | 8 | ScanConductor - state machine lifecycle |
| `relation_filter.rs` | ~315 | 6 | Post-processing confidence adjustment |
| `attacher.rs` | ~280 | 4 | Dependency graph builder from chunks |
| `resolver.rs` | ~310 | 5 | Coreference resolution (he→Gandalf) |
| `dialogue.rs` | ~180 | 3 | Speaker attribution for quotes |
| `constraints.rs` | ~450 | 14 | Ref validation, uniqueness, predicates |
| `narrative.rs` | ~270 | 8 | NarrativeGraph facade (NLP engine) |
| `change.rs` | ~240 | 10 | Content-addressable change detection |

---

## Key Improvements Over kittcore

### 1. **LCS-Based Incremental Diff** ⭐
- Single paragraph insert: 1 chunk dirty vs 100%

### 2. **O(log n) Shift Queries**
- Binary-search ShiftIndex for efficient shifts

### 3. **No WASM Overhead**
- Pure Rust, no `wasm_bindgen` serialization

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      ScanConductor                          │
│   (State machine: Uninitialized → Initialized → Ready)      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      DocumentCortex                         │
│   (Unified scan: implicit → temporal → triple → relations) │
└─────────────────────────────────────────────────────────────┘
         │           │           │           │           │
         ▼           ▼           ▼           ▼           ▼
    Implicit    Temporal     Triple    Relation   Structured
     Cortex      Cortex      Cortex     Engine    Extractor
                               │
                               ▼
                       NarrativeGraph
               (Chunker + Attacher + Resolver + Dialogue)
```

---

## TODO Queue

### ✅ COMPLETED

| Phase | Modules | Tests Added |
|-------|---------|-------------|
| P0 | TypeScript facade, Tauri commands | — |
| P1 | relation_filter, attacher | 10 |
| P1.5 | resolver, dialogue | 8 |
| P2 | constraints, narrative, change | 32 |

### 🟡 P3 - Optional (WASM-Heavy, Lower Priority)

| Module | Lines | Notes |
|--------|-------|-------|
| `projections.rs` | 880+ | Graph projection - heavy WASM |
| `reflex.rs` | 370+ | Reflexive patterns - heavy WASM |
| `core.rs` | 410 | DocumentScanner facade - depends on syntax.rs, reflex.rs |
| `syntax.rs` | 620+ | **Skip** - redundant with UnifiedScanner |
| `unified.rs` | 1000+ | **Skip** - replaced by ScanConductor |

---

## Test Coverage: 141 tests

```
scanner            4    relation_filter    6
implicit           7    attacher           4
temporal           6    resolver           5
triple            11    dialogue           3
verb_morphology    6    constraints       14
chunker            9    narrative          8
relation           8    change            10
structured_rel     5    
incremental       11    
relation_schema   10    
document           6    
conductor          8    
                        TOTAL            141
```

---

*Updated 2026-01-07 after Phase 2 completion*
