export function asJsonString(value: unknown): string | undefined {
  if (value == null) return undefined;
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}

/**
 * Generate 32 bytes of cryptographically secure random entropy as a hex string.
 * Works in both Node.js and browser environments.
 *
 * @returns A 64-character hex string representing 32 bytes of entropy
 * @throws Error if no secure random source is available
 */
export function generateEntropy(): string {
  // Web Crypto API - works in both Node.js (v15+) and browsers
  if (typeof globalThis !== 'undefined' && globalThis.crypto && globalThis.crypto.getRandomValues) {
    const buffer = new Uint8Array(32);
    globalThis.crypto.getRandomValues(buffer);
    return Array.from(buffer).map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  // Fallback for older environments
  if (typeof window !== 'undefined' && window.crypto && window.crypto.getRandomValues) {
    const buffer = new Uint8Array(32);
    window.crypto.getRandomValues(buffer);
    return Array.from(buffer).map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  throw new Error('No secure random source available. This environment does not support crypto.getRandomValues.');
}
