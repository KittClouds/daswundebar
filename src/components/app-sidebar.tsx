import { memo, useCallback, useMemo, useState } from 'react';
import * as React from "react";
import {
  ChevronRight,
  ChevronDown,
  Folder as FolderIcon,
  FolderOpen,
  Plus,
  MoreVertical,
  Star,
  StarOff,
  Link as LinkIcon,
  X,
  FileText,
  Search,
  FolderPlus,
  Check,
  Clock,
  User,
  MapPin,
  Users,
  Package,
  Flag,
  Film,
  Calendar,
  Lightbulb,
  AlertTriangle,
  Pencil,
  BookOpen,
  Zap,
  Hourglass,
  Waves,
  Drama,
  Book,
  Sparkles,
  Boxes,
  Settings,
  Cpu,
} from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
// DEPRECATED: TypeScript semantic search UI (libraries kept intact)
// import { SemanticSearchPanel } from "@/components/search/SemanticSearchPanel";
import { RustSearchPanel } from "@/components/search/RustSearchPanel";
import { RankedSearchResults } from "@/components/search/RankedSearchResults";
import { EntitiesPanel } from "@/components/EntitiesPanel";
import { SettingsPanel } from "@/components/settings";
import { ArboristTreeView } from '@/components/tree/ArboristTreeView';
import '@/styles/arborist-overrides.css';
import { NetworkFolderCreationMenu } from '@/components/network/NetworkFolderCreation';
import { useResoRankSearchWithDebounce } from '@/hooks/useResoRankSearch';
import { GraphLogo } from "@/components/ui/GraphLogo";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarRail,
  SidebarHeader,
  SidebarFooter,
} from "@/components/ui/sidebar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { FolderWithChildren, Note } from "@/types/noteTypes";
import { ENTITY_COLORS, ENTITY_SUBTYPES, EntityKind, ENTITY_ICONS, getSubtypesForKind } from "@/lib/types/entityTypes";
import { getDisplayName, parseEntityFromTitle, parseFolderEntityFromName, formatSubtypeFolderName } from "@/lib/utils/titleParser";
import { useBlueprintHub } from "@/features/blueprint-hub/hooks/useBlueprintHub";
// useJotaiNotes is already imported below or we should keep one
import { useJotaiNotes } from "@/hooks/useJotaiNotes";




// Entity types for folder creation menu
// Note: SCENE and BEAT are not main folder options - they appear as subfolders under CHAPTER, ACT, and ARC
const ENTITY_TYPES: Array<{ kind: EntityKind; label: string }> = [
  { kind: 'NARRATIVE', label: 'Narrative Timeline Folder' },
  { kind: 'TIMELINE', label: 'General Timeline Folder' },
  { kind: 'ARC', label: 'Arc Folder' },
  { kind: 'ACT', label: 'Act Folder' },
  { kind: 'CHAPTER', label: 'Chapter Folder' },
  { kind: 'EVENT', label: 'Event Folder' },
  { kind: 'CHARACTER', label: 'Character Folder' },
  { kind: 'LOCATION', label: 'Location Folder' },
  { kind: 'NPC', label: 'NPC Folder' },
  { kind: 'ITEM', label: 'Item Folder' },
  { kind: 'CONCEPT', label: 'Concept Folder' },
];

// Color palette for folders
const DEFAULT_COLORS = [
  "#10b981", "#3b82f6", "#8b5cf6", "#ec4899",
  "#f59e0b", "#ef4444", "#14b8a6", "#6366f1"
];

// Reusable rename input component
interface RenameInputProps {
  initialValue: string;
  onSave: (newName: string) => void;
  onCancel: () => void;
  placeholder?: string;
  showEntityHint?: boolean;
  cursorAfterPipe?: boolean; // Position cursor after | for entity prefix
  deleteOnEmpty?: boolean; // Delete note if empty on cancel
  onDelete?: () => void;
}

