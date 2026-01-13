import { useCallback, useEffect, useRef, useMemo, useState } from 'react';
import { useEditor, EditorContent } from '@tiptap/react';
import { RichTextProvider } from 'reactjs-tiptap-editor';

// Base tiptap extensions
import { Document } from '@tiptap/extension-document';
import { Text } from '@tiptap/extension-text';
import { Paragraph } from '@tiptap/extension-paragraph';
import { HardBreak } from '@tiptap/extension-hard-break';
import { Dropcursor, Gapcursor, Placeholder } from '@tiptap/extensions';
import { TextStyle } from '@tiptap/extension-text-style';
import { ListItem } from '@tiptap/extension-list-item';

// Import extensions from reactjs-tiptap-editor
import { Attachment } from 'reactjs-tiptap-editor/attachment';
import { Blockquote } from 'reactjs-tiptap-editor/blockquote';
import { Bold } from 'reactjs-tiptap-editor/bold';
import { BulletList } from 'reactjs-tiptap-editor/bulletlist';
import { Clear } from 'reactjs-tiptap-editor/clear';
import { Code } from 'reactjs-tiptap-editor/code';
import { CodeBlock } from 'reactjs-tiptap-editor/codeblock';
import { Color } from 'reactjs-tiptap-editor/color';
import { Emoji } from 'reactjs-tiptap-editor/emoji';
import { ExportPdf } from 'reactjs-tiptap-editor/exportpdf';
import { ExportWord } from 'reactjs-tiptap-editor/exportword';
import { FontFamily } from 'reactjs-tiptap-editor/fontfamily';
import { FontSize } from 'reactjs-tiptap-editor/fontsize';
import { Heading } from 'reactjs-tiptap-editor/heading';
import { Highlight } from 'reactjs-tiptap-editor/highlight';
import { History } from 'reactjs-tiptap-editor/history';
import { HorizontalRule } from 'reactjs-tiptap-editor/horizontalrule';
import { Iframe } from 'reactjs-tiptap-editor/iframe';
import { Image } from 'reactjs-tiptap-editor/image';
import { ImageGif } from 'reactjs-tiptap-editor/imagegif';
import { ImportWord } from 'reactjs-tiptap-editor/importword';
import { Indent } from 'reactjs-tiptap-editor/indent';
import { Italic } from 'reactjs-tiptap-editor/italic';
import { LineHeight } from 'reactjs-tiptap-editor/lineheight';
import { Link } from 'reactjs-tiptap-editor/link';
import { Mention } from 'reactjs-tiptap-editor/mention';
import { MoreMark } from 'reactjs-tiptap-editor/moremark';
import { OrderedList } from 'reactjs-tiptap-editor/orderedlist';
import { SearchAndReplace } from 'reactjs-tiptap-editor/searchandreplace';
import { Strike } from 'reactjs-tiptap-editor/strike';
import { Table } from 'reactjs-tiptap-editor/table';
import { TaskList } from 'reactjs-tiptap-editor/tasklist';
import { TextAlign } from 'reactjs-tiptap-editor/textalign';
import { TextUnderline } from 'reactjs-tiptap-editor/textunderline';
import { Video } from 'reactjs-tiptap-editor/video';
import { TextDirection } from 'reactjs-tiptap-editor/textdirection';
import { Katex } from 'reactjs-tiptap-editor/katex';
import { Drawer } from 'reactjs-tiptap-editor/drawer';
import { Excalidraw } from 'reactjs-tiptap-editor/excalidraw';
import { Twitter } from 'reactjs-tiptap-editor/twitter';
import { Mermaid } from 'reactjs-tiptap-editor/mermaid';
import { Column } from 'reactjs-tiptap-editor/column';
import { SlashCommand } from 'reactjs-tiptap-editor/slashcommand';

