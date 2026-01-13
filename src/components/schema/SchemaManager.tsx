import { useState } from 'react';
import { Database, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useSchema } from '@/contexts/SchemaContext';
import { EntityShape, ENTITY_SHAPES } from '@/types/schema';
import { ATTRIBUTE_TYPES, AttributeType } from '@/types/attributes';
import { AttributeTemplate, EntityBlueprint } from '@/types/blueprints';
import { generateId } from '@/lib/utils/ids';
import { toast } from 'sonner';

interface SchemaManagerProps {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function SchemaManager({ open: controlledOpen, onOpenChange: setControlledOpen }: SchemaManagerProps = {}) {
  const [internalOpen, setInternalOpen] = useState(false);
  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;
  const setOpen = isControlled ? setControlledOpen! : setInternalOpen;

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      {!isControlled && (
        <DialogTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            title="Schema Manager"
          >
            <Database className="h-4 w-4" />
          </Button>
        </DialogTrigger>
      )}
      <DialogContent className="max-w-4xl h-[80vh] flex flex-col p-0">
        <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
          <DialogTitle className="flex items-center gap-2">
            <Database className="h-5 w-5" />
            Schema Manager
          </DialogTitle>
        </DialogHeader>
        <Tabs defaultValue="entity-types" className="flex-1 flex flex-col min-h-0">
          <TabsList className="mx-6 mt-4 grid w-auto grid-cols-4 bg-muted/50">
            <TabsTrigger value="entity-types">Entity Types</TabsTrigger>
            <TabsTrigger value="relationship-types">Relationship Types</TabsTrigger>
            <TabsTrigger value="blueprints">Blueprints</TabsTrigger>
            <TabsTrigger value="how-to-use">How to Use</TabsTrigger>
          </TabsList>