function RenameInput({
  initialValue,
  onSave,
  onCancel,
  placeholder,
  showEntityHint,
  cursorAfterPipe,
  deleteOnEmpty,
  onDelete,
}: RenameInputProps) {
  const [value, setValue] = React.useState(initialValue);
  const inputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    // Small delay to let any aria-hidden from dropdown menus clear first
    const timer = setTimeout(() => {
      if (inputRef.current) {
        inputRef.current.focus();
        if (cursorAfterPipe && initialValue.includes('|')) {
          // Position cursor after the pipe character
          const pipeIndex = initialValue.indexOf('|') + 1;
          inputRef.current.setSelectionRange(pipeIndex, pipeIndex);
        } else {
          inputRef.current.select();
        }
      }
    }, 50);
    return () => clearTimeout(timer);
  }, [cursorAfterPipe, initialValue]);

  const handleSubmit = () => {
    const trimmed = value.trim();
    // Check if it's just a prefix like "[CHARACTER|" or "[CHARACTER:ALLY|" with no name
    const isEmptyEntityPrefix = /^\[[A-Z_]+(?::[A-Z_]+)?\|?\]?$/.test(trimmed) || trimmed.endsWith('|');

    if (isEmptyEntityPrefix || !trimmed) {
      // Empty or just prefix - cancel or delete
      if (deleteOnEmpty && onDelete) {
        onDelete();
      } else {
        onCancel();
      }
      return;
    }

    // Complete the entity syntax if needed (e.g., "[CHARACTER:ALLY|Jon Snow" → "[CHARACTER:ALLY|Jon Snow]")
    let finalValue = trimmed;
    if (trimmed.match(/^\[[A-Z_]+(?::[A-Z_]+)?\|[^\]]+$/) && !trimmed.endsWith(']')) {
      finalValue = trimmed + ']';
    }

    if (finalValue !== initialValue) {
      onSave(finalValue);
    } else {
      onCancel();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (deleteOnEmpty && onDelete) {
        onDelete();
      } else {
        onCancel();
      }
    }
  };

  return (
    <div className="flex items-center gap-2 px-1 py-1 w-full">
      <Input
        ref={inputRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={handleSubmit}
        placeholder={placeholder || "Enter name..."}
        className="h-7 text-sm flex-1"
      />
      {showEntityHint && (
        <span className="text-[10px] text-muted-foreground shrink-0 whitespace-nowrap">
          [KIND|Name]
        </span>
      )}
    </div>
  );
}

// Entity folder creation dropdown menu
function EntityFolderCreationMenu() {
  const { createFolder } = useJotaiNotes();
  const [isOpen, setIsOpen] = React.useState(false);

  const handleCreateEntityFolder = (kind: EntityKind) => {
    createFolder(`[${kind}]`, undefined, {
      entityKind: kind,
      isTypedRoot: true,
      color: ENTITY_COLORS[kind],
    });
    setIsOpen(false);
  };

  const handleCreateRegularFolder = () => {
    createFolder("New Folder");
    setIsOpen(false);
  };

  return (
    <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="h-5 w-5 p-0" aria-label="Create folder">
          <FolderPlus className="h-3 w-3" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56 bg-popover">
        <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">
          Entity Folders
        </div>
        {ENTITY_TYPES.map(({ kind, label }) => {
          const Icon = ENTITY_ICONS[kind];
          return (
            <DropdownMenuItem
              key={kind}
              textValue={label}
              onClick={() => handleCreateEntityFolder(kind)}
              className="gap-2"
            >
              <Icon className="h-4 w-4" style={{ color: ENTITY_COLORS[kind] }} />
              <span>{label}</span>
              <span
                className="ml-auto text-[10px] px-1.5 py-0.5 rounded font-medium"
                style={{
                  backgroundColor: `${ENTITY_COLORS[kind]}20`,
                  color: ENTITY_COLORS[kind],
                }}
              >
                {kind}
              </span>
            </DropdownMenuItem>
          );
        })}

        <DropdownMenuSeparator />

        <DropdownMenuItem onClick={handleCreateRegularFolder} className="gap-2">
          <FolderIcon className="h-4 w-4" />
          <span>Regular Folder</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}


// Settings Button Component (Memoized)
const SettingsButton = memo(function SettingsButton() {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpen = useCallback(() => setIsOpen(true), []);
  const handleClose = useCallback((value: boolean) => setIsOpen(value), []);

  return (
    <>
      <Button
        variant="outline"
        size="sm"
        className="h-8 w-8 p-0 shrink-0"
        onClick={handleOpen}
        aria-label="Settings"
      >
        <Settings className="h-4 w-4" />
      </Button>
      <SettingsPanel open={isOpen} onOpenChange={handleClose} />
    </>
  );
});

// Blueprint Hub Button Component (Memoized)
const BlueprintHubButton = memo(function BlueprintHubButton() {
  const { toggleHub, isHubOpen } = useBlueprintHub();

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={toggleHub}
      className={cn(
        "flex-1 gap-2 h-8",
        isHubOpen && "bg-accent text-accent-foreground border-accent-foreground/20"
      )}
    >
      <Boxes className="h-4 w-4" />
      Blueprint Hub
    </Button>
  );
});

