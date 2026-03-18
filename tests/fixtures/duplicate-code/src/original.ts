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

export function validateInput(data: string): boolean {
    if (data === null || data === undefined) {
        return false;
    }
    const cleaned = data.trim();
    if (cleaned.length < 3) {
        return false;
    }
    return true;
}

export function formatOutput(items: string[]): string {
    const sorted = items.sort();
    const unique = [...new Set(sorted)];
    const formatted = unique.map(item => `- ${item}`);
    return formatted.join("\n");
}
