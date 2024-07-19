import { execute } from '@oclif/core';

import reset from './commands/reset.js';
import restart from './commands/restart.js';
import setup from './commands/setup.js';
import start from './commands/start.js';
import stop from './commands/stop.js';
import update from './commands/update.js';

import configIndex from './commands/config/index.js';
import configCreate from './commands/config/create.js';
import configDefault from './commands/config/default.js';
import configEnvs from './commands/config/envs.js';
import configGet from './commands/config/get.js';
import configList from './commands/config/list.js';
import configRemove from './commands/config/remove.js';
import configRender from './commands/config/render.js';
import configSet from './commands/config/set.js';

import coreCli from './commands/core/cli.js';
import coreReindex from './commands/core/reindex.js';

import dockerBuild from './commands/docker/build.js';

import groupCoreReindex from './commands/group/core/reindex.js';
import groupDefault from './commands/group/default.js';
import groupList from './commands/group/list.js';
import groupReset from './commands/group/reset.js';
import groupRestart from './commands/group/restart.js';
import groupStart from './commands/group/start.js';
import groupStatus from './commands/group/status.js';
import groupStop from './commands/group/stop.js';

import sslObtain from './commands/ssl/obtain.js';

import statusCore from './commands/status/core.js';
import statusHost from './commands/status/host.js';
import statusIndex from './commands/status/index.js';
import statusMasternode from './commands/status/masternode.js';
import statusPlatform from './commands/status/platform.js';
import statusServices from './commands/status/services.js';

import walletMint from './commands/wallet/mint.js';

export const COMMANDS = {
  reset,
  restart,
  setup,
  start,
  stop,
  update,

  config: configIndex,
  'config:create': configCreate,
  'config:default': configDefault,
  'config:envs': configEnvs,
  'config:get': configGet,
  'config:list': configList,
  'config:remove': configRemove,
  'config:render': configRender,
  'config:set': configSet,

  'core:cli': coreCli,
  'core:reindex': coreReindex,

  'docker:build': dockerBuild,

  'group:core:reindex': groupCoreReindex,
  'group:default': groupDefault,
  'group:list': groupList,
  'group:reset': groupReset,
  'group:restart': groupRestart,
  'group:start': groupStart,
  'group:status': groupStatus,
  'group:stop': groupStop,

  'ssl:obtain': sslObtain,

  'status:core': statusCore,
  'status:host': statusHost,
  status: statusIndex,
  'status:masternode': statusMasternode,
  'status:platform': statusPlatform,
  'status:services': statusServices,

  'wallet:mint': walletMint,
};

export async function run() {
  await execute({ dir: import.meta.url, development: Boolean(process.env.DEBUG) });
}
