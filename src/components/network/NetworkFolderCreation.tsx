/**
 * Network Folder Creation Components
 * 
 * UI components for creating network-typed folders:
 * - NetworkFolderCreationMenu: Dropdown to select network type and create folder
 * - NetworkMemberCreationMenu: Context-aware member creation inside network folders
 */

import * as React from 'react';
import {
    Network,
    Users,
    Building,
    Flag,
    Handshake,
    Hammer,
    Heart,
    Swords,
    Wrench,
    Plus,
    UserPlus,
    ChevronRight,
} from 'lucide-react';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
    DropdownMenuSeparator,
    DropdownMenuSub,
    DropdownMenuSubContent,
    DropdownMenuSubTrigger,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import { generateId } from '@/lib/utils/ids';
import { cn } from '@/lib/utils';
import type { EntityKind } from '@/lib/types/entityTypes';
import { ENTITY_COLORS, ENTITY_ICONS } from '@/lib/types/entityTypes';
import type { NetworkKind, NetworkSchema, NetworkRelationshipDef } from '@/lib/networks/types';
import { NETWORK_COLORS } from '@/lib/networks/types';
import {
    BUILTIN_SCHEMAS,
    getDefaultSchemaForKind,
    FAMILY_SCHEMA,
    ORG_SCHEMA,
    FACTION_SCHEMA,
    ALLIANCE_SCHEMA,
    GUILD_SCHEMA,
} from '@/lib/networks/schemas';
import { saveNetworkInstance } from '@/lib/networks/storage';
import type { NetworkInstance } from '@/lib/networks/types';

/**
 * Icon mapping for network kinds
 */
const NETWORK_ICONS: Record<NetworkKind, React.ComponentType<{ className?: string; style?: React.CSSProperties }>> = {
    FAMILY: Users,
    ORGANIZATION: Building,
    FACTION: Flag,
    ALLIANCE: Handshake,
    GUILD: Hammer,
    FRIENDSHIP: Heart,
    RIVALRY: Swords,
    CUSTOM: Wrench,
};

/**
 * Network type options for the creation menu
 */
const NETWORK_TYPE_OPTIONS: Array<{
    kind: NetworkKind;
    label: string;
    description: string;
    schema: NetworkSchema;
}> = [
        {
            kind: 'FAMILY',
            label: 'Family Tree',
            description: 'Biological and adoptive family relationships',
            schema: FAMILY_SCHEMA,
        },
        {
            kind: 'ORGANIZATION',
            label: 'Organization',
            description: 'Corporate, military, or government hierarchy',
            schema: ORG_SCHEMA,
        },
        {
            kind: 'FACTION',
            label: 'Faction Network',
            description: 'Political factions, gangs, or tribal groups',
            schema: FACTION_SCHEMA,
        },
        {
            kind: 'ALLIANCE',
            label: 'Alliance Network',
            description: 'Strategic alliances and treaties',
            schema: ALLIANCE_SCHEMA,
        },
        {
            kind: 'GUILD',
            label: 'Guild Network',
            description: 'Trade guilds and craft associations',
            schema: GUILD_SCHEMA,
        },
    ];

interface NetworkFolderCreationMenuProps {
    className?: string;
    onNetworkCreated?: (network: NetworkInstance, folderId: string) => void;
}

/**
 * Dropdown menu for creating network-typed folders
 */
