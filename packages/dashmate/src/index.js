import { execute } from '@oclif/core';

// eslint-disable-next-line import/prefer-default-export
export const COMMANDS = {
  setup: (await import('./commands/setup.js')).default,
  config: (await import('./commands/config/index.js')).default,
};

export async function run() {
  await execute({ dir: import.meta.url, development: Boolean(process.env.DEBUG) });
}