// Custom entity extensions - marks for commands/parsing, unified plugin for decorations
import { EntityMark } from '@/lib/extensions/EntityMark';
import { TagMark } from '@/lib/extensions/TagMark';
import { MentionMarkExt } from '@/lib/extensions/MentionMarkExt';
import { WikiLinkMark } from '@/lib/extensions/WikiLinkMark';
// Using RustHighlighter for Tauri pipeline (A/B test)
import { RustHighlighter } from '@/lib/extensions/RustHighlighter';
import { highlighterBridge } from '@/lib/highlighter';
import { useNER } from '@/contexts/NERContext';
import type { HighlightMode } from '@/atoms/highlightingAtoms';
import type { EntityKind } from '@/lib/types/entityTypes';

// Extractor - Rust-powered entity extraction (highlighter inits at app level)
import { scannerFacade } from '@/lib/scanner';

// Import CSS
import 'reactjs-tiptap-editor/style.css';
import 'prism-code-editor-lightweight/layout.css';
import 'prism-code-editor-lightweight/themes/github-dark.css';
import 'katex/dist/katex.min.css';
import 'easydrawer/styles.css';
import 'react-image-crop/dist/ReactCrop.css';

// Import editor components
import EditorToolbar from './EditorToolbar';
import EditorBubbleMenus from './EditorBubbleMenus';



interface RichEditorProps {
  content: string;
  onChange: (content: string) => void;
  isDarkMode?: boolean;
  noteId?: string;
  toolbarVisible?: boolean;
  onToolbarVisibilityChange?: (visible: boolean) => void;
  onWikilinkClick?: (title: string) => void;
  checkWikilinkExists?: (title: string) => boolean;
  onTemporalClick?: (temporal: string) => void;
  onBacklinkClick?: (title: string) => void;
}

function convertBase64ToBlob(base64: string) {
  const arr = base64.split(',');
  const mime = arr[0].match(/:(.*?);/)![1];
  const bstr = atob(arr[1]);
  let n = bstr.length;
  const u8arr = new Uint8Array(n);
  while (n--) {
    u8arr[n] = bstr.charCodeAt(n);
  }
  return new Blob([u8arr], { type: mime });
}

/**
 * Extract plain text from TipTap JSON document for scanner
 */
function extractPlainText(node: any): string {
  if (!node) return '';
  if (typeof node === 'string') return node;
  if (node.type === 'text' && node.text) return node.text;
  if (node.content && Array.isArray(node.content)) {
    return node.content.map(extractPlainText).join('\n');
  }
  return '';
}

