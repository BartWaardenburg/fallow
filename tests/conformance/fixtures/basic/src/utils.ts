export const usedFunction = (): { value: number } => ({ value: 42 });

export const unusedFunction = (): string => 'not used anywhere';

export function anotherUnused(): void {
    // This function is exported but never imported
}