export function NetworkFolderCreationMenu({
    className,
    onNetworkCreated
}: NetworkFolderCreationMenuProps) {
    const { createFolder } = useJotaiNotes();
    const [isOpen, setIsOpen] = React.useState(false);
    const [dialogOpen, setDialogOpen] = React.useState(false);
    const [selectedKind, setSelectedKind] = React.useState<NetworkKind | null>(null);
    const [networkName, setNetworkName] = React.useState('');
    const [isCreating, setIsCreating] = React.useState(false);

    const handleSelectNetworkType = (kind: NetworkKind) => {
        setSelectedKind(kind);
        setNetworkName('');
        setDialogOpen(true);
        setIsOpen(false);
    };

    const handleCreateNetwork = async () => {
        if (!selectedKind || !networkName.trim()) return;

        setIsCreating(true);

        try {
            const schema = getDefaultSchemaForKind(selectedKind);
            if (!schema) {
                console.error('No schema found for kind:', selectedKind);
                return;
            }

            // Create folder with network syntax: [NETWORK:KIND|Name]
            const folderName = `[NETWORK:${selectedKind}|${networkName.trim()}]`;
            const folder = await createFolder(folderName, undefined, {
                entityKind: 'NETWORK',
                entitySubtype: selectedKind,
                entityLabel: networkName.trim(),
                isTypedRoot: true,
                color: NETWORK_COLORS[selectedKind],
            });

            // Create network instance
            const network: NetworkInstance = {
                id: generateId(),
                name: networkName.trim(),
                schemaId: schema.id,
                rootFolderId: folder.id,
                entityIds: [],
                namespace: 'default',
                description: `${schema.name}: ${networkName.trim()}`,
                createdAt: new Date(),
                updatedAt: new Date(),
            };

            await saveNetworkInstance(network);

            onNetworkCreated?.(network, folder.id);
            setDialogOpen(false);
            setSelectedKind(null);
            setNetworkName('');
        } catch (error) {
            console.error('Failed to create network:', error);
        } finally {
            setIsCreating(false);
        }
    };

    const selectedOption = NETWORK_TYPE_OPTIONS.find(o => o.kind === selectedKind);

    return (
        <>
            <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        size="icon"
                        className={cn("h-6 w-6 p-0", className)}
                        aria-label="Create network folder"
                    >
                        <Network className="h-4 w-4" />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-64 bg-popover">
                    <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">
                        Network Folders
                    </div>
                    <DropdownMenuSeparator />

                    {NETWORK_TYPE_OPTIONS.map(({ kind, label, description }) => {
                        const Icon = NETWORK_ICONS[kind];
                        return (
                            <DropdownMenuItem
                                key={kind}
                                onClick={() => handleSelectNetworkType(kind)}
                                className="flex items-start gap-3 py-2"
                            >
                                <Icon
                                    className="h-4 w-4 mt-0.5 shrink-0"
                                    style={{ color: NETWORK_COLORS[kind] }}
                                />
                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2">
                                        <span className="font-medium">{label}</span>
                                        <span
                                            className="text-[10px] px-1.5 py-0.5 rounded font-medium"
                                            style={{
                                                backgroundColor: `${NETWORK_COLORS[kind]}20`,
                                                color: NETWORK_COLORS[kind],
                                            }}
                                        >
                                            {kind}
                                        </span>
                                    </div>
                                    <p className="text-xs text-muted-foreground mt-0.5 truncate">
                                        {description}
                                    </p>
                                </div>
                            </DropdownMenuItem>
                        );
                    })}
                </DropdownMenuContent>
            </DropdownMenu>

            {/* Network Name Dialog */}
            <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
                <DialogContent className="sm:max-w-md">
                    <DialogHeader>
                        <DialogTitle className="flex items-center gap-2">
                            {selectedOption && (
                                <>
                                    {React.createElement(NETWORK_ICONS[selectedOption.kind], {
                                        className: "h-5 w-5",
                                        style: { color: NETWORK_COLORS[selectedOption.kind] },
                                    })}
                                    Create {selectedOption.label}
                                </>
                            )}
                        </DialogTitle>
                        <DialogDescription>
                            {selectedOption?.description}
                        </DialogDescription>
                    </DialogHeader>

                    <div className="space-y-4 py-4">
                        <div className="space-y-2">
                            <Label htmlFor="network-name">Network Name</Label>
                            <Input
                                id="network-name"
                                placeholder={`e.g., "${selectedKind === 'FAMILY' ? 'Stark Family' : selectedKind === 'ORGANIZATION' ? 'Imperial Army' : 'The Alliance'}"`}
                                value={networkName}
                                onChange={(e) => setNetworkName(e.target.value)}
                                onKeyDown={(e) => {
                                    if (e.key === 'Enter' && networkName.trim()) {
                                        handleCreateNetwork();
                                    }
                                }}
                                autoFocus
                            />
                        </div>

                        {selectedOption && (
                            <div className="text-sm text-muted-foreground">
                                <p className="font-medium mb-1">Available relationships:</p>
                                <div className="flex flex-wrap gap-1">
                                    {selectedOption.schema.relationships.slice(0, 6).map(rel => (
                                        <span
                                            key={rel.code}
                                            className="text-[10px] px-1.5 py-0.5 rounded bg-muted"
                                        >
                                            {rel.label}
                                        </span>
                                    ))}
                                    {selectedOption.schema.relationships.length > 6 && (
                                        <span className="text-[10px] px-1.5 py-0.5 text-muted-foreground">
                                            +{selectedOption.schema.relationships.length - 6} more
                                        </span>
                                    )}
                                </div>
                            </div>
                        )}
                    </div>

                    <DialogFooter>
                        <Button variant="outline" onClick={() => setDialogOpen(false)}>
                            Cancel
                        </Button>
                        <Button
                            onClick={handleCreateNetwork}
                            disabled={!networkName.trim() || isCreating}
                            style={{
                                backgroundColor: selectedKind ? NETWORK_COLORS[selectedKind] : undefined,
                            }}
                        >
                            {isCreating ? 'Creating...' : 'Create Network'}
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </>
    );
}

