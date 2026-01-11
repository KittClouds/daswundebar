/**
 * NER Suggestion Popover
 * 
 * A lightweight tooltip/popover that appears when clicking a NER suggestion
 * in the text editor. Provides Accept/Reject actions and shows entity info.
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useNER } from '@/contexts/NERContext';
import type { NerSuggestion } from '@/lib/tauri/ner-bridge';
import { EntityKind, ENTITY_COLORS } from '@/lib/types/entityTypes';

interface NerSuggestionPopoverProps {
    /** The suggestion to display */
    suggestion: NerSuggestion | null;
    /** Position of the popover */
    position: { x: number; y: number } | null;
    /** Called when popover should close */
    onClose: () => void;
    /** Called when suggestion is accepted */
    onAccepted?: (entityId: string) => void;
    /** Called when suggestion is rejected */
    onRejected?: () => void;
}

export function NerSuggestionPopover({
    suggestion,
    position,
    onClose,
    onAccepted,
    onRejected,
}: NerSuggestionPopoverProps) {
    const { acceptSuggestion, rejectSuggestion } = useNER();
    const [isProcessing, setIsProcessing] = useState(false);
    const popoverRef = useRef<HTMLDivElement>(null);

    // Close on click outside
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
                onClose();
            }
        };

        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                onClose();
            }
        };

        document.addEventListener('mousedown', handleClickOutside);
        document.addEventListener('keydown', handleEscape);

        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
            document.removeEventListener('keydown', handleEscape);
        };
    }, [onClose]);

    const handleAccept = useCallback(async () => {
        if (!suggestion || isProcessing) return;

        setIsProcessing(true);
        const success = await acceptSuggestion(suggestion.id);
        setIsProcessing(false);

        if (success) {
            onAccepted?.(suggestion.id);
            onClose();
        }
    }, [suggestion, acceptSuggestion, onAccepted, onClose, isProcessing]);

    const handleReject = useCallback(async () => {
        if (!suggestion || isProcessing) return;

        setIsProcessing(true);
        const success = await rejectSuggestion(suggestion.id);
        setIsProcessing(false);

        if (success) {
            onRejected?.();
            onClose();
        }
    }, [suggestion, rejectSuggestion, onRejected, onClose, isProcessing]);

    if (!suggestion || !position) return null;

    const confidencePercent = Math.round(suggestion.confidence * 100);

    // Get entity color from type
    const entityKind = suggestion.entity_type.toUpperCase() as EntityKind;
    const varName = `--entity-${entityKind.toLowerCase().replace('_', '-')}`;

    return (
        <div
            ref={popoverRef}
            className="ner-suggestion-popover"
            style={{
                position: 'fixed',
                left: `${position.x}px`,
                top: `${position.y}px`,
                transform: 'translateX(-50%)',
                zIndex: 9999,
            }}
        >
            <div
                style={{
                    background: 'hsl(var(--popover))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: '8px',
                    padding: '12px',
                    minWidth: '220px',
                    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
                }}
            >
                {/* Header with entity type */}
                <div
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '8px',
                        marginBottom: '8px',
                    }}
                >
                    <span
                        style={{
                            backgroundColor: `hsl(var(${varName}) / 0.2)`,
                            color: `hsl(var(${varName}))`,
                            padding: '2px 8px',
                            borderRadius: '4px',
                            fontSize: '0.75rem',
                            fontWeight: 600,
                            textTransform: 'uppercase',
                        }}
                    >
                        {suggestion.entity_type}
                    </span>
                    <span
                        style={{
                            color: 'hsl(var(--muted-foreground))',
                            fontSize: '0.75rem',
                        }}
                    >
                        {confidencePercent}% confidence
                    </span>
                </div>

                {/* Entity text */}
                <div
                    style={{
                        color: 'hsl(var(--foreground))',
                        fontWeight: 500,
                        marginBottom: '12px',
                    }}
                >
                    "{suggestion.text}"
                </div>

                {/* Actions */}
                <div
                    style={{
                        display: 'flex',
                        gap: '8px',
                    }}
                >
                    <button
                        onClick={handleAccept}
                        disabled={isProcessing}
                        style={{
                            flex: 1,
                            padding: '6px 12px',
                            backgroundColor: 'hsl(var(--primary))',
                            color: 'hsl(var(--primary-foreground))',
                            border: 'none',
                            borderRadius: '4px',
                            fontSize: '0.875rem',
                            fontWeight: 500,
                            cursor: isProcessing ? 'not-allowed' : 'pointer',
                            opacity: isProcessing ? 0.5 : 1,
                        }}
                    >
                        {isProcessing ? '...' : '✓ Accept'}
                    </button>
                    <button
                        onClick={handleReject}
                        disabled={isProcessing}
                        style={{
                            flex: 1,
                            padding: '6px 12px',
                            backgroundColor: 'transparent',
                            color: 'hsl(var(--muted-foreground))',
                            border: '1px solid hsl(var(--border))',
                            borderRadius: '4px',
                            fontSize: '0.875rem',
                            fontWeight: 500,
                            cursor: isProcessing ? 'not-allowed' : 'pointer',
                            opacity: isProcessing ? 0.5 : 1,
                        }}
                    >
                        ✕ Reject
                    </button>
                </div>
            </div>
        </div>
    );
}

/**
 * Hook to manage NER suggestion popover state
 */
export function useNerSuggestionPopover() {
    const [suggestion, setSuggestion] = useState<NerSuggestion | null>(null);
    const [position, setPosition] = useState<{ x: number; y: number } | null>(null);

    const show = useCallback((suggestion: NerSuggestion, x: number, y: number) => {
        setSuggestion(suggestion);
        setPosition({ x, y });
    }, []);

    const hide = useCallback(() => {
        setSuggestion(null);
        setPosition(null);
    }, []);

    return {
        suggestion,
        position,
        show,
        hide,
        isOpen: suggestion !== null,
    };
}
