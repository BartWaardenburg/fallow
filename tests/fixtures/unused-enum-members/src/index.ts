import { Color, HttpStatus, LogLevel } from './enums';

// Only Red is accessed
console.log(Color.Red);

// Ok and NotFound are accessed, InternalError and BadGateway are unused
console.log(HttpStatus.Ok);
console.log(HttpStatus.NotFound);

// LogLevel used via Object.values, so all members should be considered used
const levels = Object.values(LogLevel);
console.log(levels);
