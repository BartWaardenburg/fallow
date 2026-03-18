// Partial overlap: only processData is duplicated
export function processData(input: string): string {
    const trimmed = input.trim();
    if (trimmed.length === 0) {
        return "";
    }
    const parts = trimmed.split(",");
    const filtered = parts.filter(p => p.length > 0);
    const mapped = filtered.map(p => p.toUpperCase());
    return mapped.join(", ");
}

export function uniqueHelper(value: number): string {
    const doubled = value * 2;
    const formatted = `Result: ${doubled}`;
    return formatted.toLowerCase();
}
