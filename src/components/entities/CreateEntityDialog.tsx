import { useState } from 'react';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from '@/components/ui/dialog';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Plus, Loader2 } from 'lucide-react';
import { ENTITY_KINDS, ENTITY_COLORS, type EntityKind } from '@/lib/types/entityTypes';
import { smartGraphRegistry } from '@/lib/tauri';

interface CreateEntityDialogProps {
    onEntityCreated?: () => void;
    currentNoteId?: string;
}

export function CreateEntityDialog({ onEntityCreated, currentNoteId }: CreateEntityDialogProps) {
    const [open, setOpen] = useState(false);
    const [kind, setKind] = useState<EntityKind>('CHARACTER');
    const [label, setLabel] = useState('');
    const [isCreating, setIsCreating] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleCreate = async () => {
        if (!label.trim()) {
            setError('Entity label is required');
            return;
        }

        setIsCreating(true);
        setError(null);

        try {
            await smartGraphRegistry.registerEntity(
                label.trim(),
                kind,
                currentNoteId || 'manual',
                { source: 'user' }
            );

            setLabel('');
            setKind('CHARACTER');
            setOpen(false);
            onEntityCreated?.();
        } catch (err) {
            console.error('Failed to create entity:', err);
            setError(err instanceof Error ? err.message : 'Failed to create entity');
        } finally {
            setIsCreating(false);
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !isCreating) {
            handleCreate();
        }
    };

    return (
        <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
                <Button variant="outline" size="sm" className="gap-2 w-full">
                    <Plus className="h-4 w-4" />
                    Create Entity
                </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-[425px]">
                <DialogHeader>
                    <DialogTitle>Create New Entity</DialogTitle>
                    <DialogDescription>
                        Register a new entity to track across your notes.
                    </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                    <div className="grid gap-2">
                        <Label htmlFor="entity-kind">Type</Label>
                        <Select value={kind} onValueChange={(v) => setKind(v as EntityKind)}>
                            <SelectTrigger id="entity-kind">
                                <SelectValue placeholder="Select entity type" />
                            </SelectTrigger>
                            <SelectContent>
                                {ENTITY_KINDS.map((k) => (
                                    <SelectItem key={k} value={k}>
                                        <div className="flex items-center gap-2">
                                            <div
                                                className="w-3 h-3 rounded-full"
                                                style={{
                                                    backgroundColor: ENTITY_COLORS[k]
                                                        ? `hsl(var(--entity-${k.toLowerCase().replace('_', '-')}))`
                                                        : '#6b7280'
                                                }}
                                            />
                                            {k}
                                        </div>
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="grid gap-2">
                        <Label htmlFor="entity-label">Label</Label>
                        <Input
                            id="entity-label"
                            placeholder="e.g., Jon Snow, Rivendell, The One Ring"
                            value={label}
                            onChange={(e) => setLabel(e.target.value)}
                            onKeyDown={handleKeyDown}
                            autoFocus
                        />
                    </div>
                    {error && (
                        <p className="text-sm text-destructive">{error}</p>
                    )}
                </div>
                <DialogFooter>
                    <Button variant="ghost" onClick={() => setOpen(false)}>
                        Cancel
                    </Button>
                    <Button onClick={handleCreate} disabled={isCreating || !label.trim()}>
                        {isCreating ? (
                            <>
                                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                                Creating...
                            </>
                        ) : (
                            'Create Entity'
                        )}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}

export default CreateEntityDialog;
