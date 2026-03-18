// Copy with renamed variables (Type-2 clone)
export function handleData(value: string): string {
    const cleaned = value.trim();
    if (cleaned.length === 0) {
        return "";
    }
    const segments = cleaned.split(",");
    const valid = segments.filter(s => s.length > 0);
    const transformed = valid.map(s => s.toUpperCase());
    return transformed.join(", ");
}

export function checkInput(info: string): boolean {
    if (info === null || info === undefined) {
        return false;
    }
    const sanitized = info.trim();
    if (sanitized.length < 3) {
        return false;
    }
    return true;
}

export function renderOutput(entries: string[]): string {
    const ordered = entries.sort();
    const deduped = [...new Set(ordered)];
    const lines = deduped.map(entry => `- ${entry}`);
    return lines.join("\n");
}
