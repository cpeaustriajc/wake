type JsonRecord = Readonly<Record<string, unknown>>;

const isRecord = (value: unknown): value is JsonRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const textOf = (value: unknown, fallback: string): string => {
  if (typeof value === "string") {
    return value;
  }
  return fallback;
};

export { isRecord, textOf };
export type { JsonRecord };
