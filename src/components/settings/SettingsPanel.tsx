import { useState } from 'react';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { SettingsManager } from '@/lib/settings/SettingsManager';
import { UnifiedLLMEngine, ModelRegistry } from '@/lib/llm';
import type { ModelId } from '@/lib/llm';
import { EmbeddingModelRegistry } from '@/lib/embeddings/models/ModelRegistry';
import { CheckCircle, XCircle, Loader2, Key, Bot, Sparkles, Fingerprint } from 'lucide-react';

interface SettingsPanelProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

export function SettingsPanel({ open, onOpenChange }: SettingsPanelProps) {
    const settings = SettingsManager.load();

    const [geminiKey, setGeminiKey] = useState(settings.llm.geminiApiKey || '');
    const [openrouterKey, setOpenrouterKey] = useState(settings.llm.openrouterApiKey || '');
    const [defaultModel, setDefaultModel] = useState<ModelId>(settings.llm.defaultModel);
    const [extractorModel, setExtractorModel] = useState<ModelId>(settings.llm.extractorModel);
    const [agentModel, setAgentModel] = useState<ModelId>(settings.llm.agentModel);
    const [embeddingModel, setEmbeddingModel] = useState(settings.embeddings.defaultModel);
    const [folderViewTheme, setFolderViewTheme] = useState(settings.appearance.folderViewTheme);

    const [testingGemini, setTestingGemini] = useState(false);
    const [testingOpenrouter, setTestingOpenrouter] = useState(false);
    const [geminiValid, setGeminiValid] = useState<boolean | null>(null);
    const [openrouterValid, setOpenrouterValid] = useState<boolean | null>(null);

    const geminiModels = ModelRegistry.getModelsByProvider('gemini');
    const openrouterModels = ModelRegistry.getModelsByProvider('openrouter');
    const allModels = ModelRegistry.getAllModels();

    const handleTestGemini = async () => {
        if (!geminiKey) return;
        setTestingGemini(true);
        setGeminiValid(null);
        const valid = await UnifiedLLMEngine.testApiKey('gemini', geminiKey);
        setGeminiValid(valid);
        setTestingGemini(false);
    };

    const handleTestOpenrouter = async () => {
        if (!openrouterKey) return;
        setTestingOpenrouter(true);
        setOpenrouterValid(null);
        const valid = await UnifiedLLMEngine.testApiKey('openrouter', openrouterKey);
        setOpenrouterValid(valid);
        setTestingOpenrouter(false);
    };

    const handleSave = () => {
        SettingsManager.updateLLMSettings({
            geminiApiKey: geminiKey || undefined,
            openrouterApiKey: openrouterKey || undefined,
            defaultModel,
            extractorModel,
            agentModel,
        });
        SettingsManager.update({
            embeddings: {
                defaultModel: embeddingModel,
            }
        });
        SettingsManager.update({
            appearance: {
                ...settings.appearance,
                folderViewTheme,
            }
        });
        onOpenChange(false);
    };

    const renderModelSelect = (
        id: string,
        label: string,
        value: ModelId,
        onChange: (v: ModelId) => void,
        description?: string
    ) => (
        <div className="space-y-2">
            <Label htmlFor={id}>{label}</Label>
            <Select value={value} onValueChange={(v) => onChange(v as ModelId)}>
                <SelectTrigger id={id}>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                        Gemini
                    </div>
                    {geminiModels.map((model) => (
                        <SelectItem key={model.id} value={model.id}>
                            <div className="flex items-center gap-2">
                                <span>{model.name}</span>
                                {model.costPer1kTokens === 0 && (
                                    <span className="text-xs bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded">
                                        FREE
                                    </span>
                                )}
                            </div>
                        </SelectItem>
                    ))}
                    <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider mt-2">
                        OpenRouter
                    </div>
                    {openrouterModels.map((model) => (
                        <SelectItem key={model.id} value={model.id}>
                            <div className="flex items-center gap-2">
                                <span>{model.name}</span>
                                {model.costPer1kTokens === 0 && (
                                    <span className="text-xs bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded">
                                        FREE
                                    </span>
                                )}
                            </div>
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            {description && (
                <p className="text-sm text-muted-foreground">{description}</p>
            )}
            {value && (
                <p className="text-xs text-muted-foreground italic">
                    {ModelRegistry.getModel(value)?.description}
                </p>
            )}
        </div>
    );

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
                <DialogHeader>
                    <DialogTitle className="flex items-center gap-2">
                        <Sparkles className="h-5 w-5" />
                        LLM Settings
                    </DialogTitle>
                    <DialogDescription>
                        Configure global application settings including LLM providers, appearance, and models.
                    </DialogDescription>
                </DialogHeader>

                <div className="space-y-6">
                    {/* API Keys Section */}
                    <Card>
                        <CardHeader className="pb-3">
                            <CardTitle className="text-base flex items-center gap-2">
                                <Key className="h-4 w-4" />
                                API Keys
                            </CardTitle>
                            <CardDescription>
                                Configure your API keys. Keys are stored locally in your browser.
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            {/* Gemini */}
                            <div className="space-y-2">
                                <Label htmlFor="gemini-key">Gemini API Key</Label>
                                <div className="flex gap-2">
                                    <Input
                                        id="gemini-key"
                                        type="password"
                                        placeholder="AIza..."
                                        value={geminiKey}
                                        onChange={(e) => {
                                            setGeminiKey(e.target.value);
                                            setGeminiValid(null);
                                        }}
                                        className="font-mono text-sm"
                                    />
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={handleTestGemini}
                                        disabled={!geminiKey || testingGemini}
                                        className="shrink-0"
                                    >
                                        {testingGemini ? (
                                            <Loader2 className="h-4 w-4 animate-spin" />
                                        ) : geminiValid === true ? (
                                            <CheckCircle className="h-4 w-4 text-green-500" />
                                        ) : geminiValid === false ? (
                                            <XCircle className="h-4 w-4 text-red-500" />
                                        ) : (
                                            'Test'
                                        )}
                                    </Button>
                                </div>
                                {geminiValid === false && (
                                    <Alert variant="destructive" className="py-2">
                                        <AlertDescription className="text-sm">
                                            Invalid API key. Check your key at{' '}
                                            <a
                                                href="https://aistudio.google.com/apikey"
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                className="underline"
                                            >
                                                Google AI Studio
                                            </a>
                                        </AlertDescription>
                                    </Alert>
                                )}
                                {geminiValid === true && (
                                    <p className="text-sm text-green-600">✓ API key is valid</p>
                                )}
                            </div>

                            {/* OpenRouter */}
                            <div className="space-y-2">
                                <Label htmlFor="openrouter-key">OpenRouter API Key</Label>
                                <div className="flex gap-2">
                                    <Input
                                        id="openrouter-key"
                                        type="password"
                                        placeholder="sk-or-v1-..."
                                        value={openrouterKey}
                                        onChange={(e) => {
                                            setOpenrouterKey(e.target.value);
                                            setOpenrouterValid(null);
                                        }}
                                        className="font-mono text-sm"
                                    />
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={handleTestOpenrouter}
                                        disabled={!openrouterKey || testingOpenrouter}
                                        className="shrink-0"
                                    >
                                        {testingOpenrouter ? (
                                            <Loader2 className="h-4 w-4 animate-spin" />
                                        ) : openrouterValid === true ? (
                                            <CheckCircle className="h-4 w-4 text-green-500" />
                                        ) : openrouterValid === false ? (
                                            <XCircle className="h-4 w-4 text-red-500" />
                                        ) : (
                                            'Test'
                                        )}
                                    </Button>
                                </div>
                                {openrouterValid === false && (
                                    <Alert variant="destructive" className="py-2">
                                        <AlertDescription className="text-sm">
                                            Invalid API key. Get one at{' '}
                                            <a
                                                href="https://openrouter.ai/keys"
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                className="underline"
                                            >
                                                OpenRouter
                                            </a>
                                        </AlertDescription>
                                    </Alert>
                                )}
                                {openrouterValid === true && (
                                    <p className="text-sm text-green-600">✓ API key is valid</p>
                                )}
                                <p className="text-xs text-muted-foreground">
                                    OpenRouter provides access to GPT-4o, Claude, Llama, and free models like Nemotron.
                                </p>
                            </div>
                        </CardContent>
                    </Card>

                    {/* Model Selection Section */}
                    <Card>
                        <CardHeader className="pb-3">
                            <CardTitle className="text-base flex items-center gap-2">
                                <Bot className="h-4 w-4" />
                                Model Selection
                            </CardTitle>
                            <CardDescription>
                                Choose which models to use for different tasks
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            {renderModelSelect(
                                'default-model',
                                'Default Model',
                                defaultModel,
                                setDefaultModel,
                                'Used for general chat and queries'
                            )}

                            {renderModelSelect(
                                'extractor-model',
                                'Entity Extractor Model',
                                extractorModel,
                                setExtractorModel,
                                'Used for extracting entities from notes'
                            )}

                            {renderModelSelect(
                                'agent-model',
                                'Agent Model',
                                agentModel,
                                setAgentModel,
                                'Used for AI agents and complex reasoning'
                            )}
                        </CardContent>
                    </Card>


                    {/* Appearance Section */}
                    <Card>
                        <CardHeader className="pb-3">
                            <CardTitle className="text-base flex items-center gap-2">
                                <Sparkles className="h-4 w-4" />
                                Appearance
                            </CardTitle>
                            <CardDescription>
                                Customize the look and feel of the application.
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="space-y-2">
                                <Label htmlFor="folder-theme">Folder View Theme</Label>
                                <Select value={folderViewTheme} onValueChange={(v: any) => setFolderViewTheme(v)}>
                                    <SelectTrigger id="folder-theme">
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="default">Default</SelectItem>
                                        <SelectItem value="structured">Structured (VS Code)</SelectItem>
                                        <SelectItem value="minimal">Minimal</SelectItem>
                                    </SelectContent>
                                </Select>
                                <p className="text-sm text-muted-foreground">
                                    Choose the visual style for the sidebar file tree.
                                </p>
                            </div>
                        </CardContent>
                    </Card>

                    {/* Embedding Models Section */}
                    <Card>
                        <CardHeader className="pb-3">
                            <CardTitle className="text-base flex items-center gap-2">
                                <Fingerprint className="h-4 w-4" />
                                🧬 Embedding Models
                            </CardTitle>
                            <CardDescription>
                                Choose how to generate embeddings. Local models run in your browser (private, free).
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="space-y-2">
                                <Label htmlFor="embedding-model">Embedding Model</Label>
                                <Select value={embeddingModel} onValueChange={setEmbeddingModel}>
                                    <SelectTrigger id="embedding-model">
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                                            Local (In-Browser)
                                        </div>
                                        {EmbeddingModelRegistry.getLocalModels().map((model) => (
                                            <SelectItem key={model.id} value={model.id}>
                                                <div className="flex items-center gap-2">
                                                    <span>{model.name}</span>
                                                    <span className="text-xs bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded">
                                                        FREE
                                                    </span>
                                                </div>
                                            </SelectItem>
                                        ))}

                                        <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider mt-2">
                                            Cloud (API)
                                        </div>
                                        {EmbeddingModelRegistry.getCloudModels().map((model) => (
                                            <SelectItem key={model.id} value={model.id}>
                                                <div className="flex items-center gap-2">
                                                    <span>{model.name}</span>
                                                </div>
                                            </SelectItem>
                                        ))}
                                    </SelectContent>
                                </Select>

                                {embeddingModel && (
                                    <div className="text-sm text-muted-foreground space-y-1 mt-2">
                                        <p>{EmbeddingModelRegistry.getModel(embeddingModel)?.description}</p>
                                        <div className="flex gap-4 text-xs font-mono">
                                            <span>Dimensions: {EmbeddingModelRegistry.getModel(embeddingModel)?.dimensions}</span>
                                            <span>Cost: {EmbeddingModelRegistry.getModel(embeddingModel)?.costPer1kTokens === 0
                                                ? 'FREE'
                                                : `$${EmbeddingModelRegistry.getModel(embeddingModel)?.costPer1kTokens}/1k tokens`}</span>
                                        </div>
                                    </div>
                                )}
                            </div>
                        </CardContent>
                    </Card>


                    {/* Actions */}
                    <div className="flex justify-between items-center">
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => {
                                SettingsManager.reset();
                                SettingsManager.clearCache();
                                const fresh = SettingsManager.load();
                                setGeminiKey(fresh.llm.geminiApiKey || '');
                                setOpenrouterKey(fresh.llm.openrouterApiKey || '');
                                setDefaultModel(fresh.llm.defaultModel);
                                setExtractorModel(fresh.llm.extractorModel);
                                setAgentModel(fresh.llm.agentModel);
                                setEmbeddingModel(fresh.embeddings.defaultModel);
                            }}
                            className="text-muted-foreground"
                        >
                            Reset to Defaults
                        </Button>
                        <div className="flex gap-2">
                            <Button variant="outline" onClick={() => onOpenChange(false)}>
                                Cancel
                            </Button>
                            <Button onClick={handleSave}>Save Settings</Button>
                        </div>
                    </div>
                </div>
            </DialogContent>
        </Dialog>
    );
}