interface NoteItemProps {
  note: Note;
  depth?: number;
  folderColor?: string;
  autoRename?: boolean;
  onRenameComplete?: () => void;
}

function NoteItem({ note, depth = 0, folderColor, autoRename, onRenameComplete }: NoteItemProps) {
  const { selectNote, updateNote, deleteNote, state, getEntityNote } = useJotaiNotes();
  const [isHovered, setIsHovered] = React.useState(false);
  const [isRenaming, setIsRenaming] = React.useState(autoRename || false);
  const isActive = state.selectedNoteId === note.id;

  // Start renaming when autoRename becomes true
  React.useEffect(() => {
    if (autoRename) {
      setIsRenaming(true);
    }
  }, [autoRename]);

  // Parse entity info for display
  const displayName = getDisplayName(note.title);
  const EntityIcon = note.isEntity && note.entityKind ? ENTITY_ICONS[note.entityKind] : FileText;
  const entityColor = note.isEntity && note.entityKind ? ENTITY_COLORS[note.entityKind] : undefined;

  // Check for kind mismatch with folder
  const folder = note.folderId ? state.folders.find(f => f.id === note.folderId) : undefined;
  const folderKind = folder?.entityKind || folder?.inherited_kind;
  const hasKindMismatch = note.isEntity && note.entityKind && folderKind && note.entityKind !== folderKind;

  const handleCopyLink = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const url = `${window.location.origin}/note/${note.id}`;
    navigator.clipboard.writeText(url);
  };

  const handleToggleFavorite = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    updateNote(note.id, { favorite: note.favorite === 1 ? 0 : 1 });
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    deleteNote(note.id);
  };

  const handleRename = (newTitle: string) => {
    const parsed = parseEntityFromTitle(newTitle);
    updateNote(note.id, {
      title: newTitle,
      entityKind: parsed?.kind,
      entityLabel: parsed?.label,
      isEntity: parsed !== null && parsed.label !== undefined,
    });
    setIsRenaming(false);
    onRenameComplete?.();
  };

  const handleCancelRename = () => {
    setIsRenaming(false);
    onRenameComplete?.();
  };

  const handleDeleteNewNote = () => {
    deleteNote(note.id);
    onRenameComplete?.();
  };

  const handleStartRename = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsRenaming(true);
  };

  // Check if this is an auto-created entity note (has prefix like "[CHARACTER|" or "[CHARACTER:ALLY|")
  const isAutoCreatedEntity = Boolean(autoRename && /^\[[A-Z_]+(?::[A-Z_]+)?\|$/.test(note.title));

  return (
    <div className="relative w-full">
      {/* Tree line connector - inherits folder color */}
      {depth > 0 && folderColor && (
        <>
          <div
            className="absolute top-0 bottom-0 w-[2px] opacity-40 pointer-events-none"
            style={{
              left: `${(depth - 1) * 20 + 10}px`,
              borderLeft: `2px solid ${folderColor}`,
            }}
          />
          <div
            className="absolute top-[18px] w-3 h-[2px] opacity-40 pointer-events-none"
            style={{
              left: `${(depth - 1) * 20 + 10}px`,
              backgroundColor: folderColor,
            }}
          />
        </>
      )}

      <SidebarMenuItem
        className="relative z-10"
        style={{ paddingLeft: `${depth * 20}px` }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        {isRenaming ? (
          <div className="flex items-center gap-1 w-full">
            <div className="h-6 w-6" />
            <RenameInput
              initialValue={note.title}
              onSave={handleRename}
              onCancel={handleCancelRename}
              placeholder="Enter entity name..."
              showEntityHint={!!note.isEntity || isAutoCreatedEntity}
              cursorAfterPipe={isAutoCreatedEntity}
              deleteOnEmpty={isAutoCreatedEntity}
              onDelete={handleDeleteNewNote}
            />
          </div>
        ) : (
          <div className="flex items-center gap-1 w-full">
            <div className="h-6 w-6" /> {/* Spacer for alignment with folders */}

            <SidebarMenuButton
              onClick={() => selectNote(note.id)}
              isActive={isActive}
              className={cn("flex-1 justify-start gap-2 h-8 min-w-0")}
            >
              <EntityIcon
                className="h-4 w-4 shrink-0"
                style={entityColor ? { color: entityColor } : undefined}
              />
              <span className="truncate text-sm">{displayName || "Untitled Note"}</span>
              {/* Entity badge with optional subtype */}
              {note.isEntity && note.entityKind && (
                <span
                  className="text-[10px] px-1.5 py-0.5 rounded font-medium shrink-0"
                  style={{
                    backgroundColor: `${ENTITY_COLORS[note.entityKind]}20`,
                    color: ENTITY_COLORS[note.entityKind],
                  }}
                >
                  {note.entitySubtype ? `${note.entityKind}:${note.entitySubtype}` : note.entityKind}
                </span>
              )}
              {/* Kind mismatch warning */}
              {hasKindMismatch && (
                <span title="Entity kind does not match folder">
                  <AlertTriangle className="h-3 w-3 shrink-0 text-amber-500" />
                </span>
              )}
              {note.favorite && <Star className="h-3 w-3 shrink-0 fill-yellow-400 text-yellow-400 ml-auto" />}
            </SidebarMenuButton>

            {/* Action button - visible on hover */}
            <div className={cn("flex items-center shrink-0 transition-opacity", !isHovered && "opacity-0")}>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-6 w-6 p-0" aria-label="Note options">
                    <MoreVertical className="h-3 w-3" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-56 bg-popover">
                  <DropdownMenuItem onClick={handleStartRename}>
                    <Pencil className="mr-2 h-4 w-4" />
                    Rename note
                  </DropdownMenuItem>

                  <DropdownMenuSeparator />

                  <DropdownMenuItem onClick={handleToggleFavorite}>
                    {note.favorite ? (
                      <>
                        <StarOff className="mr-2 h-4 w-4" />
                        Remove from favorites
                      </>
                    ) : (
                      <>
                        <Star className="mr-2 h-4 w-4" />
                        Add to favorites
                      </>
                    )}
                  </DropdownMenuItem>

                  <DropdownMenuItem onClick={handleCopyLink}>
                    <LinkIcon className="mr-2 h-4 w-4" />
                    Copy link
                  </DropdownMenuItem>

                  <DropdownMenuSeparator />

                  <DropdownMenuItem onClick={handleDelete} className="text-destructive focus:text-destructive">
                    <X className="mr-2 h-4 w-4" />
                    Delete note
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>
        )}
      </SidebarMenuItem>
    </div>
  );
}

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const {
    state,
    folderTree,
    globalNotes,
    favoriteNotes,
    createNote,
    createFolder,
    setSearchQuery,
    selectNote,
  } = useJotaiNotes();

  // ResoRank-powered search with debounce
  const { results: searchResults, isReady: searchReady, isIndexing } = useResoRankSearchWithDebounce(
    state.notes,
    state.searchQuery,
    { debounceMs: 150, minLength: 2, limit: 20 }
  );
  const showRankedResults = state.searchQuery.trim().length >= 2 && searchResults.length > 0;

  // No longer needed here as we use memoized button
  // const { toggleHub, isHubOpen } = useBlueprintHub();

  const [activeTab, setActiveTab] = React.useState(() => {
    return localStorage.getItem('sidebar-tab') || 'folders';
  });

  const handleTabChange = (value: string) => {
    setActiveTab(value);
    localStorage.setItem('sidebar-tab', value);
  };

  // ✅ Memoize footer content
  const footerContent = useMemo(() => (
    <>
      <div className="flex items-center gap-2 mb-3">
        <BlueprintHubButton />
        <SettingsButton />
      </div>
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>{state.notes.length} notes</span>
        {state.isSaving ? (
          <span className="flex items-center gap-1 animate-pulse">
            <Clock className="h-3 w-3" />
            Saving...
          </span>
        ) : state.lastSaved ? (
          <span className="flex items-center gap-1">
            <Check className="h-3 w-3 text-green-500" />
            Saved
          </span>
        ) : null}
      </div>
    </>
  ), [state.notes.length, state.isSaving, state.lastSaved]);

  return (
    <Sidebar className="border-r border-sidebar-border" {...props}>
      <SidebarHeader className="p-4">
        <div className="flex items-center gap-2 mb-4">
          <GraphLogo className="w-10 h-10" />
          <span className="font-serif font-semibold text-lg text-sidebar-foreground tracking-tight">GraphAIte</span>
        </div>

        <Select value={activeTab} onValueChange={handleTabChange}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder="Select view" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="folders">
              <div className="flex items-center gap-2">
                <FolderIcon className="h-4 w-4" />
                <span>Folders</span>
              </div>
            </SelectItem>
            {/* DEPRECATED: TypeScript semantic search tab
            <SelectItem value="semantic">
              <div className="flex items-center gap-2">
                <Search className="h-4 w-4" />
                <span>Semantic</span>
              </div>
            </SelectItem>
            */}
            <SelectItem value="entities">
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4" />
                <span>Entities</span>
              </div>
            </SelectItem>
            <SelectItem value="rust">
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-teal-500" />
                <span className="text-teal-400">Rust</span>
              </div>
            </SelectItem>
          </SelectContent>
        </Select>
      </SidebarHeader>

      <SidebarContent className="gap-0 px-2">
        {activeTab === 'folders' ? (
          <>
            <div className="relative mb-4">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input
                placeholder="Search notes... (Ctrl+K)"
                className="pl-9 bg-sidebar-accent border-0 focus-visible:ring-1 focus-visible:ring-sidebar-ring"
                value={state.searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
              {/* ResoRank indexing indicator */}
              {isIndexing && (
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-muted-foreground animate-pulse">
                  Indexing...
                </span>
              )}
            </div>

            <div className="space-y-4">
              {/* Favorites */}
              {favoriteNotes.length > 0 && (
                <SidebarGroup>
                  <SidebarGroupLabel className="text-xs font-medium text-muted-foreground uppercase tracking-wider px-2">
                    Favorites
                  </SidebarGroupLabel>
                  <SidebarGroupContent>
                    <SidebarMenu>
                      {favoriteNotes.map((note) => (
                        <NoteItem key={note.id} note={note} depth={0} />
                      ))}
                    </SidebarMenu>
                  </SidebarGroupContent>
                </SidebarGroup>
              )}

              {/* Quick Notes (global notes without folder) */}
              {/* Unified Tree View (Folders & Quick Notes) */}
              <SidebarGroup>
                <div className="flex items-center justify-between px-2 mb-1">
                  <SidebarGroupLabel className="text-xs font-medium text-muted-foreground uppercase tracking-wider p-0">
                    Files
                  </SidebarGroupLabel>
                  <div className="flex items-center gap-1">
                    <Button variant="ghost" size="icon" className="h-5 w-5 p-0" onClick={() => createNote()} aria-label="Create quick note">
                      <Plus className="h-3 w-3" />
                    </Button>
                    <EntityFolderCreationMenu />
                    <NetworkFolderCreationMenu />
                  </div>
                </div>
                <SidebarGroupContent>
                  {showRankedResults ? (
                    <RankedSearchResults
                      results={searchResults.map(r => ({
                        doc_id: r.docId,
                        score: r.score,
                      }))}
                      onSelect={selectNote}
                      className="mt-2"
                    />
                  ) : (
                    <ArboristTreeView
                      searchTerm={state.searchQuery}
                      className="mt-2"
                    />
                  )}
                </SidebarGroupContent>
              </SidebarGroup>
            </div>
          </>
        ) : activeTab === 'entities' ? (
          <EntitiesPanel />
        ) : activeTab === 'rust' ? (
          <RustSearchPanel />
        ) : null}
      </SidebarContent>

      <SidebarFooter className="p-4 border-t border-sidebar-border">
        {footerContent}
      </SidebarFooter>

      <SidebarRail />
    </Sidebar >
  );
}

export default AppSidebar;