// ===== NETWORK MEMBER CREATION MENU =====

interface NetworkMemberCreationMenuProps {
    networkId: string;
    schema: NetworkSchema;
    currentEntityId?: string;
    currentEntityKind?: EntityKind;
    onMemberCreated?: (noteId: string, relationshipCode: string) => void;
}

/**
 * Context-aware menu for creating network members with relationships
 */
export function NetworkMemberCreationMenu({
    networkId,
    schema,
    currentEntityId,
    currentEntityKind = 'CHARACTER',
    onMemberCreated,
}: NetworkMemberCreationMenuProps) {
    const { createNote } = useJotaiNotes();
    const [isOpen, setIsOpen] = React.useState(false);

    // Get relationships where current entity can be the source
    const availableRelationships = React.useMemo(() => {
        return schema.relationships.filter(rel =>
            rel.sourceKind === currentEntityKind
        );
    }, [schema, currentEntityKind]);

    const handleCreateMember = async (relationship: NetworkRelationshipDef) => {
        // Create a note with entity prefix for the target kind
        const noteTitle = `[${relationship.targetKind}|New ${relationship.label}]`;
        const note = await createNote(undefined, noteTitle);

        onMemberCreated?.(note.id, relationship.code);
        setIsOpen(false);
    };

    if (availableRelationships.length === 0) {
        return null;
    }

    return (
        <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
            <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="h-6 w-6 p-0">
                    <UserPlus className="h-3.5 w-3.5" />
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56 bg-popover">
                <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">
                    Add Related Entity
                </div>
                <DropdownMenuSeparator />

                {availableRelationships.map(rel => {
                    const TargetIcon = ENTITY_ICONS[rel.targetKind] || Users;
                    return (
                        <DropdownMenuItem
                            key={rel.code}
                            onClick={() => handleCreateMember(rel)}
                            className="gap-2"
                        >
                            <TargetIcon
                                className="h-4 w-4"
                                style={{ color: rel.color || ENTITY_COLORS[rel.targetKind] }}
                            />
                            <span>Add {rel.label}</span>
                            <span
                                className="ml-auto text-[10px] px-1.5 py-0.5 rounded"
                                style={{
                                    backgroundColor: `${rel.color || ENTITY_COLORS[rel.targetKind]}20`,
                                    color: rel.color || ENTITY_COLORS[rel.targetKind],
                                }}
                            >
                                {rel.targetKind}
                            </span>
                        </DropdownMenuItem>
                    );
                })}
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

// ===== NETWORK BADGE =====

interface NetworkBadgeProps {
    kind: NetworkKind;
    name?: string;
    size?: 'sm' | 'md';
    className?: string;
}

/**
 * Badge component to display network type
 */
export function NetworkBadge({ kind, name, size = 'sm', className }: NetworkBadgeProps) {
    const Icon = NETWORK_ICONS[kind];
    const color = NETWORK_COLORS[kind];

    return (
        <span
            className={cn(
                "inline-flex items-center gap-1 rounded font-medium",
                size === 'sm' ? "text-[10px] px-1.5 py-0.5" : "text-xs px-2 py-1",
                className
            )}
            style={{
                backgroundColor: `${color}20`,
                color: color,
            }}
        >
            <Icon className={size === 'sm' ? "h-3 w-3" : "h-4 w-4"} />
            {name || kind}
        </span>
    );
}

export default NetworkFolderCreationMenu;
