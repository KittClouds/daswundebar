
import { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import { useSidebar } from '@/components/ui/sidebar';
import type { Note } from '@/types/noteTypes';
import { getDisplayName } from '@/lib/utils/titleParser';
import { ENTITY_ICONS, ENTITY_COLORS } from '@/lib/types/entityTypes';

interface NoteTabsProps {
    className?: string;
}

export function NoteTabs({ className }: NoteTabsProps) {
    const {
        state: { openNoteIds, selectedNoteId, notes },
        selectNote,
        closeNote
    } = useJotaiNotes();
    const scrollContainerRef = useRef<HTMLDivElement>(null);

    // Map IDs to Note objects
    const openNotes = openNoteIds
        .map(id => notes.find(n => n.id === id))
        .filter((n): n is Note => !!n);

    // Auto-scroll to active tab
    useEffect(() => {
        if (selectedNoteId && scrollContainerRef.current) {
            const activeTab = scrollContainerRef.current.querySelector('[data-state="active"]');
            if (activeTab) {
                activeTab.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
            }
        }
    }, [selectedNoteId, openNoteIds]);

    if (openNotes.length === 0) return null;

    return (
        <div
            ref={scrollContainerRef}
            className={cn("flex flex-1 items-center overflow-x-auto no-scrollbar gap-1 px-2 h-9", className)}
            onWheel={(e) => {
                if (e.deltaY !== 0) {
                    e.preventDefault();
                    scrollContainerRef.current!.scrollLeft += e.deltaY;
                }
            }}
        >
            {openNotes.map((note) => {
                const isActive = note.id === selectedNoteId;
                const Icon = note.isEntity && note.entityKind ? ENTITY_ICONS[note.entityKind] : null;
                const color = note.isEntity && note.entityKind ? ENTITY_COLORS[note.entityKind] : undefined;

                return (
                    <div
                        key={note.id}
                        data-state={isActive ? 'active' : 'inactive'}
                        className={cn(
                            "group flex items-center gap-2 px-3 py-1.5 h-8 rounded-t-md text-xs font-medium cursor-pointer transition-colors min-w-[120px] max-w-[200px] border-l border-r border-t border-transparent select-none shrink-0",
                            isActive
                                ? "bg-background border-border text-foreground relative z-10 -mb-[1px] border-b-0"
                                : "bg-muted/30 text-muted-foreground hover:bg-muted/50 border-transparent hover:text-foreground"
                        )}
                        onClick={() => selectNote(note.id)}
                        title={note.title}
                    >
                        {Icon && (
                            <Icon className="h-3.5 w-3.5 shrink-0" style={{ color }} />
                        )}
                        <span className="truncate flex-1">{getDisplayName(note.title)}</span>

                        <Button
                            variant="ghost"
                            size="icon"
                            className={cn(
                                "h-4 w-4 rounded-full hover:bg-muted-foreground/20 opacity-0 group-hover:opacity-100 transition-opacity",
                                isActive && "opacity-100"
                            )}
                            onClick={(e) => {
                                e.stopPropagation();
                                closeNote(note.id);
                            }}
                        >
                            <X className="h-3 w-3" />
                        </Button>
                    </div>
                );
            })}
        </div>
    );
}