// Create extensions factory function to allow dynamic configuration
function createExtensions(
  onWikilinkClick?: (title: string) => void,
  checkWikilinkExists?: (title: string) => boolean,
  onTemporalClick?: (temporal: string) => void,
  onBacklinkClick?: (title: string) => void,
  getNEREntities?: () => any[],
  getNoteId?: () => string | undefined,  // Dynamic getter instead of static
  getHighlightMode?: () => HighlightMode,
  getFocusEntityKinds?: () => EntityKind[],
  getFstNerEnabled?: () => boolean
) {
  return [
    // Base Extensions
    Document,
    Text,
    Paragraph,
    HardBreak,
    Dropcursor,
    Gapcursor,
    TextStyle,
    ListItem,
    Placeholder.configure({
      placeholder: "Start writing or press '/' for commands...",
      showOnlyCurrent: true,
    }),

    // Editor Extensions
    History,
    SearchAndReplace,
    Clear,
    FontFamily,
    Heading.configure({ spacer: true }),
    FontSize,
    Bold,
    Italic,
    TextUnderline,
    Strike,
    MoreMark,
    Highlight,
    Emoji,
    Color.configure({ spacer: true }),
    BulletList,
    OrderedList,
    TextAlign.configure({ types: ['heading', 'paragraph'], spacer: true }),
    Indent,
    LineHeight,
    TaskList.configure({
      spacer: true,
      taskItem: {
        nested: true,
      },
    }),
    Link,
    Image.configure({
      upload: (files: File) => {
        return new Promise((resolve) => {
          setTimeout(() => {
            resolve(URL.createObjectURL(files));
          }, 500);
        });
      },
    }),
    Video.configure({
      upload: (files: File) => {
        return new Promise((resolve) => {
          setTimeout(() => {
            resolve(URL.createObjectURL(files));
          }, 500);
        });
      },
    }),
    ImageGif.configure({
      API_KEY: import.meta.env.VITE_GIPHY_API_KEY || '',
      provider: 'giphy',
    }),
    Blockquote,
    HorizontalRule,
    Code.configure({
      toolbar: false,
    }),
    CodeBlock,
    Column,
    Table,
    Iframe,
    ExportPdf.configure({ spacer: true }),
    ImportWord.configure({
      upload: (files: File[]) => {
        const f = files.map(file => ({
          src: URL.createObjectURL(file),
          alt: file.name,
        }));
        return Promise.resolve(f);
      },
    }),
    ExportWord,
    TextDirection,
    Mention,
    Attachment.configure({
      upload: (file: any) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);

        return new Promise((resolve) => {
          setTimeout(() => {
            const blob = convertBase64ToBlob(reader.result as string);
            resolve(URL.createObjectURL(blob));
          }, 300);
        });
      },
    }),
    Katex,
    Excalidraw.configure({
      willReadFrequently: true,
      upload: (file: any) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);
        return new Promise((resolve) => {
          setTimeout(() => {
            const blob = convertBase64ToBlob(reader.result as string);
            resolve(URL.createObjectURL(blob));
          }, 300);
        });
      },
    }),
    Mermaid.configure({
      upload: (file: any) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);

        return new Promise((resolve) => {
          setTimeout(() => {
            const blob = convertBase64ToBlob(reader.result as string);
            resolve(URL.createObjectURL(blob));
          }, 300);
        });
      },
    }),
    Drawer.configure({
      // @ts-expect-error: willReadFrequently is not in the type definition but is required for performance
      willReadFrequently: true,
      upload: (file: any) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);

        return new Promise((resolve) => {
          setTimeout(() => {
            const blob = convertBase64ToBlob(reader.result as string);
            resolve(URL.createObjectURL(blob));
          }, 300);
        });
      },
    }),
    Twitter,
    SlashCommand,

    // Custom entity extensions - marks for commands/parsing
    EntityMark,
    TagMark,
    MentionMarkExt,
    WikiLinkMark,

    // RustHighlighter - Tauri-native pipeline (replaces KittHighlighter for A/B test)
    RustHighlighter.configure({
      onWikilinkClick,
      checkWikilinkExists,
      onTemporalClick,
      onBacklinkClick,
      nerEntities: getNEREntities,
      useWidgetMode: true,
      enableLinkTracking: true,
      currentNoteId: getNoteId,
      logPerformance: true,
      getHighlightMode,
      getFocusEntityKinds,
      getFstNerEnabled,
      onImplicitClick: (entityId, entityLabel) => {
        console.log('[RustHighlighter] Implicit clicked:', entityId, entityLabel);
      },
    }),
  ];
}

// ... imports
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import {
  noteContentAtom,
  debouncedNoteContentAtom,
  hasUnsavedChangesAtom,
  autosaveAtom,
  manualSaveAtom,
  initNoteContentAtom,
  selectedNoteAtom,
  selectedNoteIdAtom,
} from '@/atoms';
import { editorInstanceAtom } from '@/atoms/editorAtoms';
import { highlightSettingsAtom, fstNerEnabledAtom } from '@/atoms/highlightingAtoms';

// ... extensions factory ...

