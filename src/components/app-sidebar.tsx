import { memo, useCallback, useMemo, useState } from 'react';
import * as React from "react";
import { useNavigate } from 'react-router-dom';
import {
    Folder as FolderIcon,
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
    Sparkles,
    Boxes,
    Settings,
    Cpu,
    Brain,
    AlertTriangle,
    Pencil,
    ArchiveX,
    Trash2,
    Network,
    CalendarClock,
    BookOpen,
    Type,
    ListTree,
    Map,
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
import { NerPanel } from "@/components/search/NerPanel";
import { RankedSearchResults } from "@/components/search/RankedSearchResults";
import { EntitiesPanel } from "@/components/EntitiesPanel";
import { SettingsPanel } from "@/components/settings";
import { ArboristTreeView } from '@/components/tree/ArboristTreeView';
import '@/styles/arborist-overrides.css';
import { NetworkFolderCreationMenu } from '@/components/network/NetworkFolderCreation';
import { useResoRankSearchWithDebounce } from '@/hooks/useResoRankSearch';
import { GraphLogo } from "@/components/ui/GraphLogo";
import { SidebarGraph } from '@/components/graph/SidebarGraph';

import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupContent,
    SidebarGroupLabel,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarRail,
    SidebarHeader,
    SidebarFooter,
    SidebarInput,
    useSidebar,
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
import type { Note } from "@/types/noteTypes";
import { ENTITY_COLORS, EntityKind, ENTITY_ICONS } from "@/lib/types/entityTypes";
import { getDisplayName, parseEntityFromTitle } from "@/lib/utils/titleParser";
import { useBlueprintHub } from "@/features/blueprint-hub/hooks/useBlueprintHub";
import { useJotaiNotes } from "@/hooks/useJotaiNotes";
import { NavUser } from "@/components/nav-user";

// Entity types for folder creation menu
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


interface NoteItemProps {
    note: Note;
    depth?: number;
    folderColor?: string;
    autoRename?: boolean;
    onRenameComplete?: () => void;
}

function NoteItem({ note, depth = 0, folderColor, autoRename, onRenameComplete }: NoteItemProps) {
    const { selectNote, updateNote, deleteNote, state } = useJotaiNotes();
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

// User data for NavUser
const DATA = {
    user: {
        name: "Architect",
        email: "architect@graphaite.dev",
        avatar: "/avatars/architect.jpg",
    },
}

interface AppSidebarProps extends React.ComponentProps<typeof Sidebar> {
    toolbarVisible?: boolean;
    onToolbarToggle?: (visible: boolean) => void;
}

export function AppSidebar({ toolbarVisible, onToolbarToggle, ...props }: AppSidebarProps) {
    const {
        state,
        favoriteNotes,
        createNote,
        setSearchQuery,
        selectNote,
    } = useJotaiNotes();
    const { setOpen } = useSidebar();
    const navigate = useNavigate();

    // ResoRank-powered search with debounce
    const { results: searchResults, isIndexing } = useResoRankSearchWithDebounce(
        state.notes,
        state.searchQuery,
        { debounceMs: 150, minLength: 2, limit: 20 }
    );
    const showRankedResults = state.searchQuery.trim().length >= 2 && searchResults.length > 0;

    const [activeTab, setActiveTab] = React.useState(() => {
        return localStorage.getItem('sidebar-tab') || 'folders';
    });
    const [viewMode, setViewMode] = useState<'tree' | 'graph'>('tree');

    const handleTabChange = (value: string) => {
        setActiveTab(value);
        localStorage.setItem('sidebar-tab', value);
        setOpen(true);
    };

    return (
        <Sidebar
            collapsible="icon"
            className="border-r border-sidebar-border overflow-hidden"
            {...props}
        >
            <SidebarContent className="flex flex-row h-full overflow-hidden p-0 gap-0">

                {/* --- LEFT RAIL (PERSISTENT ICONS) --- */}
                <div className="flex flex-col items-center w-[3rem] border-r border-sidebar-border bg-sidebar h-full py-2 z-20">
                    <div className="mb-2">
                        <GraphLogo className="w-8 h-8" />
                    </div>

                    <div className="flex flex-col gap-1 w-full px-1 items-center flex-1 overflow-y-auto no-scrollbar">
                        <RailButton
                            icon={FolderIcon}
                            label="Folders"
                            isActive={activeTab === 'folders'}
                            onClick={() => handleTabChange('folders')}
                        />
                        <RailButton
                            icon={Sparkles}
                            label="Entities"
                            isActive={activeTab === 'entities'}
                            onClick={() => handleTabChange('entities')}
                        />
                        <RailButton
                            icon={Cpu}
                            label="Rust Inference"
                            isActive={activeTab === 'rust'}
                            onClick={() => handleTabChange('rust')}
                        />
                        <RailButton
                            icon={Brain}
                            label="NER"
                            isActive={activeTab === 'ner'}
                            onClick={() => handleTabChange('ner')}
                        />

                        <div className="h-px w-6 bg-sidebar-border my-2 shrink-0" />

                        <RailButton
                            icon={Network}
                            label="Graph Explorer"
                            isActive={false}
                            onClick={() => navigate('/graph')}
                        />
                        <RailButton
                            icon={CalendarClock}
                            label="Fantasy Calendar"
                            isActive={false}
                            onClick={() => navigate('/calendar')}
                        />
                        <RailButton
                            icon={BookOpen}
                            label="Wiki"
                            isActive={false}
                            onClick={() => navigate('/wiki')}
                        />

                        <div className="flex-1" />

                        {onToolbarToggle && (
                            <RailButton
                                icon={Type}
                                label={toolbarVisible ? "Hide Toolbar" : "Show Toolbar"}
                                isActive={toolbarVisible || false}
                                onClick={() => onToolbarToggle(!toolbarVisible)}
                            />
                        )}
                    </div>
                </div>


                {/* --- RIGHT PANEL (COLLAPSIBLE CONTENT) --- */}
                <div className="flex flex-col flex-1 h-full min-w-0 bg-sidebar group-data-[collapsible=icon]:hidden w-64 transition-all duration-300 ease-in-out">
                    {/* Panel Header */}
                    <div className="h-14 flex items-center justify-between px-4 border-b border-sidebar-border shrink-0">
                        <span className="font-semibold text-sm">
                            {activeTab === 'folders' && 'Folders'}
                            {activeTab === 'entities' && 'Entities'}
                            {activeTab === 'rust' && 'Rust Inference'}
                            {activeTab === 'ner' && 'NER'}
                        </span>

                        {/* Contextual Actions (Folders Only) */}
                        {activeTab === 'folders' && (
                            <div className="flex items-center gap-1">
                                <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-6 w-6"
                                    onClick={() => setViewMode(v => v === 'tree' ? 'graph' : 'tree')}
                                    title={viewMode === 'tree' ? "Switch to Graph View" : "Switch to Tree View"}
                                >
                                    {viewMode === 'tree' ? <Network className="h-4 w-4" /> : <ListTree className="h-4 w-4" />}
                                </Button>
                                <div className="w-px h-3 bg-border mx-1" />
                                <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => createNote()} title="New Note">
                                    <Plus className="h-4 w-4" />
                                </Button>
                                <EntityFolderCreationMenu />
                                <NetworkFolderCreationMenu />
                            </div>
                        )}
                    </div>

                    {/* Panel Content */}
                    <div className="flex-1 overflow-y-auto overflow-x-hidden">
                        {activeTab === 'folders' ? (
                            <div className="p-2 space-y-4">
                                <div className="relative mb-2">
                                    <SidebarInput
                                        placeholder="Search... (Ctrl+K)"
                                        value={state.searchQuery}
                                        onChange={(e) => setSearchQuery(e.target.value)}
                                        className="h-8"
                                    />
                                    {isIndexing && (
                                        <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-muted-foreground animate-pulse">
                                            Indexing
                                        </span>
                                    )}
                                </div>

                                {/* Favorites */}
                                {favoriteNotes.length > 0 && (
                                    <SidebarGroup className="p-0">
                                        <SidebarGroupLabel className="px-2 text-xs">Favorites</SidebarGroupLabel>
                                        <SidebarGroupContent>
                                            <SidebarMenu>
                                                {favoriteNotes.map((note) => (
                                                    <NoteItem key={note.id} note={note} depth={0} />
                                                ))}
                                            </SidebarMenu>
                                        </SidebarGroupContent>
                                    </SidebarGroup>
                                )}

                                {/* Tree / Search Results / Graph */}
                                <div className="mt-2">
                                    {showRankedResults ? (
                                        <RankedSearchResults results={searchResults.map(r => ({ doc_id: r.docId, score: r.score }))} onSelect={selectNote} />
                                    ) : viewMode === 'graph' ? (
                                        <SidebarGraph className="h-[60vh] w-full border border-sidebar-border rounded-md overflow-hidden" />
                                    ) : (
                                        <ArboristTreeView searchTerm={state.searchQuery} />
                                    )}
                                </div>
                            </div>
                        ) : activeTab === 'entities' ? (
                            <EntitiesPanel />
                        ) : activeTab === 'rust' ? (
                            <RustSearchPanel />
                        ) : activeTab === 'ner' ? (
                            <NerPanel />
                        ) : null}
                    </div>

                    {/* Panel Footer */}
                    <div className="border-t border-sidebar-border p-0">
                        <NavUser user={DATA.user} />
                    </div>
                </div>

            </SidebarContent>
            <SidebarRail />
        </Sidebar >
    );
}

function RailButton({ icon: Icon, label, isActive, onClick }: { icon: any, label: string, isActive: boolean, onClick: () => void }) {
    return (
        <Button
            variant="ghost"
            size="icon"
            onClick={onClick}
            className={cn(
                "h-9 w-9 rounded-lg transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
                isActive && "bg-sidebar-accent text-sidebar-accent-foreground"
            )}
            title={label}
        >
            <Icon className="h-5 w-5" />
            <span className="sr-only">{label}</span>
        </Button>
    )
}

export default AppSidebar;
