import assert from 'node:assert/strict';
import {
  mkdtemp, readFile, writeFile, mkdir, rm,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';

// The remaining facade imports are mocked; this test covers this one wrapper only.
test('identity module forwards cursor and independent versions to the actual binding export', async () => {
  const script = await readFile(new URL('../scripts/build-modules.sh', import.meta.url), 'utf8');
  const source = script.match(/cat > dist\/identity\.js << 'EOF'\n([\s\S]*?)\nEOF/)[1];
  const names = source.match(/import init, \{([\s\S]*?)\} from/)[1]
    .split(',').map((name) => name.trim().split(/\s+as\s+/)[0]).filter(Boolean);
  const mock = `
export const calls = [];
export let inits = 0;
export default async function init() { inits += 1; }
export function verifyFullIdentityByNonUniquePublicKeyHash(...args) {
  calls.push(args);
  if (args[0][0] === 255) throw new Error('binding rejected proof');
  return args;
}
` + names.filter((name) => ![
    'verify_full_identity_by_non_unique_public_key_hash',
    'verifyFullIdentityByNonUniquePublicKeyHash',
  ].includes(name))
    .map((name) => `export function ${name}() { throw new Error('unrelated wrapper'); }`)
    .join('\n');
  const directory = await mkdtemp(join(tmpdir(), 'nonunique-wrapper-'));
  try {
    await mkdir(join(directory, 'pkg'));
    await mkdir(join(directory, 'dist'));
    await writeFile(join(directory, 'package.json'), '{"type":"module"}');
    await writeFile(join(directory, 'pkg/wasm_drive_verify.js'), mock);
    await writeFile(join(directory, 'dist/identity.js'), source);
    const facade = await import(pathToFileURL(join(directory, 'dist/identity.js')));
    const binding = await import(pathToFileURL(join(directory, 'pkg/wasm_drive_verify.js')));
    const proof = new Uint8Array([1]);
    const hash = new Uint8Array(20);
    const after = new Uint8Array(32);
    assert.deepEqual(
      await facade.verifyFullIdentityByNonUniquePublicKeyHash(proof, hash, after, 13, 14),
      [proof, hash, after, 13, 14],
    );
    assert.deepEqual(
      await facade.verifyFullIdentityByNonUniquePublicKeyHash(proof, hash, undefined, 12, 14),
      [proof, hash, undefined, 12, 14],
    );
    await assert.rejects(
      facade.verifyFullIdentityByNonUniquePublicKeyHash(new Uint8Array([255]), hash, after, 13, 14),
      /binding rejected proof/,
    );
    assert.equal(binding.inits, 1);
    assert.equal(binding.calls.length, 3);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
