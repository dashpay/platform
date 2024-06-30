import { execute } from '@oclif/core';

// eslint-disable-next-line import/prefer-default-export
export const COMMANDS = {
  reset: (await import('./commands/reset.js')).default,
  restart: (await import('./commands/restart.js')).default,
  setup: (await import('./commands/setup.js')).default,
  start: (await import('./commands/start.js')).default,
  stop: (await import('./commands/stop.js')).default,
  update: (await import('./commands/update.js')).default,

  // Config
  config: (await import('./commands/config/index.js')).default,
  'config create': (await import('./commands/config/create.js')).default,
  'config default': (await import('./commands/config/default.js')).default,
  'config envs': (await import('./commands/config/envs.js')).default,
  'config get': (await import('./commands/config/get.js')).default,
  'config list': (await import('./commands/config/list.js')).default,
  'config remove': (await import('./commands/config/remove.js')).default,
  'config render': (await import('./commands/config/render.js')).default,
  'config set': (await import('./commands/config/set.js')).default,

  // Core
  'core cli': (await import('./commands/core/cli.js')).default,
  'core reindex': (await import('./commands/core/reindex.js')).default,

  // Docker
  'docker build': (await import('./commands/docker/build.js')).default,

  // Group
  'group core reindex': (await import('./commands/group/core/reindex.js')).default,
  'group default': (await import('./commands/group/default.js')).default,
  'group list': (await import('./commands/group/list.js')).default,
  'group reset': (await import('./commands/group/reset.js')).default,
  'group restart': (await import('./commands/group/restart.js')).default,
  'group start': (await import('./commands/group/start.js')).default,
  'group status': (await import('./commands/group/status.js')).default,
  'group stop': (await import('./commands/group/stop.js')).default,

  // SSL
  'ssl obtain': (await import('./commands/ssl/obtain.js')).default,

  // Status
  status: (await import('./commands/status/index.js')).default,
  'status core': (await import('./commands/status/core.js')).default,
  'status host': (await import('./commands/status/host.js')).default,
  'status masternode': (await import('./commands/status/masternode.js')).default,
  'status platform': (await import('./commands/status/platform.js')).default,
  'status services': (await import('./commands/status/services.js')).default,

  // Wallet
  'wallet mint': (await import('./commands/wallet/mint.js')).default,
};
export async function run() {
  await execute({ dir: import.meta.url, development: Boolean(process.env.DEBUG) });
}
