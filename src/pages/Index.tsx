
import { AppSidebar } from '@/components/app-sidebar';
import { FooterLinksPanel } from '@/components/FooterLinksPanel';
import { SettingsDropdown } from '@/components/header/SettingsDropdown';
import { analyzeText, parseContentToPlainText } from '@/lib/analytics/textAnalytics';
import { useBlueprintHub } from '@/features/blueprint-hub/hooks/useBlueprintHub';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { Separator } from '@/components/ui/separator';
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar';
import { Button } from '@/components/ui/button';
import { FileText, Trash2 } from 'lucide-react';
import { GraphLogo } from '@/components/ui/GraphLogo';
import RichEditor from '@/components/editor/RichEditor';
import { useTheme } from '@/hooks/useTheme';
import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { ThemeToggle } from '@/components/ThemeToggle';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import { SchemaProvider } from '@/contexts/SchemaContext';
import { SchemaManager } from '@/components/schema/SchemaManager';
import { useLinkIndex } from '@/hooks/useLinkIndex';
import { useEntitySync } from '@/hooks/useEntitySync';
import { RightSidebar, RightSidebarProvider, RightSidebarTrigger } from '@/components/RightSidebar';
import { TemporalHighlightProvider, useTemporalHighlight } from '@/contexts/TemporalHighlightContext';
import { formatEntityTitle, getDisplayName } from '@/lib/utils/titleParser';
import type { WikiLink } from '@/lib/linking/LinkIndex';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';

import { useJotaiNotes } from '@/hooks/useJotaiNotes';