const RichEditor = ({
  isDarkMode = false,
  // content, // Deprecated
  // onChange, // Deprecated
  // noteId, // Deprecated - using atom
  toolbarVisible = true,
  onToolbarVisibilityChange,
  onWikilinkClick,
  checkWikilinkExists,
  onTemporalClick,
  onBacklinkClick,
}: Omit<RichEditorProps, 'content' | 'onChange' | 'noteId'> & { content?: string, onChange?: any, noteId?: string }) => {
  const selectedNote = useAtomValue(selectedNoteAtom);
  const noteId = useAtomValue(selectedNoteIdAtom) || undefined;

  const [content, setContent] = useAtom(noteContentAtom);
  const debouncedContent = useAtomValue(debouncedNoteContentAtom);
  const hasUnsavedChanges = useAtomValue(hasUnsavedChangesAtom);
  const triggerAutosave = useSetAtom(autosaveAtom);
  const manualSave = useSetAtom(manualSaveAtom);
  const initContent = useSetAtom(initNoteContentAtom);
  const setEditorInstance = useSetAtom(editorInstanceAtom);

  // Highlighting mode settings (reactive via ref)
  const highlightSettings = useAtomValue(highlightSettingsAtom);
  const highlightSettingsRef = useRef(highlightSettings);
  useEffect(() => {
    highlightSettingsRef.current = highlightSettings;
  }, [highlightSettings]);

  // FST-NER enabled (reactive via ref)
  const fstNerEnabled = useAtomValue(fstNerEnabledAtom);
  const fstNerEnabledRef = useRef(fstNerEnabled);
  useEffect(() => {
    fstNerEnabledRef.current = fstNerEnabled;
  }, [fstNerEnabled]);

  const previousContentRef = useRef<string>('');
  const previousNoteIdRef = useRef<string | undefined>(undefined);
  const { entities } = useNER();
  const nerEntitiesRef = useRef(entities);

  // Keep NER entities ref updated
  useEffect(() => {
    nerEntitiesRef.current = entities;
  }, [entities]);

  // Ref to track if we've done initial scan for this note
  const initialScanDoneRef = useRef<string | null>(null);

  // Initialize Extractor (highlighter already init'd at app level in App.tsx)
  useEffect(() => {
    let cancelled = false;

    (async () => {
      // Initialize extractor only - highlighter is app-level
      await scannerFacade.initialize();

      if (cancelled) return;
      console.log('[RichEditor] Extractor initialized');

      // Initialize Rust highlighter bridge (WASM already loaded, this sets up hydration)
      const rustReady = await highlighterBridge.initialize();
      if (rustReady && !cancelled) {
        console.log('[RichEditor] HighlighterBridge ready');
      }

      // Pre-load decoration cache from SQLite (if cached, skips scanning)
      if (selectedNote?.id && selectedNote.content && initialScanDoneRef.current !== selectedNote.id) {
        try {
          const doc = JSON.parse(selectedNote.content);
          const text = extractPlainText(doc);
          if (text.length > 0) {
            // Try to load from persistent cache first
            const cacheHit = await highlighterBridge.preloadCache(selectedNote.id, text);
            if (cacheHit) {
              console.log('[RichEditor] Decorations loaded from cache:', selectedNote.id);
            }

            // Extraction scan (for entity extraction → DB, independent of decorations)
            console.log('[RichEditor] Extraction scan on note open:', selectedNote.id);
            scannerFacade.scan(selectedNote.id, text);
            initialScanDoneRef.current = selectedNote.id;
          }
        } catch {
          if (selectedNote.content.length > 0) {
            scannerFacade.scan(selectedNote.id, selectedNote.content);
            initialScanDoneRef.current = selectedNote.id;
          }
        }
      }
    })();

    return () => {
      cancelled = true;
      scannerFacade.shutdown();
      console.log('[RichEditor] Extractor shutdown');
    };
  }, [selectedNote?.id]);

  // Initialize content when note is selected
  useEffect(() => {
    // Only init if we have a note and it's different from previous init or we just mounted
    if (selectedNote?.id && selectedNote.id !== previousNoteIdRef.current) {
      // console.log('[RichEditor] Initializing content for note:', selectedNote.id);
      const initialContent = selectedNote.content || '';
      initContent(initialContent);
      previousNoteIdRef.current = selectedNote.id;
      previousContentRef.current = initialContent; // Reset change tracking
    }
  }, [selectedNote?.id, selectedNote?.content, initContent]);


  // Trigger autosave and Rust scanner when debounced content changes
  useEffect(() => {
    if (selectedNote?.id && debouncedContent) {
      // Autosave
      triggerAutosave();

      // Trigger Rust scanner for entity/relation extraction
      try {
        const doc = JSON.parse(debouncedContent);
        const text = extractPlainText(doc);
        if (text.length > 0) {
          scannerFacade.scan(selectedNote.id, text);
        }
      } catch (e) {
        // Non-JSON content, use as-is
        scannerFacade.scan(selectedNote.id, debouncedContent);
      }
    }
  }, [debouncedContent, selectedNote?.id, triggerAutosave]);

  // Manual save handler (Cmd+S / Ctrl+S)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        manualSave();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [manualSave]);

  // Use refs for callbacks to keep extensions stable
  const optionsRef = useRef({ onWikilinkClick, checkWikilinkExists, onTemporalClick, onBacklinkClick });
  useEffect(() => {
    optionsRef.current = { onWikilinkClick, checkWikilinkExists, onTemporalClick, onBacklinkClick };
  });

  // Use ref for noteId to avoid re-creating extensions on every note switch
  const noteIdRef = useRef(noteId);
  useEffect(() => {
    noteIdRef.current = noteId;
  }, [noteId]);

  // Create extensions ONCE - stable across note switches
  // CRITICAL: Extensions are expensive to recreate (~150ms TipTap remount)
  // Using refs for dynamic values that need to update without recreation
  const extensions = useMemo(
    () => createExtensions(
      (title) => optionsRef.current.onWikilinkClick?.(title),
      (title) => optionsRef.current.checkWikilinkExists?.(title) ?? true,
      (temporal) => optionsRef.current.onTemporalClick?.(temporal),
      (title) => optionsRef.current.onBacklinkClick?.(title),
      () => nerEntitiesRef.current,
      () => noteIdRef.current,  // Dynamic getter instead of static value
      () => highlightSettingsRef.current.mode,  // Highlighting mode getter
      () => highlightSettingsRef.current.focusEntityKinds,  // Focus kinds getter
      () => fstNerEnabledRef.current  // FST-NER enabled getter
    ),
    []  // ← STABLE - never re-creates extensions
  );

  // Parse content to JSON if needed
  const parseContent = useCallback((contentStr: string) => {
    try {
      if (!contentStr) {
        return {
          type: 'doc',
          content: [{ type: 'paragraph' }],
        };
      }
      return typeof contentStr === 'string' ? JSON.parse(contentStr) : contentStr;
    } catch {
      return {
        type: 'doc',
        content: [{ type: 'paragraph', content: [{ type: 'text', text: contentStr }] }],
      };
    }
  }, []);

  const editor = useEditor({
    extensions,
    content: parseContent(content), // Initial render content
    onUpdate: ({ editor }) => {
      const json = editor.getJSON();
      const jsonString = JSON.stringify(json);

      if (jsonString !== previousContentRef.current) {
        previousContentRef.current = jsonString;
        setContent(jsonString); // Update atom
      }
    },
    editorProps: {
      attributes: {
        class: 'prose prose-sm sm:prose lg:prose-lg xl:prose-xl focus:outline-none min-h-[400px] p-6',
        'data-gramm': 'false',            // Disable Grammarly
        'data-gramm_editor': 'false',     // Disable Grammarly
        'data-enable-grammarly': 'false', // Disable Grammarly
        'spellcheck': 'false',            // Disable native spellcheck
      },

    },
  });

  // Handle keyboard shortcut for toolbar toggle
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === '\\') {
        e.preventDefault();
        if (onToolbarVisibilityChange) {
          onToolbarVisibilityChange(!toolbarVisible);
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [toolbarVisible, onToolbarVisibilityChange]);

  // Expose editor instance via atom for AI inline editing
  useEffect(() => {
    setEditorInstance(editor);
    return () => setEditorInstance(null);
  }, [editor, setEditorInstance]);

  // Dispatch transaction when highlighting mode changes to trigger reactive rebuild
  // This interacts with KittHighlighter's apply() method
  useEffect(() => {
    if (editor) {
      editor.view.dispatch(
        editor.state.tr.setMeta('highlightModeChange', true)
      );
    }
  }, [highlightSettings, editor]);

  // Dispatch transaction when FST-NER is toggled to trigger rescan
  useEffect(() => {
    if (editor) {
      editor.view.dispatch(
        editor.state.tr.setMeta('forceRescan', true)
      );
    }
  }, [fstNerEnabled, editor]);

  // Force editor update when noteId changes (switching notes) creates a ref mismatch?
  // We use selectedNoteId vs internal previousNoteIdRef.
  // The initContent effect sets the atom. 
  // We need to update the EDITOR instance if the content changed externally (e.g. note switch).

  // Actually, I'll use a `lastRenderedNoteId` ref.
  const lastRenderedNoteId = useRef<string | undefined>(undefined);

  useEffect(() => {
    if (editor && selectedNote?.id && selectedNote.id !== lastRenderedNoteId.current) {
      // Note switched!
      const parsed = parseContent(selectedNote.content);
      editor.commands.setContent(parsed, { emitUpdate: false });
      lastRenderedNoteId.current = selectedNote.id;
      previousContentRef.current = selectedNote.content; // Reset change tracker
    }
  }, [selectedNote?.id, selectedNote?.content, editor, parseContent]);


  // Force editor update when NER entities change to refresh decorations
  useEffect(() => {
    if (editor && editor.view && entities.length > 0) {
      editor.view.dispatch(editor.state.tr.setMeta('nerUpdate', true));
    }
  }, [entities, editor]);

  // Force highlighter rescan when entities are hydrated (from useEntitySync)
  useEffect(() => {
    if (!editor?.view) return;

    const unsubscribe = highlighterBridge.onHydration(() => {
      console.log('[RichEditor] Entity hydration detected, triggering highlighter rescan');
      editor.view.dispatch(editor.state.tr.setMeta('entityHydration', true));
    });

    return unsubscribe;
  }, [editor]);


  // Delay bubble menu rendering until editor is fully ready
  const [editorReady, setEditorReady] = useState(false);
  useEffect(() => {
    if (editor?.view) {
      // Delay bubble menu mount by one frame to avoid accessing view before ready
      const timer = requestAnimationFrame(() => setEditorReady(true));
      return () => {
        cancelAnimationFrame(timer);
        setEditorReady(false);
      };
    }
    setEditorReady(false);
  }, [editor?.view]);

  if (!editor || !editor.view) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-muted-foreground">Loading editor...</div>
      </div>
    );
  }

  return (
    <div
      className="h-full flex flex-col min-h-0 reactjs-tiptap-editor relative"
      data-gramm="false"
      data-gramm_editor="false"
      data-enable-grammarly="false"
    >
      {/* Unsaved changes indicator */}
      {hasUnsavedChanges && (
        <div className="absolute top-2 right-2 flex items-center gap-2 text-xs text-muted-foreground z-50 bg-background/80 px-2 py-1 rounded border shadow-sm">
          <div className="w-2 h-2 rounded-full bg-yellow-500 animate-pulse" />
          <span>Unsaved</span>
        </div>
      )}

      <RichTextProvider editor={editor} dark={isDarkMode}>
        {/* Toolbar - delay until editor view is ready to prevent isActive() error */}
        {editorReady && toolbarVisible && <EditorToolbar />}

        {/* Editor Content Area - safe to render immediately */}
        <div
          className="flex-1 overflow-auto custom-scrollbar bg-background relative"
          data-gramm="false"
          data-gramm_editor="false"
        >
          <EditorContent editor={editor} className="min-h-full" />
        </div>

        {/* Bubble Menus - delay mount until editor is fully ready */}
        {editorReady && <EditorBubbleMenus />}
      </RichTextProvider>
    </div>
  );
};

export default RichEditor;
