import { Status, Direction, Color, Priority } from './types';

// Object.values — all Status members should be used
const allStatuses = Object.values(Status);

// Object.keys — all Direction members should be used
const dirKeys = Object.keys(Direction);

// for...in — all Color members should be used
for (const key in Color) {
    console.log(key);
}

// Computed access with string literal — only Priority.High accessed
const high = Priority["High"];

console.log(allStatuses, dirKeys, high);