function NotesApp() {
  const { theme } = useTheme();
  const { toggleHub, isHubOpen } = useBlueprintHub();
  const [toolbarVisible, setToolbarVisible] = useState(() => {
    const saved = localStorage.getItem('editor-toolbar-visible');
    return saved !== null ? JSON.parse(saved) : true;
  });

  const { selectedNote, updateNoteContent, deleteNote, createNote, selectNote, state } = useJotaiNotes();
  const { activateTimelineWithTemporal } = useTemporalHighlight();

  useEntitySync({ debounceMs: 2000 });

  // Link index for wikilink navigation and footer links panel
  const { findNoteByTitle, noteExists, getBacklinks, getOutgoingLinks, getEntityStats, getEntityMentions } = useLinkIndex(state.notes);

  const backlinks = useMemo(() =>
    selectedNote ? getBacklinks(selectedNote) : [],
    [selectedNote, getBacklinks]
  );

  const outgoingLinks = useMemo(() =>
    selectedNote ? getOutgoingLinks(selectedNote.id) : [],
    [selectedNote, getOutgoingLinks]
  );

  const entityStats = useMemo(() =>
    selectedNote ? getEntityStats(selectedNote.id) : [],
    [selectedNote, getEntityStats]
  );

  // Navigate to a note by title (for footer links panel)
  const handleNavigateToNote = useCallback(async (
    title: string,
    createIfNotExists?: boolean,
    link?: WikiLink
  ) => {
    const existingNote = findNoteByTitleRef.current(title);
    if (existingNote) {
      selectNoteRef.current(existingNote.id);
    } else if (createIfNotExists) {
      // Build proper entity title if this is an entity link
      let noteTitle = title;
      if (link?.linkType === 'entity' && link.entityKind) {
        noteTitle = formatEntityTitle(link.entityKind, link.targetTitle);
      }

      // Pass source note ID for backlink creation
      const sourceNoteId = selectedNoteRef.current?.id;
      const newNote = await createNoteRef.current(undefined, noteTitle, sourceNoteId);
      selectNoteRef.current(newNote.id);
    }
  }, []);

  // Use refs to hold latest versions without changing callback references
  const findNoteByTitleRef = useRef(findNoteByTitle);
  const selectNoteRef = useRef(selectNote);
  const createNoteRef = useRef(createNote);
  const noteExistsRef = useRef(noteExists);
  const activateTimelineRef = useRef(activateTimelineWithTemporal);
  const selectedNoteRef = useRef(selectedNote);

  // Keep refs updated
  useEffect(() => {
    findNoteByTitleRef.current = findNoteByTitle;
    selectNoteRef.current = selectNote;
    createNoteRef.current = createNote;
    noteExistsRef.current = noteExists;
    activateTimelineRef.current = activateTimelineWithTemporal;
    selectedNoteRef.current = selectedNote;
  });

  const handleToolbarVisibilityChange = useCallback((visible: boolean) => {
    setToolbarVisible(visible);
    localStorage.setItem('editor-toolbar-visible', JSON.stringify(visible));
  }, []);

  const handleEditorChange = (content: string) => {
    if (selectedNote) {
      updateNoteContent(selectedNote.id, content);
    }
  };

  const handleDeleteNote = () => {
    if (selectedNote) {
      deleteNote(selectedNote.id);
    }
  };

  // STABLE callbacks using refs - no dependencies means reference never changes
  const handleWikilinkClick = useCallback(async (title: string) => {
    const existingNote = findNoteByTitleRef.current(title);
    if (existingNote) {
      selectNoteRef.current(existingNote.id);
    } else {
      const newNote = await createNoteRef.current(undefined, title);
      selectNoteRef.current(newNote.id);
    }
  }, []);

  const checkWikilinkExists = useCallback((title: string): boolean => {
    return noteExistsRef.current(title);
  }, []);

  const handleTemporalClick = useCallback((temporal: string) => {
    activateTimelineRef.current(temporal);
  }, []);

  // Handle backlink clicks - parse entity syntax from backlink title if present
  const handleBacklinkClick = useCallback((backlinkTitle: string) => {
    // Check if backlink contains entity syntax like [KIND|Label]
    const entityMatch = backlinkTitle.match(/^\[([A-Z_]+)(?::[A-Z_]+)?\|([^\]]+)\]$/);
    const searchTitle = entityMatch ? entityMatch[2] : backlinkTitle;

    const existingNote = findNoteByTitleRef.current(searchTitle);
    if (existingNote) {
      selectNoteRef.current(existingNote.id);
    } else {
      // Try to find by full title (entity syntax)
      const fullMatch = findNoteByTitleRef.current(backlinkTitle);
      if (fullMatch) {
        selectNoteRef.current(fullMatch.id);
      }
    }
  }, []);

  const textAnalytics = useMemo(() => {
    if (!selectedNote?.content) return null;
    // Import dynamically or assume imported. I will add import in next step if not present.
    // For now I'll use the imported functions.
    // Wait, I need to add the import first or this will fail compilation.
    // I can do it in one go if I check imports, but I'll assume I need to add it.
    // Actually, I can use the existing `analyzeText` and `parseContentToPlainText` if I import them.
    // Let me rewrite the component part first.

    // Actually, let's use the provided logic.
    const plainText = parseContentToPlainText(selectedNote.content);
    return analyzeText(plainText);
  }, [selectedNote?.content]);

  return (
    <RightSidebarProvider>
      <SidebarProvider>
        <div className="h-screen overflow-hidden flex w-full bg-background">
          {/* Left Sidebar */}
          <AppSidebar />

          {/* Main Content Area */}
          <div className="flex flex-col flex-1 min-w-0 h-full overflow-hidden">
            {/* Header */}
            <header className="flex h-14 shrink-0 items-center gap-2 border-b border-border bg-background px-4">
              <SidebarTrigger className="-ml-1" />
              <Separator orientation="vertical" className="mr-2 h-4" />
              <Breadcrumb className="flex-1">
                <BreadcrumbList>
                  <BreadcrumbItem className="hidden md:block">
                    <BreadcrumbLink
                      href="#"
                      className="text-muted-foreground hover:text-foreground transition-colors"
                    >
                      Notes
                    </BreadcrumbLink>
                  </BreadcrumbItem>
                  <BreadcrumbSeparator className="hidden md:block" />
                  <BreadcrumbItem>
                    <BreadcrumbPage className="text-foreground font-medium">
                      {selectedNote ? getDisplayName(selectedNote.title) : 'Select a note'}
                    </BreadcrumbPage>
                  </BreadcrumbItem>
                </BreadcrumbList>
              </Breadcrumb>

              {/* Header Actions */}
              <div className="flex items-center gap-1">
                {selectedNote && (
                  <AlertDialog>
                    <AlertDialogTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-muted-foreground hover:text-destructive"
                        title="Delete note"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>Delete note?</AlertDialogTitle>
                        <AlertDialogDescription>
                          This will permanently delete "{selectedNote.title}". This action cannot be
                          undone.
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                          onClick={handleDeleteNote}
                          className="bg-destructive hover:bg-destructive/90"
                        >
                          Delete
                        </AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                )}

                <SchemaManager />

                <SettingsDropdown
                  toolbarVisible={toolbarVisible}
                  onToolbarToggle={handleToolbarVisibilityChange}
                  schemaManagerTrigger={null}
                />

                <RightSidebarTrigger />
                <ThemeToggle />
              </div>
            </header>

            {/* Editor Area */}
            <main className="flex-1 min-h-0 overflow-auto custom-scrollbar">
              {selectedNote ? (
                <RichEditor
                  content={selectedNote.content}
                  onChange={handleEditorChange}
                  isDarkMode={theme === 'dark'}
                  toolbarVisible={toolbarVisible}
                  onToolbarVisibilityChange={handleToolbarVisibilityChange}
                  noteId={selectedNote.id}
                  onWikilinkClick={handleWikilinkClick}
                  checkWikilinkExists={checkWikilinkExists}
                  onTemporalClick={handleTemporalClick}
                  onBacklinkClick={handleBacklinkClick}
                />
              ) : (
                <div className="flex flex-col items-center justify-center h-full bg-background">
                  <div className="text-center space-y-6 max-w-md mx-auto px-6 animate-fade-in">
                    <div className="flex justify-center">
                      <GraphLogo className="w-24 h-24 opacity-80" />
                    </div>
                    <div className="space-y-3">
                      <h2 className="text-2xl font-semibold text-foreground">No note selected</h2>
                      <p className="text-muted-foreground leading-relaxed">
                        Select a note from the sidebar or create a new one to start writing
                      </p>
                    </div>
                    <Button onClick={() => createNote()} className="gap-2">
                      <FileText className="h-4 w-4" />
                      Create new note
                    </Button>
                  </div>
                </div>
              )}
            </main>
            <FooterLinksPanel
              backlinks={selectedNote ? backlinks : []}
              outgoingLinks={selectedNote ? outgoingLinks : []}
              entityStats={selectedNote ? entityStats : []}
              notes={state.notes}
              getEntityMentions={getEntityMentions}
              onNavigate={handleNavigateToNote}
              isHubOpen={isHubOpen}
              toggleHub={toggleHub}
              isSaving={state.isSaving}
              lastSaved={state.lastSaved}
              notesCount={state.notes.length}
              wordCount={textAnalytics?.wordCount}
              characterCount={textAnalytics?.characterCount}
            />
          </div>

          {/* Right Sidebar */}
          <RightSidebar />
        </div>

      </SidebarProvider>
    </RightSidebarProvider>
  );
}

const Index = () => {
  return (
    <ErrorBoundary>
      <SchemaProvider>
        <TemporalHighlightProvider>
          <NotesApp />
        </TemporalHighlightProvider>
      </SchemaProvider>
    </ErrorBoundary>
  );
};

export default Index;
