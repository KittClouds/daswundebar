/**
 * SQLite Quarantine Utilities
 * 
 * Provides safe stubs for deprecated SQLite methods that:
 * - Log warnings (max 3 per method)
 * - Never throw errors
 * - Return safe default values
 * - Track call counts for audit
 * 
 * @module lib/db/quarantine
 */

const callLog = new Map<string, number>();

/**
 * Create a quarantine stub for a deprecated SQLite method.
 * Returns a function that logs a warning and returns a safe default.
 */
export function quarantineStub<T>(
    name: string,
    defaultValue: T,
    options: { maxWarnings?: number } = {}
): (...args: unknown[]) => Promise<T> {
    const maxWarnings = options.maxWarnings ?? 3;

    return async (...args: unknown[]): Promise<T> => {
        const count = (callLog.get(name) ?? 0) + 1;
        callLog.set(name, count);

        if (count <= maxWarnings) {
            console.warn(
                `[QUARANTINE] ${name}() called (${count}x) - SQLite deprecated, using stub`,
                count === maxWarnings ? '(further warnings suppressed)' : ''
            );
        }

        return defaultValue;
    };
}

/**
 * Create a synchronous quarantine stub.
 */
export function quarantineStubSync<T>(
    name: string,
    defaultValue: T,
    options: { maxWarnings?: number } = {}
): (...args: unknown[]) => T {
    const maxWarnings = options.maxWarnings ?? 3;

    return (...args: unknown[]): T => {
        const count = (callLog.get(name) ?? 0) + 1;
        callLog.set(name, count);

        if (count <= maxWarnings) {
            console.warn(
                `[QUARANTINE] ${name}() called (${count}x) - SQLite deprecated, using stub`,
                count === maxWarnings ? '(further warnings suppressed)' : ''
            );
        }

        return defaultValue;
    };
}

/**
 * Get a report of all quarantined method calls.
 * Use this to audit what's still hitting deprecated code paths.
 */
export function getQuarantineReport(): Record<string, number> {
    return Object.fromEntries(callLog);
}

/**
 * Clear the quarantine report (for testing).
 */
export function clearQuarantineReport(): void {
    callLog.clear();
}

/**
 * Check if any quarantined methods have been called.
 */
export function hasQuarantineActivity(): boolean {
    return callLog.size > 0;
}

/**
 * Log a summary of quarantine activity to console.
 */
export function logQuarantineSummary(): void {
    if (callLog.size === 0) {
        console.log('[QUARANTINE] ✅ No deprecated SQLite methods called');
        return;
    }

    console.group('[QUARANTINE] ⚠️ Deprecated SQLite methods still in use:');
    for (const [name, count] of callLog.entries()) {
        console.log(`  ${name}: ${count} call(s)`);
    }
    console.groupEnd();
}