          <ScrollArea className="flex-1 px-6 py-4">
            <TabsContent value="entity-types" className="mt-0">
              <EntityTypesTab />
            </TabsContent>
            <TabsContent value="relationship-types" className="mt-0">
              <RelationshipTypesTab />
            </TabsContent>
            <TabsContent value="blueprints" className="mt-0">
              <BlueprintsTab />
            </TabsContent>
            <TabsContent value="how-to-use" className="mt-0">
              <HowToUseTab />
            </TabsContent>
          </ScrollArea>
        </Tabs>
        <div className="flex justify-end px-6 py-4 border-t border-border">
          <Button variant="secondary" onClick={() => setOpen(false)}>
            Close
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function EntityTypesTab() {
  const { entityTypes, registerEntityType, deleteEntityType } = useSchema();
  const [name, setName] = useState('');
  const [labelProp, setLabelProp] = useState('title');
  const [shape, setShape] = useState<EntityShape>('rectangle');
  const [color, setColor] = useState('#7C5BF1');

  const handleCreate = () => {
    if (!name.trim()) {
      toast.error('Entity type name is required');
      return;
    }
    const upperName = name.toUpperCase().replace(/\s+/g, '_');
    try {
      registerEntityType(upperName, labelProp, { shape, color });
      toast.success(`Created entity type: ${upperName}`);
      setName('');
      setLabelProp('title');
      setShape('rectangle');
      setColor('#7C5BF1');
    } catch (error: any) {
      toast.error(error.message);
    }
  };

  const handleDelete = (kind: string) => {
    try {
      deleteEntityType(kind);
      toast.success(`Deleted entity type: ${kind}`);
    } catch (error: any) {
      toast.error(error.message);
    }
  };

  return (
    <div className="grid grid-cols-2 gap-6">
      <div>
        <h3 className="text-lg font-semibold mb-2">Entity Types</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Define types of entities that can be used in your notes.
        </p>
        <div className="border border-border rounded-lg overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Type</TableHead>
                <TableHead>Visual</TableHead>
                <TableHead>Label Property</TableHead>
                <TableHead className="w-12"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {entityTypes.map((entity) => (
                <TableRow key={entity.kind}>
                  <TableCell className="font-medium">{entity.kind}</TableCell>
                  <TableCell>
                    <div
                      className="w-6 h-6 rounded"
                      style={{ backgroundColor: entity.defaultStyle?.color || '#7C5BF1' }}
                    />
                  </TableCell>
                  <TableCell>{entity.labelProp}</TableCell>
                  <TableCell>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-muted-foreground hover:text-destructive"
                      onClick={() => handleDelete(entity.kind)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </div>

      <div>
        <h3 className="text-lg font-semibold mb-4">New Entity Type</h3>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Entity Type Name</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value.toUpperCase())}
              placeholder="CHARACTER"
            />
            <p className="text-xs text-muted-foreground">
              Type must be uppercase (e.g., CHARACTER, LOCATION)
            </p>
          </div>

          <div className="space-y-2">
            <Label>Label Property</Label>
            <Input
              value={labelProp}
              onChange={(e) => setLabelProp(e.target.value)}
              placeholder="title"
            />
            <p className="text-xs text-muted-foreground">
              Property used to display entity name (usually "title")
            </p>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Shape</Label>
              <Select value={shape} onValueChange={(v) => setShape(v as EntityShape)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ENTITY_SHAPES.map((s) => (
                    <SelectItem key={s} value={s}>
                      {s.charAt(0).toUpperCase() + s.slice(1)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>Color</Label>
              <div className="flex gap-2">
                <div
                  className="w-10 h-10 rounded border border-border"
                  style={{ backgroundColor: color }}
                />
                <Input
                  value={color}
                  onChange={(e) => setColor(e.target.value)}
                  placeholder="#7C5BF1"
                />
              </div>
            </div>
          </div>

          <Button onClick={handleCreate} className="w-full">
            Create Entity Type
          </Button>
        </div>
      </div>
    </div>
  );
}

function RelationshipTypesTab() {
  const { entityTypes, relationshipTypes, registerRelationshipType, deleteRelationshipType } = useSchema();
  const [label, setLabel] = useState('');
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [directed, setDirected] = useState(true);
  const [color, setColor] = useState('#7C5BF1');

  const handleCreate = () => {
    if (!label.trim()) {
      toast.error('Relationship type label is required');
      return;
    }
    if (!from || !to) {
      toast.error('From and To entity types are required');
      return;
    }
    const upperLabel = label.toUpperCase().replace(/\s+/g, '_');
    try {
      registerRelationshipType(upperLabel, from, to, directed, { color });
      toast.success(`Created relationship type: ${upperLabel}`);
      setLabel('');
      setFrom('');
      setTo('');
      setDirected(true);
      setColor('#7C5BF1');
    } catch (error: any) {
      toast.error(error.message);
    }
  };

  const formatFromTo = (value: string | string[]) => {
    if (Array.isArray(value)) return value.join(', ');
    return value;
  };

  return (
    <div className="grid grid-cols-2 gap-6">
      <div>
        <h3 className="text-lg font-semibold mb-2">Relationship Types</h3>
        <p className="text-sm text-muted-foreground mb-4">
          Define types of relationships that can connect entities in your notes.
        </p>
        <div className="border border-border rounded-lg overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Type</TableHead>
                <TableHead>From</TableHead>
                <TableHead>To</TableHead>
                <TableHead className="w-12"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {relationshipTypes.map((rel) => (
                <TableRow key={rel.label}>
                  <TableCell className="font-medium">{rel.label}</TableCell>
                  <TableCell>{formatFromTo(rel.from)}</TableCell>
                  <TableCell>{formatFromTo(rel.to)}</TableCell>
                  <TableCell>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-muted-foreground hover:text-destructive"
                      onClick={() => deleteRelationshipType(rel.label)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </div>

      <div>
        <h3 className="text-lg font-semibold mb-4">New Relationship Type</h3>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Relationship Type</Label>
            <Input
              value={label}
              onChange={(e) => setLabel(e.target.value.toUpperCase().replace(/\s+/g, '_'))}
              placeholder="FRIEND_OF"
            />
            <p className="text-xs text-muted-foreground">
              Type must be uppercase with underscores (e.g., FRIEND_OF)
            </p>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>From Entity Type</Label>
              <Select value={from} onValueChange={setFrom}>
                <SelectTrigger>
                  <SelectValue placeholder="Select type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="*">Any (*)</SelectItem>
                  {entityTypes.map((e) => (
                    <SelectItem key={e.kind} value={e.kind}>
                      {e.kind}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>To Entity Type</Label>
              <Select value={to} onValueChange={setTo}>
                <SelectTrigger>
                  <SelectValue placeholder="Select type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="*">Any (*)</SelectItem>
                  {entityTypes.map((e) => (
                    <SelectItem key={e.kind} value={e.kind}>
                      {e.kind}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="flex items-start gap-4">
            <div className="flex items-center space-x-2 border border-border rounded-lg p-4 flex-1">
              <Checkbox
                id="directed"
                checked={directed}
                onCheckedChange={(checked) => setDirected(checked === true)}
              />
              <div className="grid gap-1.5 leading-none">
                <label htmlFor="directed" className="text-sm font-medium">
                  Directed Relationship
                </label>
                <p className="text-xs text-muted-foreground">
                  If directed, relationship has a specific direction (from → to)
                </p>
              </div>
            </div>

            <div className="space-y-2">
              <Label>Color</Label>
              <div className="flex gap-2">
                <div
                  className="w-10 h-10 rounded border border-border"
                  style={{ backgroundColor: color }}
                />
                <Input
                  value={color}
                  onChange={(e) => setColor(e.target.value)}
                  placeholder="#7C5BF1"
                  className="w-24"
                />
              </div>
            </div>
          </div>

          <Button onClick={handleCreate} className="w-full">
            Create Relationship Type
          </Button>
        </div>
      </div>
    </div>
  );
}

function BlueprintsTab() {
  const { entityTypes, blueprints, createBlueprint, deleteBlueprint } = useSchema();
  const [name, setName] = useState('');
  const [entityKind, setEntityKind] = useState('');
  const [description, setDescription] = useState('');
  const [templates, setTemplates] = useState<AttributeTemplate[]>([]);
  const [showForm, setShowForm] = useState(false);

  const addAttribute = () => {
    setTemplates([
      ...templates,
      {
        id: generateId(),
        name: '',
        type: 'Text',
        required: false,
      },
    ]);
  };

  const updateAttribute = (id: string, updates: Partial<AttributeTemplate>) => {
    setTemplates(templates.map((t) => (t.id === id ? { ...t, ...updates } : t)));
  };

  const removeAttribute = (id: string) => {
    setTemplates(templates.filter((t) => t.id !== id));
  };

  const handleCreate = () => {
    if (!name.trim()) {
      toast.error('Blueprint name is required');
      return;
    }
    if (!entityKind) {
      toast.error('Entity type is required');
      return;
    }

    const blueprint: EntityBlueprint = {
      id: generateId(),
      entityKind,
      name: name.trim(),
      description: description.trim(),
      templates,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    createBlueprint(blueprint);
    toast.success(`Created blueprint: ${name}`);
    setName('');
    setEntityKind('');
    setDescription('');
    setTemplates([]);
    setShowForm(false);
  };

  const handleCancel = () => {
    setName('');
    setEntityKind('');
    setDescription('');
    setTemplates([]);
    setShowForm(false);
  };

  return (
    <div>
      <h3 className="text-lg font-semibold mb-2">Attribute Blueprints</h3>
      <p className="text-sm text-muted-foreground mb-6">
        Create reusable attribute templates for entity types. When you create a new entity, you can apply a blueprint to automatically add predefined attributes.
      </p>

      {blueprints.length > 0 && (
        <div className="border border-border rounded-lg overflow-hidden mb-6">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Entity Type</TableHead>
                <TableHead>Attributes</TableHead>
                <TableHead className="w-12"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {blueprints.map((bp) => (
                <TableRow key={bp.id}>
                  <TableCell className="font-medium">{bp.name}</TableCell>
                  <TableCell>{bp.entityKind}</TableCell>
                  <TableCell>{bp.templates.length}</TableCell>
                  <TableCell>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-muted-foreground hover:text-destructive"
                      onClick={() => deleteBlueprint(bp.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {showForm ? (
        <div className="border border-border rounded-lg p-4 space-y-4 bg-muted/30">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Blueprint Name</Label>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Blueprint name"
              />
            </div>

            <div className="space-y-2">
              <Label>Entity Type</Label>
              <Select value={entityKind} onValueChange={setEntityKind}>
                <SelectTrigger>
                  <SelectValue placeholder="Entity type" />
                </SelectTrigger>
                <SelectContent>
                  {entityTypes.map((e) => (
                    <SelectItem key={e.kind} value={e.kind}>
                      {e.kind}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label>Description (optional)</Label>
            <Textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Blueprint description (optional)"
              rows={3}
            />
          </div>

          {templates.length > 0 && (
            <div className="space-y-2">
              <Label>Attributes</Label>
              <div className="space-y-2">
                {templates.map((template) => (
                  <div key={template.id} className="flex gap-2 items-center">
                    <Input
                      value={template.name}
                      onChange={(e) => updateAttribute(template.id, { name: e.target.value })}
                      placeholder="Attribute name"
                      className="flex-1"
                    />
                    <Select
                      value={template.type}
                      onValueChange={(v) => updateAttribute(template.id, { type: v as AttributeType })}
                    >
                      <SelectTrigger className="w-28">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {ATTRIBUTE_TYPES.map((t) => (
                          <SelectItem key={t} value={t}>
                            {t}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <div className="flex items-center gap-1">
                      <Checkbox
                        checked={template.required}
                        onCheckedChange={(checked) =>
                          updateAttribute(template.id, { required: checked === true })
                        }
                      />
                      <span className="text-xs">Required</span>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => removeAttribute(template.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          )}

          <Button variant="outline" size="sm" onClick={addAttribute} className="gap-1">
            <Plus className="h-3.5 w-3.5" />
            Add Attribute
          </Button>

          <div className="flex gap-2">
            <Button onClick={handleCreate}>Create Blueprint</Button>
            <Button variant="ghost" onClick={handleCancel}>
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <Button variant="outline" onClick={() => setShowForm(true)} className="gap-1">
          <Plus className="h-4 w-4" />
          New Blueprint
        </Button>
      )}
    </div>
  );
}

function HowToUseTab() {
  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-2">Using the Dynamic Schema System</h3>
        <p className="text-muted-foreground">
          Galaxy Notes now supports a dynamic schema system for creating and connecting rich entity types.
        </p>
      </div>

      <div>
        <h4 className="font-semibold mb-2">Creating Entities</h4>
        <p className="text-sm text-muted-foreground mb-3">
          Use the following syntax in your notes to create entity instances:
        </p>
        <div className="bg-muted/50 rounded-lg p-3 font-mono text-sm">
          [TYPE|Name]
        </div>
        <div className="mt-3">
          <p className="text-sm font-medium mb-2">Examples:</p>
          <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
            <li>[CHARACTER|Jon Snow]</li>
            <li>[LOCATION|Winterfell]</li>
            <li>[CONCEPT|Winter is Coming]</li>
            <li>[CHARACTER:ALLY|Samwell Tarly]</li>
          </ul>
        </div>
      </div>

      <div>
        <h4 className="font-semibold mb-2">Creating Relationships</h4>
        <p className="text-sm text-muted-foreground mb-3">
          Connect entities using triple syntax:
        </p>
        <div className="bg-muted/50 rounded-lg p-3 font-mono text-sm">
          [TYPE_A|Entity1] (RELATIONSHIP) [TYPE_B|Entity2]
        </div>
        <div className="mt-3">
          <p className="text-sm font-medium mb-2">Examples:</p>
          <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
            <li>[CHARACTER|Jon Snow] (ALLY_OF) [CHARACTER|Arya Stark]</li>
            <li>[CHARACTER|Daenerys] (RULES) [LOCATION|Meereen]</li>
            <li>[FACTION|Night's Watch] (GUARDS) [LOCATION|The Wall]</li>
          </ul>
        </div>
      </div>

      <div>
        <h4 className="font-semibold mb-2">Other Syntax</h4>
        <div className="space-y-4">
          <div>
            <p className="text-sm font-medium">Tags:</p>
            <div className="bg-muted/50 rounded-lg p-2 font-mono text-sm mt-1">
              #tagname
            </div>
          </div>
          <div>
            <p className="text-sm font-medium">Mentions:</p>
            <div className="bg-muted/50 rounded-lg p-2 font-mono text-sm mt-1">
              @username
            </div>
          </div>
          <div>
            <p className="text-sm font-medium">Wiki Links:</p>
            <div className="bg-muted/50 rounded-lg p-2 font-mono text-sm mt-1">
              [[Page Title]]
            </div>
          </div>
          <div>
            <p className="text-sm font-medium">Backlinks:</p>
            <div className="bg-muted/50 rounded-lg p-2 font-mono text-sm mt-1">
              {'<<Page Title>>'}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
